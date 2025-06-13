use std::sync::{Arc, Weak};

use mod_network::{
    components::UserId,
    protocol::{Packet, PacketType, RawPacket, RoomLeaveNotifyPacket, RoomReadyRequestPacket},
};

use crate::{
    session::Session,
    token::UserTokenMap,
    world::{GameWorld, GameWorldEvent, GameWorldRoomStateEvent},
};

use super::{SessionState, SessionStateFlow};

pub struct SessionRoomState {
    /// 사용자 식별자
    uid: UserId,
    /// 참가한 게임 월드
    world: Weak<GameWorld>,
    /// 유효하지 않은 패킷 경고 횟수
    packet_warn_count: usize,
}

impl SessionRoomState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(uid: UserId, world: Arc<GameWorld>) -> Self {
        Self {
            uid,
            world: Arc::downgrade(&world),
            packet_warn_count: 0,
        }
    }

    /// [`RoomLeaveNotifyPacket`]을 처리합니다.
    fn handle_room_leave_notify_packet(
        &mut self,
        session: &Arc<Session>,
        packet: RoomLeaveNotifyPacket,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::RoomLeaveNotify
            );
            session.close();
            return;
        }

        // 커스텀 게임 대기실 객체를 가져옵니다.
        if let Some(world) = self.world.upgrade() {
            // 다음 세션 상태로 전환합니다.
            session.flows.push(SessionStateFlow::Pop);
        } else {
            log::error!("{} accesses an invalid custom game", session);
            session.close();
            return;
        }
    }

    /// [`RoomReadyRequestPacket`]을 처리합니다.
    fn handle_room_ready_request_packet(
        &mut self,
        session: &Arc<Session>,
        packet: RoomReadyRequestPacket,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::RoomReadyRequest
            );
            session.close();
            return;
        }

        // 커스텀 게임 대기실 객체를 가져옵니다.
        if let Some(world) = self.world.upgrade() {
            // 게임 준비 요청을 보냅니다.
            let event = GameWorldRoomStateEvent::Ready(packet.ready_to_play);
            let event = GameWorldEvent::RoomState {
                session: session.clone(),
                uid: packet.uid,
                event,
            };
            world.push_event(event);
        } else {
            log::error!("{} accesses an invalid custom game", session);
            session.close();
            return;
        }
    }
}

impl SessionState for SessionRoomState {
    fn on_resume(&mut self, _session: &Arc<Session>) {
        self.packet_warn_count = 0;
    }

    fn on_exit(&mut self, session: &Arc<Session>) {
        // 커스텀 게임 대기실에서 플레이어를 제거합니다.
        if let Some(world) = self.world.upgrade() {
            world.exit(self.uid, session.clone());
        }
    }

    fn handle_packets(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::RoomLeaveNotify => {
                let packet = match RoomLeaveNotifyPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_room_leave_notify_packet(session, packet);
            }
            PacketType::RoomReadyRequest => {
                let packet = match RoomReadyRequestPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_room_ready_request_packet(session, packet);
            }
            PacketType::ChangeTeamRequest => {}
            _ => {
                log::warn!(
                    "{} invalid packet received! (STATE:{:?}, PACKET:{:?})",
                    &session,
                    &self,
                    &packet_type
                );

                // 유효하지 않은 패킷 경고 횟수를 증가시킵니다.
                self.packet_warn_count += 1;
                // 일정 횟수를 초과한 경우 세션을 종료시킵니다.
                const MAX_WARN_COUNT: usize = 0;
                if self.packet_warn_count > MAX_WARN_COUNT {
                    log::info!("{} closed after exceeding warning limit.", &session);
                    session.close();
                }
            }
        }
    }
}
