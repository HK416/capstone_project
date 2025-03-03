use std::sync::Arc;

use mod_network::protocol::{Packet, PacketType, PushStatusPacket};

use crate::{session::Session, world::GameWorld};

use super::{ControlFlow, SessionState};

#[derive(Debug)]
pub struct InGameState {
    world: Arc<GameWorld>,
}

impl InGameState {
    /// 샤로운 인게임 상태를 생성합니다.
    pub fn new(world: Arc<GameWorld>) -> Self {
        Self { world }
    }

    /// `PushStatusPacket`을 처리합니다.
    fn handle_push_status_packet(
        &mut self,
        packet: PushStatusPacket,
        _flow: &mut Option<ControlFlow>,
        session: &Arc<Session>,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if packet.token != session.token {
            log::warn!(
                "{} invalid token (SESSION:{}, PACKET:{})",
                &session,
                &session.token,
                &packet.token,
            );
            session.close();
            return;
        }

        self.world.get_mut_player(session.user.id(), |_, player| {
            if let Some(mut player) = player {
                *player.rotation_mut() = glam::Quat::from_array(packet.rotation);
                *player.direction_mut() = glam::Vec3A::from_array(packet.direction);
                player.update_state(packet.input_flags);
                player.set_view(
                    packet.view_state,
                    packet.view_state_timer,
                    packet.view_rotation,
                );
            }
        });
    }
}

impl SessionState for InGameState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        if let Some(packet) = session.received_packets.pop() {
            let mut last_packet = match packet.packet_type() {
                PacketType::PushStatus => match PushStatusPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                },
                _ => {
                    log::warn!("{} invalid packet received (PACKET:{:?})", session, packet);
                    session.close();
                    return;
                }
            };

            while let Some(packet) = session.received_packets.pop() {
                let packet = match packet.packet_type() {
                    PacketType::PushStatus => match PushStatusPacket::try_from_raw(packet) {
                        Some(packet) => packet,
                        None => {
                            session.close();
                            return;
                        }
                    },
                    _ => {
                        log::warn!("{} invalid packet received (PACKET:{:?})", session, packet);
                        session.close();
                        return;
                    }
                };

                if last_packet.epoch < packet.epoch {
                    self.handle_push_status_packet(last_packet, flow, session);
                    last_packet = packet;
                }
            }

            self.handle_push_status_packet(last_packet, flow, session);
        }
    }

    fn on_exit(&mut self, session: &Arc<Session>) {
        self.world.exit(session);
    }
}
