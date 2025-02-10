use std::sync::Arc;

use mod_network::protocol::{EnterStagePacket, Packet, PacketType};

use crate::world::World;

use super::{Session, SessionState};

/// 클라이언트 패킷을 처리합니다.
pub fn handle_packets(session: &Arc<Session>) -> SessionState {
    let mut state = SessionState::Lobby;
    on_received_packets(session, &mut state);
    state
}

/// 수신된 패킷을 처리합니다.
fn on_received_packets(session: &Arc<Session>, state: &mut SessionState) {
    while let Some(packet) = session.received_packets.pop() {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::EnterStage => match EnterStagePacket::try_from_raw(packet) {
                Some(packet) => handle_enter_stage_packet(packet, session, state),
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

/// `EnterStagePacket`을 처리합니다.
fn handle_enter_stage_packet(
    packet: EnterStagePacket,
    session: &Arc<Session>,
    state: &mut SessionState,
) {
    // 수신한 패킷이 올바른지 검사합니다.
    if packet.client_id != session.client_id {
        log::warn!(
            "{} invalid client id (SESSION:{}, PACKET:{})",
            &session,
            &session.client_id.to_string(),
            &packet.client_id.to_string(),
        );
        session.close();
        return;
    }

    // 게임 월드에 플레이어를 추가합니다.
    // TODO: 나중에 매칭 대기열에 추가하는 것으로 변경해야 함.
    let world = World::get_instance();
    world.join(session.clone(), packet.character_kind);
    *state = SessionState::InGame(world);
}
