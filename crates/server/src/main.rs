use std::env;
use std::str::FromStr;
use std::sync::Mutex;
use tokio::net::{TcpListener, TcpStream};

use mod_network::{
    addr::Addr,
    components::{
        ClientId,
        StageKind,
    },
};

use server::{
    world::*,
    session::Session,
};


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

    let mut world = World::new(StageKind::default());

    // 새로운 쓰레드에서 클라이언트 연결 관리
    tokio::spawn(wait_for_players(listener, (&world).into()));

    // 메인 쓰레드에서 월드 업데이트
    world.update_loop().await;
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

                match slots.iter().position(|x| x.is_none()) {
                    Some(id) => {
                        slots[id] = Some(());
                        println!("Accepted connection from: {}", id + 1);
                        tokio::spawn(handle_connection(id as u32, stream, world));
                    },
                    None => {
                        // 입장 거부
                    },
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
    let mut session = Session::new(ClientId::new(id as u128 + 1), stream, WorldInterface::new(world));

    {
        let slots = CLIENT_SLOTS.lock().unwrap();
        println!("num clients: {}", slots.iter().filter(|x| x.is_some()).count());
    }   // lock 해제
    
    session.handle_connection().await;

    {
        let mut slots = CLIENT_SLOTS.lock().unwrap();
        slots[id as usize] = None;
        println!("Connection {} closed", id + 1);

        println!("num clients: {}", slots.iter().filter(|x| x.is_some()).count());
    }
}



#[tokio::main]
async fn main() {
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
        None => Addr::default()
    };

    run_server(&addr.to_string()).await;
}



















#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use mod_network::{
        protocol::{
            Packet,
            PacketParser,
            PacketType,
            EnterStagePacket,
        },
        components::{ClientId, CharacterKind},
    };

    #[tokio::test]
    async fn test_connection() {
        tokio::spawn(run_server("localhost:7878"));

        let mut parser = PacketParser::new();

        let mut client_stream = TcpStream::connect("localhost:7878").await.unwrap();

        let mut buf = [0; 1024];
        if let Ok(n) = client_stream.read(&mut buf).await {
            parser.push(&buf[..n]);

            let packet = parser.pop().unwrap();
            assert_eq!(packet.packet_type(), PacketType::Connect);
        }

        let packet = EnterStagePacket::new(ClientId::new(1), CharacterKind::ArisOriginal).as_raw();
        client_stream.write(&packet.as_bytes()).await.unwrap();

        let mut buf = [0; 1024];
        if let Ok(n) = client_stream.read(&mut buf).await {
            parser.push(&buf[..n]);

            let packet = parser.pop().unwrap();
            assert_eq!(packet.packet_type(), PacketType::InitStage);
        }
    }
}