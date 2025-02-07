use std::{
    env,
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering as MemOrdering},
        Arc, OnceLock,
    },
};

use mod_network::{addr::Addr, components::ClientId, protocol::RawPacket};
use mod_parallelism::collections::{Queue, SkipMap};
use server::{
    session::{handle_connection, Session},
    world::{update_game_world, World},
};
use tokio::net::{TcpListener, UdpSocket};

/// 현재 클라이언트의 수 입니다.
static NUM_CLIENTS: AtomicU64 = AtomicU64::new(0);
/// 사용되지 않는 클라이언트 식별자 목록입니다.
static RETIRE_IDS: OnceLock<Queue<u64>> = OnceLock::new();
/// 현재 서버에 접속중인 세션 집합입니다.
static SESSIONS: OnceLock<SkipMap<SocketAddr, Arc<Session>>> = OnceLock::new();

/// 사용되지 않는 클라이언트 식별자 목록을 가져옵니다.
fn get_retire_ids() -> &'static Queue<u64> {
    RETIRE_IDS.get_or_init(|| Queue::default())
}

/// 현재 서버에 접속중인 세션 집합을 가져옵니다.
fn get_seesions() -> &'static SkipMap<SocketAddr, Arc<Session>> {
    SESSIONS.get_or_init(|| SkipMap::default())
}

/// 메인 쓰레드에서 월드 업데이트, 새로운 쓰레드를 생성해서 연결 관리
pub async fn run_server(addr: &str) {
    // TCP 소켓을 바인드합니다.
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Failed to bind to address: {}", e);
            return;
        }
    };
    println!("Server listening on: {}", listener.local_addr().unwrap());

    // TODO: UDP 소켓을 바인드합니다.
    // NOTE: TCP 소켓과 다르게 connect가 필요 없다.
    let udp_sender = Arc::new(Queue::new());

    // TODO: 새로운 스레드에서 UDP 패킷 수신 루프를 실행합니다.
    //
    // TODO: 새로운 스레드에서 UDP 패킷 전송 루프를 실행합니다.
    //

    // 새로운 쓰레드에서 클라이언트 연결 관리
    tokio::spawn(wait_for_players(listener, udp_sender));

    // 게임 월드 업데이트
    // TODO: 나중에 여러 개의 게임 월드를 실행해야함.
    let world = World::get_instance();
    update_game_world(world).await;
}

/// UDP 통신으로 수신된 패킷을 각 세션에 전달하는 루프 함수입니다.
async fn udp_packet_receive_loop(socket: Arc<UdpSocket>) {
    let mut buf = [0; 1024];
    loop {
        buf.fill(0);

        // TODO
        // 1. tokio::UdpSocket의 recv_from함수로 패킷 데이터와 클라이언트 주소 값을 가져온다.
        // 2. 바이트 배열을 RawPacket으로 변환한다.
        //    - RawPacket으로 변환에 실패한 경우 생략
        //      (UDP로 보낸 패킷 데이터는 중요하지 않고, 1024byte 보다 작은 데이터이기 때문)
        //
        // 3. SESSIONS에 클라이언트 주소에 해당하는 세션이 존재할 경우
        //    - 해당 세션으로 RawPacket을 전송한다.

        // 다른 태스크들이 실행될 기회를 주기 위해 양보
        tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
    }
}

/// UDP 통신으로 패킷을 전송하는 루프 함수입니다.
async fn udp_packet_send_loop(
    socket: Arc<UdpSocket>,
    udp_sender: Arc<Queue<(SocketAddr, RawPacket)>>,
) {
    loop {
        // TODO
        // 1. `udp_sender`에서 값을 하나 가져온다.
        // 2. tokio::UdpSocket의 send_to함수로 패킷 데이터를 클라이언트로 보낸다.

        // 다른 태스크들이 실행될 기회를 주기 위해 양보
        tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
    }
}

async fn wait_for_players(listener: TcpListener, udp_sender: Arc<Queue<(SocketAddr, RawPacket)>>) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                // 클라이언트 식별자를 할당합니다.
                let value = get_retire_ids()
                    .pop()
                    .unwrap_or_else(|| NUM_CLIENTS.fetch_add(1, MemOrdering::AcqRel) + 1);
                let client_id = ClientId::new(value as u128);

                let udp_sender = udp_sender.clone();
                tokio::spawn(async move {
                    // 클라이언트 세션을 생성하고 등록합니다.
                    let session = Arc::new(Session::new(addr, client_id, udp_sender));
                    get_seesions().insert(addr, session.clone());

                    println!("Accepted connection from: {}", client_id.to_string());
                    handle_connection(stream, session).await;
                    println!("{} left.", client_id.to_string());

                    // 등록된 클라이언트 세션을 제거하고, 클라이언트 식별자를 반납합니다.
                    get_seesions().remove(&addr);
                    get_retire_ids().push(value);
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection; err = {:?}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut args = env::args();
    args.next();

    let addr = match args.next() {
        Some(args) => match Addr::from_str(&args) {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        },
        None => Addr::default(),
    };

    run_server(&addr.to_string()).await;
}
