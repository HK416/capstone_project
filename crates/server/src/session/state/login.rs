use std::{fmt, sync::Arc};

use mod_network::protocol::{
    LoginRequestPacket, LoginSuccessPacket, Packet, PacketType, RawPacket,
};

use crate::{
    account::AccountManager,
    session::{Session, SessionEvents},
    token::UserTokenMap,
};

use super::{SessionState, SessionStateFlow, lobby::SessionLobbyState};

/// 클라이언트가 서버에 로그인을 시도하는 상태입니다.
pub struct SessionLoginState;

impl SessionLoginState {
    /// 새로운 `LoginState`를 생성합니다.
    pub fn new() -> Self {
        Self {}
    }

    /// `LoginRequestPacket`을 처리합니다.
    ///
    /// 현재는 로그인 데이터베이스가 없습니다.
    /// 로그인 요청 순서대로 사용자 계정을 할당 후 로그인 토큰을 발행합니다.
    ///
    fn handle_login_request_packet(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let _packet = match LoginRequestPacket::try_from_raw(packet) {
            Some(packet) => packet,
            None => {
                log::warn!("{} failed to convert packet!", session);
                session.close();
                return;
            }
        };

        // 사용자 계정을 할당합니다.
        let account = AccountManager::alloc();

        // 로그인 토큰을 발행합니다.
        let token = UserTokenMap::alloc((account.uid, session.addr));

        // 패킷을 생성하고 전송합니다.
        let packet = LoginSuccessPacket::new(account, token);
        session.tcp_write(packet.as_raw());

        // 다음 세션 상태로 전환합니다.
        let next_state = Box::new(SessionLobbyState::new(account));
        let control_flow = SessionStateFlow::Change(next_state);
        let event = SessionEvents::SetControlFlow(control_flow);
        session.push_event(event);
    }
}

impl SessionState for SessionLoginState {
    fn handle_packets(&mut self, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            // 취소된 패킷의 경우 스킵합니다.
            if session.packet_canceled() {
                continue;
            }

            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::LoginRequest => {
                    self.handle_login_request_packet(session, packet);
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
}

impl fmt::Debug for SessionLoginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LoginState))
    }
}
