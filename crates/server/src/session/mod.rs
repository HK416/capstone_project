mod pool;
mod state;

use std::{
    cmp, fmt, hash,
    io::ErrorKind,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering as MemOrdering},
    },
};

use mod_network::{
    components::{LoginToken, UserInfo},
    protocol::{PacketParser, RawPacket},
};
use mod_parallelism::collections::Queue;
use state::SessionStateManager;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
};

pub use self::pool::*;

/// 클라이언트 네트워크 통신 정보를 저장
#[derive(Debug)]
pub struct Session {
    /// 클라이언트 소켓 주소
    addr: SocketAddr,
    /// 세션의 사용자 데이터
    info: UserInfo,

    /// TCP 패킷 데이터 전송 대기열
    tcp_sender: Queue<RawPacket>,
    /// UDP 패킷 데이터 전송 대기열
    udp_sender: Arc<Queue<(SocketAddr, RawPacket)>>,
    /// 수신된 패킷 데이터 대기열
    received_packets: Queue<RawPacket>,

    /// 세션의 실행 상태
    running: AtomicBool,
}

impl Session {
    /// 새로운 클라이언트 세션을 생성합니다.
    pub fn new(addr: SocketAddr, udp_sender: Arc<Queue<(SocketAddr, RawPacket)>>) -> Self {
        Self {
            addr,
            info: UserInfo::default(),
            tcp_sender: Queue::new(),
            udp_sender,
            received_packets: Queue::new(),
            running: AtomicBool::new(true),
        }
    }

    /// 세션의 사용자 정보를 반환합니다.
    pub fn user(&self) -> &UserInfo {
        &self.info
    }

    /// 클라이언트 세션이 동작중인지 여부를 반환합니다.
    pub fn is_running(&self) -> bool {
        self.running.load(MemOrdering::Relaxed)
    }

    /// 클라이언트 세션을 닫습니다.
    pub fn close(&self) {
        self.running.store(false, MemOrdering::Release);
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
    pub fn udp_write(&self, packet: RawPacket) {
        assert!(
            packet.as_bytes().len() < 1024,
            "the size of the UDP packet to be transmitted from {} is greather than or equal to 1KB.",
            &self
        );
        self.udp_sender.push((self.addr, packet));
    }

    /// 수신된 패킷 데이터를 세션에 추가합니다.
    ///
    /// 추가된 패킷 데이터는 바로 처리되지 않습니다.
    ///
    pub fn push_received_packet(&self, packet: RawPacket) {
        self.received_packets.push(packet);
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
        self.info.eq(&other.info)
    }
}

impl cmp::Ord for Session {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.info.cmp(&other.info)
    }
}

impl cmp::PartialOrd for Session {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.info.partial_cmp(&other.info)
    }
}

impl hash::Hash for Session {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.info.hash(state)
    }
}

/// 클라이언트 연결을 제어합니다.
pub async fn handle_connection(stream: TcpStream, session: Arc<Session>) {
    log::info!("{} start connection.", &session);

    // 비동기 네트워크 처리 루프를 실행한다.
    let (tcp_reader, tcp_writer) = stream.into_split();
    tokio::spawn(tcp_read_loop(tcp_reader, session.clone()));
    tokio::spawn(tcp_write_loop(tcp_writer, session.clone()));

    // 네트워크 패킷 처리 루프를 실행합니다.
    SessionStateManager::new(&session).run().await;

    log::info!("{} connection closed.", &session);
}

/// TCP 소켓의 데이터를 읽는 루프 함수입니다.
async fn tcp_read_loop(mut tcp_reader: OwnedReadHalf, session: Arc<Session>) {
    let mut buf = vec![0; 10240]; // 10KB
    let mut packet_parser = PacketParser::new();
    'tcp: while session.is_running() {
        // 버퍼 초기화
        buf.fill(0);

        // 소켓으로부터 데이터 읽기
        let result = tcp_reader.read(&mut buf).await;
        match result {
            Ok(0) => {
                log::trace!("{} connection closed.", &session);
                session.close();
                break 'tcp;
            }
            Ok(n) => {
                log::trace!("{} data received (SIZE:{}, BYTES:{:?})", &session, n, &buf);
                packet_parser.push(&buf[..n]);
            }
            Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                log::trace!("{} connection closed.", &session);
                session.close();
                break 'tcp;
            }
            Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
                log::trace!("{} connection closed.", &session);
                session.close();
                break 'tcp;
            }
            Err(e) => {
                log::error!("{} {}", &session, e);
                session.close();
                break 'tcp;
            }
        }

        // 패킷 구문 분석 및 대기열에 추가.
        while let Some(packet) = packet_parser.pop() {
            log::trace!("{} packet received (PACKET:{:?})", &session, &packet);
            session.received_packets.push(packet);
        }

        // 다른 태스크들이 실행될 기회를 주기 위해 양보
        tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
    }
}

/// TCP 소켓에 데이터를 쓰는 루프 함수입니다.
async fn tcp_write_loop(mut tcp_writer: OwnedWriteHalf, session: Arc<Session>) {
    'tcp: while session.is_running() {
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
                    log::trace!("{} packet sent (PACKET:{:?})", &session, &packet);
                }
                Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                    log::trace!("{} connection closed.", &session);
                    session.close();
                    break 'tcp;
                }
                Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
                    log::trace!("{} connection closed.", &session);
                    session.close();
                    break 'tcp;
                }
                Err(e) => {
                    log::error!("{} {}", &session, e);
                    session.close();
                    break 'tcp;
                }
            };
        } else {
            // 다른 태스크들이 실행될 기회를 주기 위해 양보
            tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
        }
    }
}
