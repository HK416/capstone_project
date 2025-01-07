use tokio::{
    net::TcpStream,
    io::{AsyncReadExt, AsyncWriteExt},
};
use super::world::WorldInterface;
use mod_network::*;
use mod_network::components::{ClientId, ObjectId};


pub struct Session {
    id: ClientId,
    
    stream: TcpStream,
    packet_parser: PacketParser,

    world: WorldInterface,

    running: bool,

    shot_count: u32,
}

impl Session {
    pub fn new(id: ClientId, stream: TcpStream, world: WorldInterface) -> Self {
        Self {
            id,
            stream,
            packet_parser: PacketParser::new(),
            world,
            running: true,
            shot_count: 0,
        }
    }

    pub async fn handle_connection(&mut self) {
        self.world.add_player(self.id.into());

        match self.stream_write(ConnectPacket::new(self.id).as_raw()).await {
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

        self.world.remove_player(self.id.into());
    }

    
    async fn process_packets(&mut self, data: &[u8]) {
        self.packet_parser.push(data);

        while let Some(packet) = self.packet_parser.pop() {
            match packet.packet_type() {
                PacketType::PUSH => {
                    let push_packet = PushPacket::from_raw(packet);
                    self.world.update_player(push_packet.player);

                    let players = self.world.get_players();
                    let bullets = self.world.get_bullets();
                    let raw_packet = PullPacket::new(players, bullets).as_raw();
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
                    self.world.move_player(self.id.into(), move_packet.x, move_packet.y, move_packet.z);
                },

                PacketType::MESSAGE => {
                    let message_packet = MessagePacket::from_raw(packet);
                    if message_packet.msg == "ping" {
                        self.stream_write(MessagePacket::new(message_packet.time, "pong").as_raw()).await.unwrap();
                    }
                },

                PacketType::FIRED => {
                    let fired_packet = ShotPacket::from_raw(packet);
                    
                    self.shot_count += 1;
                    self.shot_count %= 1000;        // 총알 번호는 0 ~ 999

                    let mut bullet = fired_packet.bullet;
                    let bid: u32 = self.id.into();
                    bullet.id = ObjectId::new(bid * 1000 + self.shot_count);     // 총알 ID는 클라이언트 ID * 1000 + 총알 번호

                    self.world.add_bullet(bullet);
                }

                _ => {},
            }
        }
    }

    async fn stream_write(&mut self, packet: RawPacket) -> Result<(), std::io::Error> {
        self.stream.write_all(&packet.as_bytes()).await
    }
}