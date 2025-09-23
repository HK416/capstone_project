use std::sync::Arc;

use mod_network::{
    components::UserId,
    protocol::{InGameControlLosePacket, InGameInputPacket, Packet, PacketType, RawPacket},
};
use mod_parallelism::collections::Queue;

use crate::{
    session::{Session, SessionState},
    token::UserTokenMap,
    world::{GameWorldEvent, GameWorldInGameRunStateEvent, GameWorldSystemEvent},
};

/// 클라이언트가 인게임 진행 장면에 위치하고 있는 상태입니다.
pub struct SessionInGameRunState {
    /// 사용자 식별자
    uid: UserId,
    /// 게임 월드 이벤트 전송자
    sender: Arc<Queue<GameWorldEvent>>,
    /// 네트워크 상태 갱신을 위한 경과 시간
    elapsed_time_sec: f32,
    /// 유효하지 않은 패킷 경고 횟수
    packet_warn_count: usize,
}

impl SessionInGameRunState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(uid: UserId, sender: Arc<Queue<GameWorldEvent>>) -> Self {
        Self {
            uid,
            sender,
            elapsed_time_sec: 0.0,
            packet_warn_count: 0,
        }
    }

    /// [`InGameInputPacket`]을 처리합니다.
    fn handle_in_game_input_event_packet(
        &mut self,
        session: &Arc<Session>,
        packet: InGameInputPacket,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if self.uid != packet.uid {
            log::error!(
                "{} invalid identifier (PACKET:{:?})",
                &session,
                &PacketType::InGameInput
            );
            session.close();
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::InGameInput
            );
            session.close();
            return;
        }

        // 이벤트를 전송합니다.
        let event = GameWorldInGameRunStateEvent::Input {
            client_play_elapsed_time: packet.play_elapsed_time_ms,
            snapshots: packet.snapshots,
        };
        let event = GameWorldEvent::InGameRunState {
            session: session.clone(),
            uid: self.uid,
            event,
        };
        self.sender.push(event);
    }

    /// [`InGameControlLosePacket`]을 처리합니다.
    fn handle_in_game_control_lose_packet(
        &mut self,
        session: &Arc<Session>,
        packet: InGameControlLosePacket,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        if self.uid != packet.uid {
            log::error!(
                "{} invalid identifier (PACKET:{:?})",
                &session,
                &PacketType::InGameInput
            );
            session.close();
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::InGameInput
            );
            session.close();
            return;
        }

        // 이벤트를 전송합니다.
        let event = GameWorldInGameRunStateEvent::InputReset;
        let event = GameWorldEvent::InGameRunState {
            session: session.clone(),
            uid: self.uid,
            event,
        };
        self.sender.push(event);
    }
}

impl SessionState for SessionInGameRunState {
    fn handle_packets(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::InGameInput => {
                let packet = match InGameInputPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_in_game_input_event_packet(session, packet);
            }
            PacketType::InGameControlLose => {
                let packet = match InGameControlLosePacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_in_game_control_lose_packet(session, packet);
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

    fn on_advanced(&mut self, session: &Arc<Session>, elapsed_time_sec: f32) {
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
