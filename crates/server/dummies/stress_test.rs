use mod_network::components::{Email, LoginToken, Passwd, UserAccount, WorldId};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use mod_network::protocol::{
    CustomGameJoinRequestPacket, CustomGameJoinSuccessPacket, 
    CustomGamePullPacket, CustomGameReadyPacket, 
    LoginRequestPacket, LoginSuccessPacket, 
    Packet, PacketParser, PacketType
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

    async fn read(&mut self) -> Result<(), std::io::Error> {
        let mut buf = [0; 32];

        match self.reader.read(&mut buf).await {
            Ok(0) => {
                self.connected = false;
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Connection closed by server"));
            }
            Ok(n) => {
                println!("Received {} bytes: {:?}", n, &buf[..n]);
                self.packet_parser.push(&buf[..n]);
                return Ok(());
            }
            Err(e) => {
                self.connected = false;
                return Err(e);
            }
        }
    }

    async fn run(&mut self) -> Result<(), std::io::Error> {
        if !self.connected {
            return Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Client is not connected"));
        }
        
        self.login(Email::default(), Passwd::default()).await?; // 기본값으로 전송
        self.join(WorldId::NULL).await?;
        self.ready().await?;

        Ok(())
    }

    async fn login(&mut self, email: Email, passwd: Passwd) -> Result<(), std::io::Error> {
        let packet = LoginRequestPacket::new(email, passwd).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                println!("Parsed packet: {:?}", packet);
                if packet.packet_type() == PacketType::LoginSuccess {
                    let p = LoginSuccessPacket::from_raw(packet);
                    self.account = p.account;
                    self.token = p.token;
                    break 'readloop;
                }
            }
        }

        Ok(())
    }

    async fn join(&mut self, world_id: WorldId) -> Result<(), std::io::Error> {
        let packet = CustomGameJoinRequestPacket::new(world_id, self.account.uid, self.token).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                println!("Parsed packet: {:?}", packet);
                if packet.packet_type() == PacketType::CustomGameJoinSuccess {
                    let p = CustomGameJoinSuccessPacket::from_raw(packet);
                    println!("Join success: {:?}", p);
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
                println!("Parsed packet: {:?}", packet);
                if packet.packet_type() == PacketType::CustomGamePull {
                    let p = CustomGamePullPacket::from_raw(packet);
                    println!("Pull: {:?}", p);
                    break 'readloop;
                }
            }
        }

        Ok(())
    }
}


#[tokio::main]
async fn main() {
    println!("Stress test started");

    let mut client = Client::new(UserAccount::default()).await.unwrap();
    client.run().await.unwrap();

    println!("Stress test finished");
}