use std::sync::Arc;

use mod_network::protocol::{EnterStagePacket, Packet, PacketType};

use crate::{session::Session, world::World};

use super::{in_game::InGameState, ControlFlow, SessionState};

#[derive(Debug)]
pub struct LobbyState;

impl LobbyState {
    /// `EnterStagePacket`을 처리합니다.
    fn handle_enter_stage_packet(
        &mut self,
        packet: EnterStagePacket,
        flow: &mut Option<ControlFlow>,
        session: &Arc<Session>,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if packet.token != session.token {
            log::warn!(
                "{} invalid token (SESSION:{}, PACKET:{})",
                &session,
                &session.token.to_string(),
                &packet.token.to_string(),
            );
            session.close();
            return;
        }

        // 게임 월드에 플레이어를 추가합니다.
        // TODO: 나중에 매칭 대기열에 추가하는 것으로 변경해야 함.
        let world = World::get_instance();
        world.join(session.clone(), packet.character_kind);

        // 다음 상태로 전환합니다.
        let next_state = Box::new(InGameState::new(world));
        *flow = Some(ControlFlow::Push(next_state));
    }
}

impl SessionState for LobbyState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::EnterStage => match EnterStagePacket::try_from_raw(packet) {
                    Some(packet) => self.handle_enter_stage_packet(packet, flow, session),
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
            }
        }
    }
}
