use std::sync::Arc;

use mod_network::protocol::{ConnectPacket, Packet};

use crate::session::Session;

use super::{lobby::LobbyState, ControlFlow, SessionState};

#[derive(Debug)]
pub struct InitState;

impl SessionState for InitState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        // `ConnectPacket`을 전송합니다.
        let packet = ConnectPacket::new(session.client_id);
        session.tcp_sender.push(packet.as_raw());

        // 다음 상태로 전환합니다.
        let next_state = Box::new(LobbyState);
        *flow = Some(ControlFlow::Change(next_state));
    }
}
