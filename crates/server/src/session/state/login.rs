use std::sync::Arc;

use mod_network::{
    components::UserId,
    protocol::{
        LoginFailedPacket, LoginFailedReason, LoginRequestPacket, Packet, PacketType, RawPacket,
    },
};

use crate::{account::AccountManager, session::Session};

use super::{SessionState, SessionStateFlow, lobby::SessionLobbyState};

/// 클라이언트가 서버에 로그인을 시도하는 상태입니다.
pub struct SessionLoginState {
    /// 유효하지 않은 패킷 수신 경고 횟수
    packet_warn_count: usize,
    /// 로그인 실패 횟수
    #[allow(dead_code)]
    login_failed_count: usize,
}

impl SessionLoginState {
    /// 새로운 `LoginState`를 생성합니다.
    pub const fn new() -> Self {
        Self {
            packet_warn_count: 0,
            login_failed_count: 0,
        }
    }

    /// `LoginRequestPacket`을 처리합니다.
    ///
    /// 현재는 로그인 데이터베이스가 없습니다.
    /// 로그인 요청 순서대로 사용자 계정을 할당 후 로그인 토큰을 발행합니다.
    ///
    fn handle_login_request_packet(&mut self, session: &Arc<Session>, packet: LoginRequestPacket) {
        let account = if packet.uid == UserId::NULL {
            // 사용자 계정을 할당합니다.
            AccountManager::alloc()
        } else {
            // 사용자 계정을 로드합니다.
            if let Some(account) = AccountManager::load(packet.uid) {
                account
            } else {
                // 사용자 계정이 존재하지 않는 경우 로그인 실패 패킷을 전송합니다.
                let packet = LoginFailedPacket::new(LoginFailedReason::Invalid);
                session.tcp_write(packet.as_raw());
                return;
            }
        };

        // 다음 세션 상태로 전환합니다.
        let next_state = Box::new(SessionLobbyState::new(account));
        let flow = SessionStateFlow::Change(next_state);
        session.flows.push(flow);
    }
}

impl SessionState for SessionLoginState {
    fn handle_packets(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::LoginRequest => {
                let packet = match LoginRequestPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        session.close();
                        return;
                    }
                };

                self.handle_login_request_packet(session, packet);
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
}
