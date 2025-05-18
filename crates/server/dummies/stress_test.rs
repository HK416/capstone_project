use mod_network::components::{Email, LoginToken, Passwd, UserAccount, WorldId};
use rand::seq::IndexedRandom;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use mod_network::protocol::{
    CustomGameJoinRequestPacket, CustomGameJoinSuccessPacket, 
    CustomGamePullPacket, CustomGameReadyPacket, 
    LoginRequestPacket, LoginSuccessPacket, 
    Packet, PacketParser, PacketType, 
    AvailableWorldsPacket, RequestAvailableWorldsPacket
};


struct Client {
    account: UserAccount,
    token: LoginToken,
    
    reader: tokio::net::tcp::OwnedReadHalf,
    writer: tokio::net::tcp::OwnedWriteHalf,
    packet_parser: PacketParser,
    connected: bool,
}

impl Client {
    async fn new(account: UserAccount) -> Result<Self, std::io::Error> {
        let stream = TcpStream::connect("localhost:7878").await?;
        let (reader, writer) = stream.into_split();

        Ok(Self {
            account,
            token: LoginToken::default(),
            reader,
            writer,
            packet_parser: PacketParser::new(),
            connected: true,
        })
    }

    async fn run(&mut self) -> Result<(), std::io::Error> {
        if !self.connected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected, "Client is not connected"
            ));
        }
        
        // 일단 기본값으로 로그인. 나중에 DB에서 불러온 정보로 로그인하도록 해야함
        self.login(Email::default(), Passwd::default()).await?;

        // 방 접속 시도 
        loop {
            // 접속 가능한 월드 목록 불러오기
            let available = self.request_available_worlds().await?;

            let world_id = if available.is_empty() {
                println!("No available worlds, creating a new one...");
                // 방 생성
                WorldId::NULL
            } else {
                println!("Available worlds: {:?}", available);
                // 랜덤으로 방 선택
                *available.choose(&mut rand::rng()).unwrap()
            };

            //////////////////////////////////////////////////// **꽉 찬 방이 있어도 접속할 수 있다고 뜨는거같음**

            // 방 접속
            match self.join(world_id).await {
                Ok(_) => break,
                Err(_) => continue,
            }
        }

        // 준비 신호 전송
        self.ready().await?;

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Ok(())
    }

    async fn read(&mut self) -> Result<(), std::io::Error> {
        let mut buf = [0; 32];

        match self.reader.read(&mut buf).await {
            Ok(0) => {
                self.connected = false;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof, "Connection closed by server"
                ));
            }
            Ok(n) => {
                // println!("Received {} bytes: {:?}", n, &buf[..n]);
                self.packet_parser.push(&buf[..n]);
                return Ok(());
            }
            Err(e) => {
                self.connected = false;
                return Err(e);
            }
        }
    }

    async fn login(&mut self, email: Email, passwd: Passwd) -> Result<(), std::io::Error> {
        let packet = LoginRequestPacket::new(email, passwd).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                // println!("Parsed packet: {:?}", packet);
                if packet.packet_type() == PacketType::LoginSuccess {
                    let p = LoginSuccessPacket::from_raw(packet);
                    self.account = p.account;
                    self.token = p.token;
                    // println!("Login success: {:?}", p);
                    break 'readloop;
                }
            }
        }

        Ok(())
    }

    async fn request_available_worlds(&mut self) -> Result<Vec<WorldId>, std::io::Error> {
        let packet = RequestAvailableWorldsPacket::new(self.account.uid, self.token).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                // println!("Parsed packet: {:?}", packet);
                if packet.packet_type() == PacketType::AvailableWorlds {
                    let p = AvailableWorldsPacket::from_raw(packet);
                    return Ok(p.worlds);
                }
            }
        }
    }

    async fn join(&mut self, world_id: WorldId) -> Result<(), std::io::Error> {
        let packet = CustomGameJoinRequestPacket::new(world_id, self.account.uid, self.token).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                // println!("Parsed packet: {:?}", packet);
                if packet.packet_type() == PacketType::CustomGameJoinSuccess {
                    let p = CustomGameJoinSuccessPacket::from_raw(packet);
                    // println!("Join success: {:?}", p);
                    break 'readloop;
                }
            }
        }

        Ok(())
    }

    async fn ready(&mut self) -> Result<(), std::io::Error> {
        let packet = CustomGameReadyPacket::new(self.account.uid, self.token, true).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                // println!("Parsed packet: {:?}", packet);
                if packet.packet_type() == PacketType::CustomGamePull {
                    let p = CustomGamePullPacket::from_raw(packet);
                    // println!("Pull: {:?}", p);
                    break 'readloop;
                }
            }
        }

        Ok(())
    }
}


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    println!("Stress test started");

    loop {
        let mut client = Client::new(UserAccount::default()).await.unwrap();
        tokio::spawn(async move { client.run().await });

        // 접속속도 초당 10개로 제한
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("Stress test finished");

    Ok(())
}