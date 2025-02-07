// Session은 클라이언트의 상태에 따라 패킷을 다르게 처리해야한다.
// 따라서 Session의 상태에 따라 패킷 처리 함수를 변경하게 한다.
//
// Session 상태
// - Init: 클라이언트가 서버에 처음 연결된 상태
// - Lobby: 클라이언트가 게임 로비화면에 있는 상태
// - Matching: 클라이언트가 게임 월드에 참여를 대기하고 있는 상태
// - Draft: 게임 월드에 참가하여 캐릭터를 선택하고 있는 상태
// - InGame: 클라이언트가 게임을 진행중인 상태
// ...
//
mod in_game;
mod init;
mod lobby;

use std::{
    io::ErrorKind,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering as MemOrdering},
        Arc,
    },
};

use mod_network::{
    components::ClientId,
    protocol::{PacketParser, RawPacket},
};
use mod_parallelism::collections::Queue;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

use crate::world::World;

/// 세션의 상태 목록
#[derive(Debug)]
pub enum SessionState {
    Init,
    Lobby,
    InGame(Arc<World>),
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Init
    }
}

/// 클라이언트 네트워크 통신 정보를 저장
#[derive(Debug)]
pub struct Session {
    /// 클라이언트 소켓 주소
    addr: SocketAddr,
    /// 세션의 클라이언트 식별자
    client_id: ClientId,

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
    pub fn new(
        addr: SocketAddr,
        client_id: ClientId,
        udp_sender: Arc<Queue<(SocketAddr, RawPacket)>>,
    ) -> Self {
        Self {
            addr,
            client_id,
            tcp_sender: Queue::new(),
            udp_sender,
            received_packets: Queue::new(),
            running: AtomicBool::new(true),
        }
    }

    /// 클라이언트 식별자를 반환합니다.
    pub fn client_id(&self) -> ClientId {
        self.client_id
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

impl std::fmt::Display for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Session({})", self.client_id.to_string())
    }
}

impl std::cmp::Eq for Session {}

impl std::cmp::PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.client_id.eq(&other.client_id)
    }
}

impl std::cmp::Ord for Session {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.client_id.cmp(&other.client_id)
    }
}

impl std::cmp::PartialOrd for Session {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.client_id.partial_cmp(&other.client_id)
    }
}

impl std::hash::Hash for Session {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.client_id.hash(state)
    }
}

/// 클라이언트 연결을 제어합니다.
pub async fn handle_connection(stream: TcpStream, session: Arc<Session>) {
    // 비동기 네트워크 처리 루프를 실행한다.
    let (tcp_reader, tcp_writer) = stream.into_split();
    tokio::spawn(tcp_read_loop(tcp_reader, session.clone()));
    tokio::spawn(tcp_write_loop(tcp_writer, session.clone()));

    // 패킷 처리 루프를 실행합니다.
    let mut state = SessionState::default();
    while session.is_running() {
        state = match state {
            SessionState::Init => init::handle_packets(&session),
            SessionState::Lobby => lobby::handle_packets(&session),
            SessionState::InGame(world) => in_game::handle_packets(&session, world),
        };

        // 다른 태스크들이 실행될 기회를 주기 위해 양보
        tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
    }
    log::info!("{} connection closed.", &session);

    // 세션이 게임 월드에 참가한 상태인 경우
    // 게임 월드에서 세션의 데이터를 제거합니다.
    if let SessionState::InGame(world) = state {
        world.exit(&session);
    }
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
