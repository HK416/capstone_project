use mod_network::components::{Email, LoginToken, Passwd, UserAccount, UserId, WorldId};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use mod_network::protocol::{
    CustomGameJoinRequestPacket, CustomGameJoinSuccessPacket, 
    CustomGamePullPacket, CustomGameReadyPacket, 
    LoginRequestPacket, LoginSuccessPacket, 
    Packet, PacketParser, PacketType
};


#[tokio::main]
async fn main() {
    println!("Stress test started");

    let mut packet_parser = PacketParser::new();

    let mut client = TcpStream::connect("localhost:7878").await.unwrap();
    println!("Connected to server");

    println!();
    println!(" - - - ");
    println!();

    // verify
    // let packet = ClientVerifyPacket::new().as_raw();
    // println!("Sending packet: {:?}", packet);
    // client.write_all(&packet.as_bytes()).await.unwrap();
    
    let mut id = UserId::default();
    let mut token = LoginToken::default();
    let mut buf = [0; 10];

    // Login
    let packet = LoginRequestPacket::new(Email::default(), Passwd::default()).as_raw();
    println!("Sending packet: {:?}", packet);
    client.write_all(&packet.as_bytes()).await.unwrap();
    
    'login: loop {
        match client.read(&mut buf).await {
            Ok(0) => {
                println!("Connection closed by server");
                break;
            }
            Ok(n) => {
                println!("Received {} bytes: {:?}", n, &buf[..n]);
                packet_parser.push(&buf[..n]);
            }
            Err(e) => {
                println!("Error reading from server: {}", e);
                break;
            }
        }

        while let Some(packet) = packet_parser.pop() {
            println!("Parsed packet: {:?}", packet);
            assert_eq!(packet.packet_type(), PacketType::LoginSuccess);
            let p = LoginSuccessPacket::from_raw(packet);
            id = p.account.uid;
            token = p.token;
            break 'login;
        }
    }
    
    println!();
    println!(" - - - ");
    println!();

    if id == UserId::default() {
        println!("Login failed");
        return;
    }
    if token == LoginToken::default() {
        println!("Login failed");
        return;
    }

    // join(lobby)
    let packet = CustomGameJoinRequestPacket::new(
        WorldId::NULL,
        id,
        token,
    );
    println!("Sending packet: {:?}", packet);
    client.write_all(&packet.as_raw().as_bytes()).await.unwrap();

    'join: loop {
        match client.read(&mut buf).await {
            Ok(0) => {
                println!("Connection closed by server");
                break;
            }
            Ok(n) => {
                println!("Received {} bytes: {:?}", n, &buf[..n]);
                packet_parser.push(&buf[..n]);
            }
            Err(e) => {
                println!("Error reading from server: {}", e);
                break;
            }
        }

        while let Some(packet) = packet_parser.pop() {
            println!("Parsed packet: {:?}", packet);
            assert_eq!(packet.packet_type(), PacketType::CustomGameJoinSuccess);
            let p = CustomGameJoinSuccessPacket::from_raw(packet);
            println!("Join success: {:?}", p);
            break 'join;
        }
    }

    println!();
    println!(" - - - ");
    println!();

    // ready(room)
    let packet = CustomGameReadyPacket::new(
        id,
        token,
        true,
    );
    println!("Sending packet: {:?}", packet);
    client.write_all(&packet.as_raw().as_bytes()).await.unwrap();

    'ready: loop {
        match client.read(&mut buf).await {
            Ok(0) => {
                println!("Connection closed by server");
                break;
            }
            Ok(n) => {
                println!("Received {} bytes: {:?}", n, &buf[..n]);
                packet_parser.push(&buf[..n]);
            }
            Err(e) => {
                println!("Error reading from server: {}", e);
                break;
            }
        }

        while let Some(packet) = packet_parser.pop() {
            println!("Parsed packet: {:?}", packet);
            assert_eq!(packet.packet_type(), PacketType::CustomGamePull);
            let p = CustomGamePullPacket::from_raw(packet);
            println!("Pull: {:?}", p);
            break 'ready;
        }
    }

    println!("Stress test finished");
}