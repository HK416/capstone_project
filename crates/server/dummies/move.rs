use futures::future::join_all;
use tokio::{
    net::TcpStream,
    io::{AsyncReadExt, AsyncWriteExt},
};
use rand::Rng;
use std::env;
use std::str::FromStr;

use mod_network::*;


async fn run_client(addr: &str) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut parser = PacketParser::new();

    let mut player = Player::default();

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
            if raw_packet.packet_type() == PacketType::INIT {
                let packet = InitPacket::from_raw(raw_packet);
                player.id = packet.client_id;

                break;
            }
        }
    }

    let packet = PushPacket::new(player).as_raw();
    stream.write_all(&packet.as_bytes()).await.unwrap();

    let mut start = std::time::Instant::now();
    let mut rng = rand::thread_rng();
    let mut x = rng.gen_range(-1.0..=1.0);
    let mut y = rng.gen_range(-1.0..=1.0);

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
                PacketType::PULL => {
                    let packet = PullPacket::from_raw(raw_packet);
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

        if start.elapsed().as_secs() >= 1 {
            x = rng.gen_range(-1.0..=1.0);
            y = rng.gen_range(-1.0..=1.0);
            start = std::time::Instant::now();
        }

        player.translation.x += x * 0.00001;
        player.translation.z += y * 0.00001;
        let packet = PushPacket::new(player).as_raw();
        stream.write_all(&packet.as_bytes()).await.unwrap();
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

    println!("client {}", num_clients);

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

    let handles = (0..num_clients).map(|_| run_client(&addr));

    join_all(handles).await;
}