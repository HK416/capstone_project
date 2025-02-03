use std::sync::Arc;

use mod_network::protocol::{ConnectPacket, Packet};

use super::{Session, SessionState};

/// 클라이언트 패킷을 처리합니다.
pub fn handle_packets(session: &Arc<Session>) -> SessionState {
    // `ConnectPacket`을 전송합니다.
    let packet = ConnectPacket::new(session.client_id);
    session.tcp_sender.push(packet.as_raw());
    SessionState::Lobby
}
