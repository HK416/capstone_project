use tokio::net::{TcpListener, TcpStream};
use std::sync::Mutex;

use server::{
    world::*,
    session::Session,
};



pub async fn run_server(addr: &str) {
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Failed to bind to address: {}", e);
            return;
        }
    };

    println!("Server listening on: {}", addr);

    let world = World::new();

    wait_for_players(listener, (&world).into()).await;
}


const MAX_CLIENTS: usize =  10;
/// World를 직접 읽으면 최신 데이터가 아닐 가능성이 있다.  
/// World에 Mutex, RwLock등을 걸면 클라이언트가 읽는데 병목이 생길 수 있다.  
/// 따라서 클라이언트 개수만 세기 위해 따로 분리.  
static CLIENT_SLOTS: Mutex<[Option<()>; MAX_CLIENTS]> = Mutex::new([None; MAX_CLIENTS]);


async fn wait_for_players(listener: TcpListener, world: WorldPointer) {
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let mut slots = CLIENT_SLOTS.lock().unwrap();
                let mut accepted = false;

                for id in 0..MAX_CLIENTS {
                    if slots[id].is_none() {
                        slots[id] = Some(());
                        // println!("Accepted connection from: {}", addr);
                        accepted = true;
                        tokio::spawn(handle_connection(id as u32, stream, world));
                        break;
                    }
                }
                if !accepted {
                    // 서버가 가득 차서 연결 거부
                }
            },
            Err(e) => {
                eprintln!("Failed to accept connection; err = {:?}", e);
            }
        }
    }
}


/// 별개의 스레드에서 동작하며, 시작시와 종료시 Mutex lock을 걸어서 클라이언트 개수 파악
async fn handle_connection(id: u32, stream: TcpStream, world: WorldPointer) {
    let mut session = Session::new(id, stream, WorldInterface::new(world));

    {
        let slots = CLIENT_SLOTS.lock().unwrap();
        println!("num clients: {}", slots.iter().filter(|x| x.is_some()).count());
    }   // lock 해제
    
    session.handle_connection().await;

    {
        let mut slots = CLIENT_SLOTS.lock().unwrap();
        slots[id as usize] = None;
        println!("Connection {} closed", id);

        println!("num clients: {}", slots.iter().filter(|x| x.is_some()).count());
    }
}



#[tokio::main]
async fn main() {
    run_server("localhost:7878").await;
}
