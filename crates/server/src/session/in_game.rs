use std::sync::Arc;

use mod_network::protocol::{Packet, PacketType, PushStatusPacket};

use crate::world::{World, WorldEvents};

use super::{Session, SessionState};

/// 클라이언트 패킷을 처리합니다.
pub fn handle_packets(session: &Arc<Session>, world: Arc<World>) -> SessionState {
    let mut state = SessionState::InGame(world.clone());
    on_received_packets(session, &mut state, &world);
    state
}

/// 수신된 패킷을 처리합니다.
fn on_received_packets(session: &Arc<Session>, state: &mut SessionState, world: &World) {
    while let Some(packet) = session.received_packets.pop() {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::PushStatus => match PushStatusPacket::try_from_raw(packet) {
                Some(packet) => handle_push_status_packet(packet, session, state, world),
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

/// `PushStatusPacket`을 처리합니다.
fn handle_push_status_packet(
    packet: PushStatusPacket,
    session: &Arc<Session>,
    _: &mut SessionState,
    world: &World,
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

    // 플레이어 상태를 갱신합니다.
    // TODO: `ActionState`, `ActionStateTimer`, `MovementState`, `MovementStateTimer`는 저장하지 않아도 됨. (판단을 위한 데이터)
    //
    // 곧 바로 게임 월드에 상태를 저장하는 것이 아닌
    // 이 함수에서 계산을 수행 후 이벤트 전송으로 변경해야 함.
    //
    world.send_event(WorldEvents::UpdatePlayerStatus(
        packet.epoch,
        session.client_id,
        glam::Quat::from_array(packet.rotation),
        glam::Vec3A::from_array(packet.direction),
        packet.action_state,
        packet.movement_state,
        packet.view_state,
        packet.view_rotation,
    ));
}
