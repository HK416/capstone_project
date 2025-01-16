use futures::future::join_all;
use tokio::{
    net::TcpStream,
    io::{AsyncReadExt, AsyncWriteExt},
};
use rand::Rng;
use std::env;
use std::str::FromStr;

use mod_network::*;
use mod_math::LatLon;


async fn run_client(addr: &str, idx: usize, wait: f32) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut parser = PacketParser::new();

    let mut player = Player::default();
    player.translation[0] = (0.0 + idx as f32 % 10.0) / 10.0;
    player.translation[2] = (0.0 + idx as f32 / 10.0) / 10.0;

    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer).await {
            Ok(n) =>{
                parser.push(&buffer[..n]);
            },
            Err(e) => {
                eprintln!("failed to read from socket; err = {:?}", e);
                return;
            }
        }

        if let Some(raw_packet) = parser.pop() {
            if raw_packet.packet_type() == PacketType::Connect {
                let packet = ConnectPacket::from_raw(raw_packet);
                player.id = packet.client_id.into();

                break;
            }
        }
    }

    let packet = PushStatusPacket::default().as_raw();
    stream.write_all(&packet.as_bytes()).await.unwrap();

    let start = std::time::Instant::now();

    loop {
        match stream.read(&mut buffer).await {
            Ok(n) =>{
                parser.push(&buffer[..n]);
            },
            Err(e) => {
                eprintln!("failed to read from socket; err = {:?}", e);
                break;
            }
        }

        while let Some(raw_packet) = parser.pop() {
            match raw_packet.packet_type() {
                PacketType::PullStage => {
                    let packet = PullStagePacket::from_raw(raw_packet);
                    for p in packet.players {
                        if p.id == player.id {
                            player = p;
                        }
                    }
                },
                _ => {
                    eprintln!("unexpected packet type: {:?}", raw_packet.packet_type());
                }
            }
        }
        
        if start.elapsed().as_secs_f32() >= wait {
            stream.shutdown().await.unwrap();
            return;
        }

        let packet = PushStatusPacket::default().as_raw();
        stream.write_all(&packet.as_bytes()).await.unwrap();
    }
}


async fn player_add_remove(addr: &str, idx: usize) {
    loop {
        let mut rng = rand::thread_rng();
        let wait = rng.gen_range(1.0..2.0);
        run_client(addr, idx, wait).await;
        let wait = rng.gen_range(1.0..2.0);
        tokio::time::sleep(std::time::Duration::from_secs_f32(wait)).await;
    }
}


#[tokio::main]
async fn main() {
    let mut args = env::args();
    args.next();
    let num_clients = match args.next() {
        Some(num) => match num.parse::<usize>() {
            Ok(num) => num,
            Err(_) => {
                eprintln!("invalid number of clients: '{}'.\n  number of clients must be a unsigned integer.", num);
                eprintln!("usage: client <num_clients> <mode(or ip)>:<port>");
                return;
            }
        },
        None => 100
    };

    println!("add-remove {}", num_clients);

    let addr = match args.next() {
        Some(args) => match Addr::from_str(&args) {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        },
        None => Addr::default()
    }.to_string();

    let handles = (0..num_clients).map(|idx| player_add_remove(&addr, idx));

    join_all(handles).await;
}