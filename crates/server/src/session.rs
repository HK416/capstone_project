use tokio::{
    net::TcpStream,
    io::{AsyncReadExt, AsyncWriteExt},
};
use super::world::WorldInterface;
use mod_network::*;
use mod_network::components::{
    ClientId, 
    // ObjectId, 
    StageKind,
    MovementState,
};


pub struct Session {
    id: ClientId,
    
    stream: TcpStream,
    packet_parser: PacketParser,

    world: WorldInterface,

    running: bool,

    // shot_count: u32,
}

impl Session {
    pub fn new(id: ClientId, stream: TcpStream, world: WorldInterface) -> Self {
        Self {
            id,
            stream,
            packet_parser: PacketParser::new(),
            world,
            running: true,
            // shot_count: 0,
        }
    }

    pub async fn handle_connection(&mut self) {
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
                PacketType::EnterStage => {
                    let enter_packet = EnterStagePacket::from_raw(packet);
                    self.world.add_player(self.id.into(), enter_packet.character_kind);

                    println!("Client {:?} entered stage", self.id);

                    let players = self.world.get_players();
                    let raw_packet = InitStagePacket::new(players, StageKind::Downtown).as_raw();
                    match self.stream_write(raw_packet).await {
                        Ok(_) => {

                        }, 
                        Err(_) => {
                            self.running = false;
                            return;
                        }
                    }
                },
                
                PacketType::PushStatus => {
                    let push_packet = PushStatusPacket::from_raw(packet);
                    let player = push_packet.player;
                    
                    self.world.update_player(player);

                    if player.movement_state == MovementState::Moving {
                        let dir = gmm::Vector::from_slice(&push_packet.move_direction)
                            .vec3_normalize();
                        // 속력값(캐릭터 능력치) 곱하기
                        let dir = dir * 12.3;
                        let dir = dir.store_float3();
                        self.world.push_move_data(self.id.into(), dir.x, 0.0, dir.z);
                    }
                    
                    let players = self.world.get_players();
                    let bullets = self.world.get_bullets();
                    let raw_packet = PullStagePacket::new(players, bullets).as_raw();
                    match self.stream_write(raw_packet).await {
                        Ok(_) => {

                        }, 
                        Err(_) => {
                            self.running = false;
                            return;
                        }
                    }
                }, 

                // PacketType::FIRED => {
                //     let fired_packet = ShotPacket::from_raw(packet);
                    
                //     self.shot_count += 1;
                //     self.shot_count %= 1000;        // 총알 번호는 0 ~ 999

                //     let mut bullet = fired_packet.bullet;
                //     let bid: u32 = self.id.into();
                //     bullet.id = ObjectId::new(bid * 1000 + self.shot_count);     // 총알 ID는 클라이언트 ID * 1000 + 총알 번호

                //     self.world.add_bullet(bullet);
                // }

                _ => {},
            }
        }
    }

    async fn stream_write(&mut self, packet: RawPacket) -> Result<(), std::io::Error> {
        self.stream.write_all(&packet.as_bytes()).await
    }
}