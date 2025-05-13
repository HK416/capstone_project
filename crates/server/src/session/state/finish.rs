use std::{fmt, sync::Arc};

use mod_network::protocol::{FinishStageResponsePacket, Packet, PacketType, RawPacket};

use crate::{
    session::{Session, SessionEvents},
    token::UserTokenMap,
};

use super::{SessionState, SessionStateFlow};

pub struct SessionFinishState {
    /// 세션 상태 실행 여부
    is_running: bool,
}

impl SessionFinishState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new() -> Self {
        Self { is_running: true }
    }
}

//--------------------------------------------------------------------------------------------
// 처리와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl SessionFinishState {
    /// `FinishStageResponse` 패킷을 처리합니다.
    fn handle_finish_stage_response_packet(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet = match FinishStageResponsePacket::try_from_raw(packet) {
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

        // 이젠 세션 상태로 전환합니다.
        self.is_running = false;
        let control_flow = SessionStateFlow::Pop;
        let event = SessionEvents::SetControlFlow(control_flow);
        session.push_event(event);
    }
}

//--------------------------------------------------------------------------------------------

impl SessionState for SessionFinishState {
    fn handle_packets(&mut self, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            // 세션 상태가 실행 중이 아닌 경우 스킵합니다
            // 취소된 패킷의 경우 스킵합니다.
            if !self.is_running && session.packet_canceled() {
                continue;
            }
            
            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::FinishStageResponse => {
                    self.handle_finish_stage_response_packet(session, packet);
                }
                PacketType::PushStatus
                | PacketType::CustomGameLeave
                | PacketType::CustomGameReady => { /* empty */ }
                _ => {
                    log::warn!(
                        "{} invalid packet received! (STATE:{:?}, PACKET:{:?})",
                        &session,
                        &self,
                        &packet,
                    );
                    session.close();
                    return;
                }
            }
        }
    }
}

impl fmt::Debug for SessionFinishState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionFinishState))
    }
}
