use std::sync::Arc;

use mod_network::{
    components::UserId,
    protocol::{
        MatchCancelPacket, MatchRequestRejectedPacket, MatchRequestRejectedReason, Packet,
        PacketType, RawPacket,
    },
};

use crate::{matching::MatchMaker, session::Session, token::UserTokenMap};

use super::{SessionState, SessionStateFlow};

/// 클라이언트가 랜덤매치 대기 장면에 위치하고 있는 상태입니다.
pub struct SessionQueuedState {
    /// 사용자 식별자
    uid: UserId,
    /// 유효하지 않은 패킷 수신 경고 횟수
    packet_warn_count: usize,
}

impl SessionQueuedState {
    /// 새로운 `QueuedState`를 생성합니다.
    pub const fn new(uid: UserId) -> Self {
        Self {
            uid,
            packet_warn_count: 0,
        }
    }

    /// `MatchCancelPacket`을 처리합니다.
    fn handle_match_cancel_packet(&mut self, session: &Arc<Session>, packet: MatchCancelPacket) {
        // 수신한 패킷이 올바른지 검사합니다.
        if self.uid != packet.uid {
            log::error!(
                "{} invalid identifier (PACKET:{:?})",
                &session,
                &PacketType::MatchCancel
            );
            session.close();
            return;
        }

        // 수신한 패킷이 올바른지 검사합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token (PACKET:{:?})",
                &session,
                &PacketType::MatchCancel
            );
            session.close();
            return;
        }

        MatchMaker::remove_from_queue(self.uid);

        // 패킷을 전송합니다.
        let reason = MatchRequestRejectedReason::Canceled;
        let packet = MatchRequestRejectedPacket::new(reason);
        session.tcp_write(packet.as_raw());

        // 이전 세션 상태로 돌아갑니다.
        session.flows.push(SessionStateFlow::Pop);
    }
}

impl SessionState for SessionQueuedState {
    fn handle_packets(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::MatchCancel => {
                let packet = match MatchCancelPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_match_cancel_packet(session, packet);
            }
            PacketType::MatchRequest => {
                // ...
            }
            PacketType::CharacterSelectRequest => { /* empty */ }
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
}
