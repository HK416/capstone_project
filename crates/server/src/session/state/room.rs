use std::sync::{Arc, Weak};

use mod_network::protocol::{
    CustomGameLeavePacket, CustomGamePushStatusPacket, Packet, PacketType, RawPacket,
};

use crate::{room::CustomGameRoom, session::Session, token::UserTokenMap};

use super::{ControlFlow, SessionState};

#[derive(Debug)]
pub struct RoomState {
    room: Weak<CustomGameRoom>,
}

impl RoomState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(room: &Arc<CustomGameRoom>) -> Self {
        Self {
            room: Arc::downgrade(room),
        }
    }

    /// `CustomGameLeavePacket`을 처리합니다.
    fn handle_custom_game_leave_packet(
        &mut self,
        flow: &mut Option<ControlFlow>,
        session: &Arc<Session>,
        packet: RawPacket,
    ) {
        let packet = match CustomGameLeavePacket::try_from_raw(packet) {
            Some(packet) => packet,
            None => {
                log::warn!("{} failed to convert packet!", session);
                session.close();
                return;
            }
        };

        // 수신한 패킷이 올바른지 검사합니다.
        let user_id = packet.user_id;
        let addr = session.addr;
        let token = packet.token;
        if !UserTokenMap::is_valid(&(user_id, addr), token) {
            log::warn!("{} invalid token (PACKET:{:?})", &session, &packet,);
            session.close();
            return;
        }

        // 커스텀 게임 대기실 객체를 가져옵니다.
        if let Some(room) = self.room.upgrade() {
            // 커스텀 게임 대기실에서 플레이어 정보를 제거합니다.
            room.exit(session);

            // 다음 세션 상태로 전환합니다.
            *flow = Some(ControlFlow::Pop);
        } else {
            log::warn!("{} accesses an invalid custom game", session);
            session.close();
            return;
        }
    }

    /// `CustomGamePushPacket`을 처리합니다.
    fn handle_custom_game_push_status_packet(
        &mut self,
        _flow: &mut Option<ControlFlow>,
        session: &Arc<Session>,
        packet: RawPacket,
    ) {
        let packet = match CustomGamePushStatusPacket::try_from_raw(packet) {
            Some(packet) => packet,
            None => {
                log::warn!("{} failed to convert packet!", session);
                session.close();
                return;
            }
        };

        // 수신한 패킷이 올바른지 검사합니다.
        let user_id = packet.user_id;
        let addr = session.addr;
        let token = packet.token;
        if !UserTokenMap::is_valid(&(user_id, addr), token) {
            log::warn!("{} invalid token (PACKET:{:?})", &session, &packet,);
            session.close();
            return;
        }

        // 커스텀 게임 대기실 객체를 가져옵니다.
        if let Some(room) = self.room.upgrade() {
            // 커스텀 게임 대기실에서 플레이어 정보를 제거합니다.
            if !room.access(session, |player| {
                player.status = packet.status;
            }) {
                log::warn!("{} accesses an invalid custom game player", session);
                session.close();
                return;
            }
        } else {
            log::warn!("{} accesses an invalid custom game", session);
            session.close();
            return;
        }
    }
}

impl SessionState for RoomState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::CustomGameLeave => {
                    self.handle_custom_game_leave_packet(flow, session, packet);
                }
                PacketType::CustomGamePushStatus => {
                    self.handle_custom_game_push_status_packet(flow, session, packet);
                }
                _ => {
                    log::warn!(
                        "{} invalid packet received! (STATE:{:?}, PACKET:{:?})",
                        &session,
                        &self,
                        &packet
                    );
                    session.close();
                    return;
                }
            }
        }
    }

    fn on_exit(&mut self, session: &Arc<Session>) {
        // 커스텀 게임 대기실에서 플레이어를 제거합니다.
        if let Some(room) = self.room.upgrade() {
            room.exit(session);
        }
    }
}
