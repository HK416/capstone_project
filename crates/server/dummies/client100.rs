use futures::future::join_all;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use mod_network::PacketParser;
use mod_network::Player;


async fn run_client(addr: &str) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut parser = PacketParser::new();

    
}

#[tokio::main]
async fn main() {
    println!("client100");

    let addr = "localhost:7878";

    let handles = (0..100).map(|_| run_client(addr));

    join_all(handles).await;
}