use std::{
    env,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering as MemOrdering},
        OnceLock,
    },
};

use mod_network::{addr::Addr, components::ClientId};
use mod_parallelism::collections::Queue;
use server::{
    session::handle_connection,
    world::{update_game_world, World},
};
use tokio::net::TcpListener;

static NUM_CLIENTS: AtomicU64 = AtomicU64::new(0);
static RETIRES: OnceLock<Queue<u64>> = OnceLock::new();

fn get_retires() -> &'static Queue<u64> {
    RETIRES.get_or_init(|| Queue::default())
}

/// 메인 쓰레드에서 월드 업데이트, 새로운 쓰레드를 생성해서 연결 관리
pub async fn run_server(addr: &str) {
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Failed to bind to address: {}", e);
            return;
        }
    };

    println!("Server listening on: {}", listener.local_addr().unwrap());

    // 새로운 쓰레드에서 클라이언트 연결 관리
    tokio::spawn(wait_for_players(listener));

    // 메인 쓰레드에서 월드 업데이트
    let world = World::get_instance();
    update_game_world(world).await;
}

async fn wait_for_players(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let value = get_retires()
                    .pop()
                    .unwrap_or_else(|| NUM_CLIENTS.fetch_add(1, MemOrdering::AcqRel) + 1);
                let client_id = ClientId::new(value as u128);
                println!("Accepted connection from: {}", client_id.to_string());
                tokio::spawn(async move {
                    handle_connection(client_id, stream).await;
                    get_retires().push(value);
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
