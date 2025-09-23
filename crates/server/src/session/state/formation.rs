use std::sync::Arc;

use mod_network::{
    components::UserId,
    protocol::{
        CharacterReleaseNotifyPacket, CharacterSelectRequestPacket, Packet, PacketType, RawPacket,
        RoomLeaveNotifyPacket,
    },
};
use mod_parallelism::collections::Queue;

use crate::{
    session::{Session, SessionState, SessionStateFlow},
    token::UserTokenMap,
    world::{GameWorldEvent, GameWorldFormationStateEvent, GameWorldSystemEvent},
};

/// 지연 시간 (초)
const DELAY_TIME: f32 = 0.4;

/// 클라이언트가 캐릭터 편성 장면에 위치하고 있는 상태입니다.
pub struct SessionFormationState {
    /// 사용자 식별자
    uid: UserId,
    /// 게임 월드 이벤트 전송자
    sender: Arc<Queue<GameWorldEvent>>,
    /// 요청 지연 시간
    request_delay_time: f32,
    // 네트워크 상태 갱신을 위한 경과 시간
    elapsed_time_sec: f32,
    /// 유효하지 않은 패킷 경고 횟수
    packet_warn_count: usize,
}

impl SessionFormationState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(uid: UserId, sender: Arc<Queue<GameWorldEvent>>) -> Self {
        Self {
            uid,
            sender,
            request_delay_time: 0.0,
            elapsed_time_sec: 0.0,
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
        if self.uid != packet.uid {
            log::error!(
                "{} invalid identifier (PACKET:{:?})",
                &session,
                &PacketType::RoomLeaveNotify
            );
            session.close();
            return;
        }

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

        // 로비 세션 상태로 되돌아갑니다.
        session.flows.push(SessionStateFlow::Pop);
        session.flows.push(SessionStateFlow::Pop);
    }

    /// [`CharacterSelectRequestPacket`]을 처리합니다.
    fn handle_character_select_request_packet(
        &mut self,
        session: &Arc<Session>,
        packet: CharacterSelectRequestPacket,
    ) {
        // 지연 시간이 남은 경우 해당 패킷을 무시합니다.
        if self.request_delay_time > 0.0 {
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if self.uid != packet.uid {
            log::error!(
                "{} invalid identifier (PACKET:{:?})",
                &session,
                &PacketType::CharacterSelectRequest
            );
            session.close();
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::CharacterSelectRequest
            );
            session.close();
            return;
        }

        // 캐릭터 선택 요청을 보냅니다.
        let event = GameWorldFormationStateEvent::CharacterSelect(packet.character_kind);
        let event = GameWorldEvent::FormationState {
            session: session.clone(),
            uid: packet.uid,
            event,
        };
        self.sender.push(event);
        self.request_delay_time = DELAY_TIME;
    }

    /// [`CharacterReleaseNotifyPacket`]을 처리합니다.
    fn handle_character_release_notify_packet(
        &mut self,
        session: &Arc<Session>,
        packet: CharacterReleaseNotifyPacket,
    ) {
        // 지연 시간이 남은 경우 해당 패킷을 무시합니다.
        if self.request_delay_time > 0.0 {
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if self.uid != packet.uid {
            log::error!(
                "{} invalid identifier (PACKET:{:?})",
                &session,
                &PacketType::CharacterReleaseNotify
            );
            session.close();
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::CharacterReleaseNotify
            );
            session.close();
            return;
        }

        // 캐릭터 선택 해제 요청을 보냅니다.
        let event = GameWorldFormationStateEvent::CharacterRelease;
        let event = GameWorldEvent::FormationState {
            session: session.clone(),
            uid: packet.uid,
            event,
        };
        self.sender.push(event);
        self.request_delay_time = DELAY_TIME;
    }
}

impl SessionState for SessionFormationState {
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
            PacketType::CharacterSelectRequest => {
                let packet = match CharacterSelectRequestPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_character_select_request_packet(session, packet);
            }
            PacketType::CharacterReleaseNotify => {
                let packet = match CharacterReleaseNotifyPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_character_release_notify_packet(session, packet);
            }
            PacketType::RoomReadyRequest
            | PacketType::TeamChangeRequest
            | PacketType::DuplicateOptChangeRequest
            | PacketType::UnBalanceOptChangeRequest
            | PacketType::RoomPlayerBanRequest
            | PacketType::InGameReadyNotify => { /* empty */ }
            _ => {
                log::warn!(
                    "{} invalid packet received! (STATE:{:?}, PACKET:{:?})",
                    &session,
                    &self,
                    &packet_type,
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

    fn on_advanced(&mut self, session: &Arc<Session>, elapsed_time_sec: f32) {
        self.request_delay_time = (self.request_delay_time - elapsed_time_sec).max(0.0);
        self.elapsed_time_sec += elapsed_time_sec;

        const TICK: f32 = 1000.0;
        if self.elapsed_time_sec >= TICK {
            // 핑 갱신 요청을 보냅니다.
            self.elapsed_time_sec = 0.0;
            let event = GameWorldSystemEvent::UpdatePing(session.network_state());
            let event = GameWorldEvent::System {
                session: session.clone(),
                uid: self.uid,
                event,
            };
            self.sender.push(event);
        }
    }
}
