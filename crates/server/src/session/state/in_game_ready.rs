use std::sync::Arc;

use mod_network::{
    components::UserId,
    protocol::{InGameReadyNotifyPacket, Packet, PacketType, RawPacket},
};
use mod_parallelism::collections::Queue;

use crate::{
    session::{Session, SessionState},
    token::UserTokenMap,
    world::{GameWorldEvent, GameWorldInGameReadyStateEvent, GameWorldSystemEvent},
};

/// 클라이언트가 인게임 준비 장면에 위치하고 있는 상태입니다.
pub struct SessionInGameReadyState {
    /// 사용자 식별자
    uid: UserId,
    /// 게임 월드 이벤트 전송자
    sender: Arc<Queue<GameWorldEvent>>,
    // 네트워크 상태 갱신을 위한 경과 시간
    elapsed_time_sec: f32,
    /// 유효하지 않은 패킷 경고 횟수
    packet_warn_count: usize,
}

impl SessionInGameReadyState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(uid: UserId, sender: Arc<Queue<GameWorldEvent>>) -> Self {
        Self {
            uid,
            sender,
            elapsed_time_sec: 0.0,
            packet_warn_count: 0,
        }
    }

    /// [`InGameReadyNotifyPacket`]을 처리합니다.
    fn handle_in_game_ready_notify_packet(
        &mut self,
        session: &Arc<Session>,
        packet: InGameReadyNotifyPacket,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if self.uid != packet.uid {
            log::error!(
                "{} invalid identifier (PACKET:{:?})",
                &session,
                &PacketType::InGameReadyNotify
            );
            session.close();
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::InGameReadyNotify
            );
            session.close();
            return;
        }

        // 로드 완료 요청을 보냅니다.
        let event = GameWorldInGameReadyStateEvent::ReadyToPlay;
        let event = GameWorldEvent::InGameReadyState {
            session: session.clone(),
            uid: packet.uid,
            event,
        };
        self.sender.push(event);
    }
}

impl SessionState for SessionInGameReadyState {
    #[rustfmt::skip]
    fn handle_packets(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::CharacterSelectRequest | PacketType::CharacterReleaseNotify => { /* empty */ }
            PacketType::InGameReadyNotify => {
                let packet = match InGameReadyNotifyPacket::try_from_raw(packet){
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_in_game_ready_notify_packet(session, packet);
            }
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
        self.elapsed_time_sec += elapsed_time_sec;

        const TICK: f32 = 1.0;
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
