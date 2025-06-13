mod pool;
mod state;

use std::{
    cmp, fmt, hash,
    io::ErrorKind,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering as MemOrdering},
    },
};

use mod_network::{
    components::NetworkState,
    protocol::{PacketParser, RawPacket},
};
use mod_parallelism::collections::Queue;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    time::Duration,
};

pub use self::{pool::*, state::*};

/// 수신된 패킷 데이터 대기열의 최대 용량입니다.
pub const MAX_QUEUE_CAPACITY: usize = 64;

/// 클라이언트 네트워크 통신 정보를 저장
#[derive(Debug)]
pub struct Session {
    /// 클라이언트 소켓 주소
    addr: SocketAddr,

    /// 세션 패킷 전송 시간 (단위: ms)
    ping: AtomicU32,

    /// TCP 패킷 데이터 전송 대기열
    tcp_sender: Queue<RawPacket>,
    /// UDP 패킷 데이터 전송 대기열
    #[allow(dead_code)]
    udp_sender: Arc<Queue<(SocketAddr, RawPacket)>>,

    /// 수신된 패킷 데이터 대기열
    received_packets: Queue<RawPacket>,
    /// 세션 상태 흐름 대기열입니다.
    flows: Queue<SessionStateFlow>,

    /// 수신된 패킷의 버림 여부
    cancel_token: AtomicBool,
    /// 세션의 실행 상태
    running: AtomicBool,
}

impl Session {
    /// 새로운 클라이언트 세션을 생성합니다.
    pub fn new(addr: SocketAddr, udp_sender: Arc<Queue<(SocketAddr, RawPacket)>>) -> Self {
        Self {
            addr,
            ping: AtomicU32::new(250),
            tcp_sender: Queue::new(),
            udp_sender,
            received_packets: Queue::new(),
            flows: Queue::new(),
            cancel_token: AtomicBool::new(false),
            running: AtomicBool::new(true),
        }
    }

    /// TCP 통신으로 패킷을 전송합니다.
    ///
    /// 이 함수는 패킷을 즉시 전송하지 않습니다.
    ///
    pub fn tcp_write(&self, packet: RawPacket) {
        self.tcp_sender.push(packet);
    }

    /// UDP 통신으로 패킷을 전송합니다.
    ///
    /// # Panics
    /// 주어진 `RawPacket`의 크기는 1KB 미만이어야합니다.
    /// 그렇지 않는 경우 [`panic!`]을 호출합니다.
    ///
    #[allow(dead_code)]
    pub fn udp_write(&self, packet: RawPacket) {
        assert!(
            packet.as_bytes().len() < 1024,
            "the size of the UDP packet to be transmitted from {} is greather than or equal to 1KB.",
            &self
        );
        self.udp_sender.push((self.addr, packet));
    }

    /// 수신된 패킷 데이터를 세션에 추가합니다.  
    /// 추가된 패킷 데이터는 바로 처리되지 않습니다.
    pub fn push_received_packet(&self, packet: RawPacket) {
        self.received_packets.push(packet);
    }

    /// 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        let ping = self.ping.load(MemOrdering::Acquire);
        match ping {
            0..=50 => NetworkState::Good,
            51..=100 => NetworkState::Fair,
            101..=200 => NetworkState::Poor,
            _ => NetworkState::Critical,
        }
    }

    /// 수신된 패킷의 처리가 취소됐는지 여부를 반환합니다.
    pub fn packet_canceled(&self) -> bool {
        self.cancel_token.load(MemOrdering::Acquire)
    }

    /// 클라이언트 세션이 동작중인지 여부를 반환합니다.
    pub fn is_running(&self) -> bool {
        self.running.load(MemOrdering::Relaxed)
    }

    /// 클라이언트 세션을 닫습니다.
    pub fn close(&self) {
        self.running.store(false, MemOrdering::Release);
    }
}

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Session(Addr:{})", &self.addr)
    }
}

impl cmp::Eq for Session {}

impl cmp::PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.addr.eq(&other.addr)
    }
}

impl cmp::Ord for Session {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.addr.cmp(&other.addr)
    }
}

impl cmp::PartialOrd for Session {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.addr.partial_cmp(&other.addr)
    }
}

impl hash::Hash for Session {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.addr.hash(state)
    }
}

/// 클라이언트 연결을 제어합니다.
pub async fn handle_connection(stream: TcpStream, mut session: Arc<Session>) {
    log::info!("{} start connection.", &session);

    // 비동기 네트워크 처리 루프를 실행한다.
    let (tcp_reader, tcp_writer) = stream.into_split();
    tokio::spawn(tcp_read_loop(tcp_reader, session.clone()));
    tokio::spawn(tcp_write_loop(tcp_writer, session.clone()));

    // 네트워크 패킷 처리 루프를 실행합니다.
    session = session_state_loop(session).await;

    log::info!("{} connection closed.", &session);
}

/// TCP 소켓의 데이터를 읽는 루프 함수입니다.
async fn tcp_read_loop(mut tcp_reader: OwnedReadHalf, session: Arc<Session>) {
    const TICK: Duration = Duration::from_millis(1);
    let mut interval = tokio::time::interval(TICK);

    let mut buf = vec![0; 10240]; // 10KB
    let mut packet_parser = PacketParser::new();
    'tcp: while session.is_running() {
        interval.tick().await;

        // 버퍼 초기화
        buf.fill(0);

        // 소켓으로부터 데이터 읽기
        let result = tcp_reader.read(&mut buf).await;
        match result {
            Ok(0) => {
                log::debug!("{} connection closed.", &session);
                session.close();
                break 'tcp;
            }
            Ok(n) => {
                log::trace!("{} data received (SIZE:{}, BYTES:{:?})", &session, n, &buf);
                packet_parser.push(&buf[..n]);
            }
            Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                log::debug!("{} connection closed.", &session);
                session.close();
                break 'tcp;
            }
            Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
                log::debug!("{} connection closed.", &session);
                session.close();
                break 'tcp;
            }
            Err(e) => {
                log::error!("{} {}", &session, e);
                session.close();
                break 'tcp;
            }
        }

        while let Some(packet) = packet_parser.pop() {
            log::debug!("{} packet received (PACKET:{:?})", &session, &packet);

            // 수신된 패킷 데이터가 가득찼는지 확인합니다.
            if session.received_packets.len() >= MAX_QUEUE_CAPACITY {
                // 큐를 비웁니다.
                log::warn!("the number of received packets exceeded the allowed capacity!");
                session.cancel_token.store(true, MemOrdering::Release);
                while let Some(_) = session.received_packets.pop() {
                    std::hint::spin_loop();
                }
                session.cancel_token.store(false, MemOrdering::Release);

                // 다른 작업이 실행될 기회를 주기 위해 반복문을 탈출합니다.
                log::info!("clearing received packet data.");
                packet_parser.clear();
                break;
            }

            session.received_packets.push(packet);
        }
    }
}

/// TCP 소켓에 데이터를 쓰는 루프 함수입니다.
async fn tcp_write_loop(mut tcp_writer: OwnedWriteHalf, session: Arc<Session>) {
    const TICK: Duration = Duration::from_millis(1);
    let mut interval = tokio::time::interval(TICK);

    'tcp: while session.is_running() {
        interval.tick().await;

        // 대기열에서 패킷을 가져온다.
        if let Some(packet) = session.tcp_sender.pop() {
            if !session.is_running() {
                return;
            }

            // 소켓에 데이터를 작성한다.
            let bytes = packet.as_bytes();
            let result = tcp_writer.write_all(&bytes).await;
            match result {
                Ok(_) => {
                    log::debug!("{} packet sent (PACKET:{:?})", &session, &packet);
                }
                Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                    log::debug!("{} connection closed.", &session);
                    session.close();
                    break 'tcp;
                }
                Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
                    log::debug!("{} connection closed.", &session);
                    session.close();
                    break 'tcp;
                }
                Err(e) => {
                    log::error!("{} {}", &session, e);
                    session.close();
                    break 'tcp;
                }
            };
        }
    }
}
