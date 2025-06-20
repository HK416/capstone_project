use std::sync::{Arc, Weak};

use mod_network::{
    components::UserId,
    protocol::{
        Packet, PacketType, RawPacket, RoomDuplicateOptChangeRequestPacket, RoomLeaveNotifyPacket,
        RoomPlayerBanRequestPacket, RoomReadyRequestPacket, RoomTeamChangeRequestPacket,
        RoomUnbalancedOptChangeRequestPacket,
    },
};

use crate::{
    session::Session,
    token::UserTokenMap,
    world::{GameWorld, GameWorldEvent, GameWorldRoomStateEvent},
};

use super::{SessionState, SessionStateFlow};

/// 지연 시간 (초)
const DELAY_TIME: f32 = 0.5;

pub struct SessionRoomState {
    /// 사용자 식별자
    uid: UserId,
    /// 참가한 게임 월드
    world: Weak<GameWorld>,
    /// 요청 지연 시간
    request_delay_time: f32,
    /// 유효하지 않은 패킷 경고 횟수
    packet_warn_count: usize,
}

impl SessionRoomState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(uid: UserId, world: Arc<GameWorld>) -> Self {
        Self {
            uid,
            world: Arc::downgrade(&world),
            request_delay_time: 0.0,
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

        // 이전 세션 상태로 되돌아갑니다.
        session.flows.push(SessionStateFlow::Pop);
    }

    /// [`RoomReadyRequestPacket`]을 처리합니다.
    fn handle_room_ready_request_packet(
        &mut self,
        session: &Arc<Session>,
        packet: RoomReadyRequestPacket,
    ) {
        // 지연 시간이 남은 경우 해당 패킷을 무시합니다.
        if self.request_delay_time > 0.0 {
            return;
        }

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
            let event = GameWorldRoomStateEvent::Ready;
            let event = GameWorldEvent::RoomState {
                session: session.clone(),
                uid: packet.uid,
                event,
            };
            world.push_event(event);
            self.request_delay_time = DELAY_TIME;
        } else {
            log::error!("{} accesses an invalid custom game", session);
            session.close();
            return;
        }
    }

    /// [`RoomTeamChangeRequestPacket`]을 처리합니다.
    fn handle_room_team_change_request_packet(
        &mut self,
        session: &Arc<Session>,
        packet: RoomTeamChangeRequestPacket,
    ) {
        // 지연 시간이 남은 경우 해당 패킷을 무시합니다.
        if self.request_delay_time > 0.0 {
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::TeamChangeRequest
            );
            session.close();
            return;
        }

        // 커스텀 게임 대기실 객체를 가져옵니다.
        if let Some(world) = self.world.upgrade() {
            // 팀 변경 요청을 보냅니다.
            let event = GameWorldRoomStateEvent::ChangeTeam(packet.target);
            let event = GameWorldEvent::RoomState {
                session: session.clone(),
                uid: packet.uid,
                event,
            };
            world.push_event(event);
            self.request_delay_time = DELAY_TIME;
        } else {
            log::error!("{} accesses an invalid custom game", session);
            session.close();
            return;
        }
    }

    /// [`RoomPlayerBanRequestPacket`]을 처리합니다.
    fn handle_room_player_ban_request_packet(
        &mut self,
        session: &Arc<Session>,
        packet: RoomPlayerBanRequestPacket,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::RoomPlayerBanRequest
            );
            session.close();
            return;
        }

        // 커스텀 게임 대기실 객체를 가져옵니다.
        if let Some(world) = self.world.upgrade() {
            // 팀 변경 요청을 보냅니다.
            let event = GameWorldRoomStateEvent::PlayerBan(packet.target);
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

    /// [`RoomDuplicateOptChangeRequestPacket`]을 처리합니다.
    fn handle_room_duplicate_opt_change_request_packet(
        &mut self,
        session: &Arc<Session>,
        packet: RoomDuplicateOptChangeRequestPacket,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::DuplicateOptChangeRequest
            );
            session.close();
            return;
        }

        // 커스텀 게임 대기실 객체를 가져옵니다.
        if let Some(world) = self.world.upgrade() {
            // 팀 변경 요청을 보냅니다.
            let event = GameWorldRoomStateEvent::ChangeDuplicateOption;
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

    /// [`RoomDuplicateOptChangeRequestPacket`]을 처리합니다.
    fn handle_room_unbalanced_opt_change_request_packet(
        &mut self,
        session: &Arc<Session>,
        packet: RoomUnbalancedOptChangeRequestPacket,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::UnBalanceOptChangeRequest
            );
            session.close();
            return;
        }

        // 커스텀 게임 대기실 객체를 가져옵니다.
        if let Some(world) = self.world.upgrade() {
            // 팀 변경 요청을 보냅니다.
            let event = GameWorldRoomStateEvent::ChangeUnbalanceOption;
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
            PacketType::TeamChangeRequest => {
                let packet = match RoomTeamChangeRequestPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_room_team_change_request_packet(session, packet);
            }
            PacketType::DuplicateOptChangeRequest => {
                let packet = match RoomDuplicateOptChangeRequestPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };
                self.handle_room_duplicate_opt_change_request_packet(session, packet);
            }
            PacketType::UnBalanceOptChangeRequest => {
                let packet = match RoomUnbalancedOptChangeRequestPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_room_unbalanced_opt_change_request_packet(session, packet);
            }
            PacketType::RoomPlayerBanRequest => {
                let packet = match RoomPlayerBanRequestPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_room_player_ban_request_packet(session, packet);
            }
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

    fn on_advanced(&mut self, _session: &Arc<Session>, elapsed_time_sec: f32) {
        self.request_delay_time = (self.request_delay_time - elapsed_time_sec).max(0.0);
    }
}
