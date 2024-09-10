use tokio::{
    net::TcpStream,
    io::{AsyncReadExt, AsyncWriteExt},
};
use super::world::WorldInterface;
use mod_network::*;


pub struct Session {
    id: u32,
    
    stream: TcpStream,
    packet_parser: PacketParser,

    world: WorldInterface,

    running: bool,
}

impl Session {
    pub fn new(id: u32, stream: TcpStream, world: WorldInterface) -> Self {
        Self {
            id,
            stream,
            packet_parser: PacketParser::new(),
            world,
            running: true,
        }
    }

    pub async fn handle_connection(&mut self) {
        self.world.add_player(self.id);

        match self.stream_write(InitPacket::new(self.id, self.world.get_objects()).as_raw()).await {
            Ok(_) => {
                // println!("Client {} connected", self.id);
            },
            Err(_) => {
                self.running = false;
                return;
            }
        }

        let mut buf = [0; 1024];
    
        while self.running {
            let read = self.stream.read(&mut buf).await;
    
            match read {
                Ok(0) => break,     // Connection closed
    
                Ok(n) => {
                    self.process_packets(&buf[..n]).await;
                },
    
                Err(_) => break,
            };
        }

        self.world.remove_player(self.id);
    }

    
    async fn process_packets(&mut self, data: &[u8]) {
        self.packet_parser.push(data);

        while let Some(packet) = self.packet_parser.pop() {
            match packet.packet_type() {
                PacketType::PUSH => {
                    let push_packet = PushPacket::from_raw(packet);
                    self.world.update_player(push_packet.player);

                    let world = self.world.get_objects();
                    let raw_packet = PullPacket::new(world).as_raw();
                    match self.stream_write(raw_packet).await {
                        Ok(_) => {

                        }, 
                        Err(_) => {
                            self.running = false;
                            return;
                        }
                    }
                }, 
                PacketType::MOVE => {
                    let move_packet = MovePacket::from_raw(packet);
                    self.world.move_player(self.id, move_packet.x, move_packet.y, move_packet.z);
                },
                PacketType::MESSAGE => {
                    let message_packet = MessagePacket::from_raw(packet);
                    if message_packet.msg == "ping" {
                        self.stream_write(MessagePacket::new(message_packet.time, "pong").as_raw()).await.unwrap();
                    }
                },
                _ => {},
            }
        }
    }

    async fn stream_write(&mut self, packet: RawPacket) -> Result<(), std::io::Error> {
        self.stream.write_all(&packet.as_bytes()).await
    }
}