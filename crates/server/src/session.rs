use glam::Vec4Swizzles;
use tokio::{
    net::TcpStream,
    io::{AsyncReadExt, AsyncWriteExt},
};
use super::world::WorldInterface;
use mod_network::{
    components::{
        ClientId, 
        ObjectId, 
        MovementState,
        ActionState,
        Epoch,
    },
    protocol::{
        PacketParser, 
        PacketType, 
        Packet,
        RawPacket, 
        ConnectPacket, 
        EnterStagePacket, 
        PushStatusPacket, 
        InitStagePacket, 
        PullStagePacket,
    },
};


pub struct Session {
    id: ClientId,
    
    stream: TcpStream,
    packet_parser: PacketParser,

    world: WorldInterface,

    running: bool,

    recent_shot_time: tokio::time::Instant,
}

impl Session {
    pub fn new(id: ClientId, stream: TcpStream, world: WorldInterface) -> Self {
        Self {
            id,
            stream,
            packet_parser: PacketParser::new(),
            world,
            running: true,
            recent_shot_time: tokio::time::Instant::now(),
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
                    self.process_enter_stage(enter_packet).await;
                },
                
                PacketType::PushStatus => {
                    let push_packet = PushStatusPacket::from_raw(packet);
                    self.process_push_status(push_packet).await;
                }, 

                _ => {},
            }
        }
    }

    async fn process_enter_stage(&mut self, packet: EnterStagePacket) {
        self.world.add_player(self.id.into(), packet.character_kind);

        println!("Client {:?} entered stage", self.id);

        let players = self.world.get_players();
        let raw_packet = InitStagePacket::new(
            self.world.get_stage_kind(),
            Epoch::new(0), 
            self.id.into(), 
            players
        ).as_raw();
        match self.stream_write(raw_packet).await {
            Ok(_) => {

            }, 
            Err(_) => {
                self.running = false;
                return;
            }
        }
    }

    async fn process_push_status(&mut self, packet: PushStatusPacket) {     
        self.world.update_player(
            self.id.into(), 
            packet.rotation, 
            packet.action_state, 
            packet.movement_state, 
            packet.view_state, 
            packet.action_state_timer, 
            packet.movement_state_timer, 
            packet.view_state_timer,
        );

        match packet.movement_state {
            MovementState::Moving => {
                let dir = glam::Vec3A::from_slice(&packet.direction)
                    .normalize();
                // 속력값(캐릭터 능력치) 곱하기
                let dir = glam::Vec3::from(dir * 5.5);
                self.world.push_move_data(self.id.into(), dir.x, 0.0, dir.z);
            },
            
            MovementState::MoveToEnd => {
                self.world.push_move_data(self.id.into(), 0.0, 0.0, 0.0);
            },

            _ => {},
        }

        match packet.action_state {
            ActionState::Attack => {
                const SHOT_INTERVAL: f32 = 2.0;     // 캐릭터 애니메이션에 맞춰야함
                const SHOT_DELAY: f32 = 1.0;        // 캐릭터 애니메이션에 맞춰야함
                if self.recent_shot_time.elapsed().as_secs_f32() >= SHOT_INTERVAL {
                    if packet.action_state_timer.0 >= SHOT_DELAY {
                        self.recent_shot_time = tokio::time::Instant::now();
                        
                        println!("shot!");

                        let mut transform = glam::Mat4::from_translation(glam::vec3(0.0, 0.0, -1.0));
                        let rotation = glam::Mat4::from_rotation_y(packet.view_rotation.lon);
                        transform = rotation * transform;

                        let z_axis = transform.z_axis.xyz().normalize_or(glam::Vec3::Z);
                        let x_axis = glam::Vec3::Y.cross(z_axis);
                        let rotation = glam::Mat4::from_axis_angle(x_axis, packet.view_rotation.lat);
                        transform = rotation * transform;
                        
                        self.world.add_bullet(
                            ObjectId::new(uuid::Uuid::new_v4().as_u128()),
                            self.id,
                            transform,
                        );
                    }
                }
            }, 

            _ => {},
        }

        
        
        let players = self.world.get_players();
        let bullets = self.world.get_bullets();
        let raw_packet = PullStagePacket::new(Epoch::new(0), players, bullets).as_raw();
        match self.stream_write(raw_packet).await {
            Ok(_) => {

            }, 
            Err(_) => {
                self.running = false;
                return;
            }
        }
    }

    async fn stream_write(&mut self, packet: RawPacket) -> Result<(), std::io::Error> {
        self.stream.write_all(&packet.as_bytes()).await
    }
}