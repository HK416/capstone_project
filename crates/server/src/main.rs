mod account;
mod ai; // AI 모듈 추가
mod data;
mod entities;
mod formula;
mod matching;
mod session;
mod token;
mod world;

use std::{env, net::SocketAddr, str::FromStr, sync::Arc};

use data::{get_current_path, init_character_attributes, init_stage_attributes};
use mod_network::{addr::Addr, protocol::RawPacket};
use mod_parallelism::collections::Queue;
use session::{Session, SessionManager, handle_connection};
use tokio::net::{TcpListener, UdpSocket};
use tracing::level_filters::LevelFilter;
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::EnvFilter;
use world::GameWorldPool;

// AI 그리드 맵 사전 로딩 함수 가져오기
use ai::ai_player::preload_all_grid_maps;

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

    // 클라이언트 연결 관리
    wait_for_players(listener, udp_sender).await;
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
                        // 3. SESSIONS에 클라이언트 주소에 해당하는 세션이 존재할 경우
                        //    - 해당 세션으로 RawPacket을 전송한다.
                        if let Some(session) = SessionManager::get(&addr) {
                            session.add_received_packet(packet);
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
                let udp_sender = udp_sender.clone();
                tokio::spawn(async move {
                    // 클라이언트 세션을 생성하고 등록합니다.
                    let session = Arc::new(Session::new(addr, udp_sender));
                    SessionManager::regist(addr, session.clone());
                    println!(
                        "Accepted connection from: {} (Concurrent Users:{})",
                        &session,
                        &SessionManager::count()
                    );

                    handle_connection(stream, session).await;

                    // 등록된 클라이언트 세션을 제거합니다.
                    let session = SessionManager::unregist(&addr).unwrap();
                    println!(
                        "{} left. (Concurrent Users:{})",
                        &session,
                        &SessionManager::count()
                    );
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

    init_character_attributes();
    init_stage_attributes();

    // **AI 그리드 맵 사전 로딩**: 서버 시작 시 모든 스테이지의 그리드 맵을 미리 생성
    log::info!("[SERVER INIT] Starting AI grid map preloading...");
    preload_all_grid_maps();
    log::info!("[SERVER INIT] AI grid map preloading completed!");

    // 게임 월드 풀 객체를 초기화합니다.
    GameWorldPool::init();

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
                            eprintln!(
                                "명령줄 인자 형식이 잘못되었습니다.\n  `--set-addr` - 잘못된 주소 형식입니다.\n{}",
                                e
                            );
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
                            eprintln!(
                                "명령줄 인자 형식이 잘못되었습니다.\n  `--num_threads` - 스레드 수는 양의 정수여야 합니다.\n{}",
                                e
                            );
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
fn init_log_system() -> Option<WorkerGuard> {
    #[cfg(feature = "console")]
    {
        use console_subscriber;
        console_subscriber::init();
        return None;
    }

    // 현재 실행 파일의 디렉토리 경로에 로그 디렉토리 경로를 생성합니다.
    let mut dir = get_current_path().to_path_buf();
    dir.push("logs");

    // 매 시간 마다 새 파일을 생성하는 로그 시스템을 생성합니다.
    let formatted = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let file_name = format!("service_log-{}", formatted);
    let file_appender = rolling::hourly(dir, file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 로그에 남길 오류 수준을 설정합니다.
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();


    // 로그 시스템을 초기화합니다.
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_thread_names(true)
        .init();

    Some(guard)
}
