use std::sync::Arc;

use mod_network::components::CharacterKind;
use mod_parallelism::collections::Queue;

use crate::session::Session;

use super::{ControlFlow, SessionState};

#[derive(Debug)]
pub struct FormationState {
    select_commands: Arc<Queue<(Arc<Session>, CharacterKind)>>,
}

impl FormationState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(select_commands: Arc<Queue<(Arc<Session>, CharacterKind)>>) -> Self {
        Self { select_commands }
    }
}

impl SessionState for FormationState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            let packet_type = packet.packet_type();
        }
    }
}
