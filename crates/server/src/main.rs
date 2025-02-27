use std::{
    env,
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicU32, Ordering as MemOrdering},
        Arc, OnceLock,
    }, time::{SystemTime, UNIX_EPOCH},
};

use mod_network::{addr::Addr, components::ClientId, protocol::RawPacket};
use mod_parallelism::collections::{Queue, SkipMap};
use server::{
    data::get_current_path, session::{handle_connection, Session}, world::{update_game_world, World}
};
use tokio::net::{TcpListener, UdpSocket};
use tracing::Level;
use tracing_appender::{non_blocking::WorkerGuard, rolling};

/// 현재 접속중인 클라이언트의 수 입니다.
static NUM_CLIENTS: AtomicU32 = AtomicU32::new(0);
/// 현재 서버에 접속중인 세션 집합입니다.
static SESSIONS: OnceLock<SkipMap<SocketAddr, Arc<Session>>> = OnceLock::new();

/// 현재 서버에 접속중인 세션 집합을 가져옵니다.
fn get_sessions() -> &'static SkipMap<SocketAddr, Arc<Session>> {
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

    // UDP 소켓을 바인드합니다.
    // NOTE: TCP 소켓과 다르게 connect가 필요 없다.
    let udp_socket = match UdpSocket::bind(addr).await {
        Ok(socket) => Arc::new(socket),
        Err(e) => {
            eprintln!("Failed to bind UDP socket: {}", e);
            return;
        }
    };
    let udp_sender = Arc::new(Queue::new());

    // 새로운 스레드에서 UDP 패킷 수신 루프를 실행합니다.
    let udp_recv_socket = udp_socket.clone();
    tokio::spawn(udp_packet_receive_loop(udp_recv_socket));

    // 새로운 스레드에서 UDP 패킷 전송 루프를 실행합니다.
    let udp_send_socket = udp_socket.clone();
    let udp_sender_clone = udp_sender.clone();
    tokio::spawn(udp_packet_send_loop(udp_send_socket, udp_sender_clone));

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

        // tokio::UdpSocket의 recv_from함수로 패킷 데이터와 클라이언트 주소 값을 가져온다.
        match socket.recv_from(&mut buf).await {
            Ok((size, addr)) => {
                let received_data = &buf[..size];

                // 바이트 배열을 RawPacket으로 변환한다.
                //    - RawPacket으로 변환에 실패한 경우 생략
                //      (UDP로 보낸 패킷 데이터는 중요하지 않고, 1024byte 보다 작은 데이터이기 때문)
                match RawPacket::try_from_bytes(received_data) {
                    Ok(packet) => {
                        let sessions = get_sessions();
                        // 3. SESSIONS에 클라이언트 주소에 해당하는 세션이 존재할 경우
                        //    - 해당 세션으로 RawPacket을 전송한다.
                        if let Some(session) = sessions.get(&addr) {
                            session.push_received_packet(packet);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to parse packet from {}: {}", addr, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to receive UDP packet: {}", e);
            }
        }

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
        // `udp_sender`에서 값을 하나 가져온다.
        if let Some((addr, packet)) = udp_sender.pop() {
            let packet_data = packet.as_bytes();

            // tokio::UdpSocket의 send_to함수로 패킷 데이터를 클라이언트로 보낸다.
            if let Err(e) = socket.send_to(&packet_data, addr).await {
                eprintln!("Failed to send UDP packet to {}: {}", addr, e);
            }
        }

        // 다른 태스크들이 실행될 기회를 주기 위해 양보
        tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
    }
}

async fn wait_for_players(listener: TcpListener, udp_sender: Arc<Queue<(SocketAddr, RawPacket)>>) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                // 클라이언트 식별자를 할당합니다.
                let client_id = generate_client_id();

                let udp_sender = udp_sender.clone();
                tokio::spawn(async move {
                    
                    // 클라이언트 세션을 생성하고 등록합니다.
                    let session = Arc::new(Session::new(addr, client_id, udp_sender));
                    get_sessions().insert(addr, session.clone());
                    NUM_CLIENTS.fetch_add(1, MemOrdering::AcqRel);

                    println!("Accepted connection from: {} (Concurrent Users:{})", &client_id, &NUM_CLIENTS.load(MemOrdering::Relaxed));
                    
                    handle_connection(stream, session).await;
                    
                    // 등록된 클라이언트 세션을 제거합니다.
                    get_sessions().remove(&addr);
                    NUM_CLIENTS.fetch_sub(1, MemOrdering::AcqRel);
                    println!("{} left. (Concurrent Users:{})", &client_id, &NUM_CLIENTS.load(MemOrdering::Relaxed));
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection; err = {:?}", e);
            }
        }
    }
}

fn main() {
    // 서버를 실행하기 전에 필요한 모든 데이터를 여기서 초기화합니다.
    //
    let _guard = init_log_system();

    let mut args = env::args();
    args.next();

    let mut addr = Addr::default();
    let mut num_threads = num_cpus::get();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--set-addr" => {
                if let Some(addr_str) = args.next() {
                    addr = match Addr::from_str(&addr_str) {
                        Ok(addr) => addr,
                        Err(e) => {
                            eprintln!("명령줄 인자 형식이 잘못되었습니다.\n  `--set-addr` - 잘못된 주소 형식입니다.\n{}", e);
                            return;
                        }
                    }
                }
            }
            "--num-threads" => {
                if let Some(threads_str) = args.next() {
                    num_threads = match threads_str.parse::<usize>() {
                        Ok(num_threads) => num_threads,
                        Err(e) => {
                            eprintln!("명령줄 인자 형식이 잘못되었습니다.\n  `--num_threads` - 스레드 수는 양의 정수여야 합니다.\n{}", e);
                            return;
                        }
                    }
                }
            }
            _ => {
                eprintln!("Invalid option: {}", arg);
                return;
            }
        }
    }

    println!("num_threads: {}", num_threads);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_threads)
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_server(&addr.to_string()));
}

/// 로그 시스템을 초기화 합니다.
/// 
/// # Note
/// 반환되는 `WorkerGuard`를 유지해야 로그가 정상적으로 저장됩니다.
/// 
fn init_log_system() -> WorkerGuard {
    // 현재 실행 파일의 디렉토리 경로에 로그 디렉토리 경로를 생성합니다.
    let mut dir = get_current_path().to_path_buf();
    dir.push("logs");

    // 매 시간 마다 새 파일을 생성하는 로그 시스템을 생성합니다.
    let file_appender = rolling::hourly(dir, "service_log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_max_level(Level::INFO)
        .init();

    guard
}

/// 클라이언트 식별자를 생성합니다.
fn generate_client_id() -> ClientId {
    // 난수를 생성하기 위한 카운터입니다.
    // 해당 함수를 호출할 때 마다 1씩 증가합니다.
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let part_0 = COUNTER.fetch_add(1, MemOrdering::AcqRel) & 0xFFFF;
    let part_1 = duration.subsec_micros() & 0xFFFF;

    ClientId::new((part_1 << 16) | part_0)
}
