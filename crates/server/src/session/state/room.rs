use std::sync::{Arc, Weak};

use crate::{room::CustomGameRoom, session::Session};

use super::{ControlFlow, SessionState};

#[derive(Debug)]
pub struct RoomState {
    room: Weak<CustomGameRoom>,
}

impl RoomState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(room: &Arc<CustomGameRoom>) -> Self {
        Self {
            room: Arc::downgrade(room),
        }
    }
}

impl SessionState for RoomState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            // TODO
        }
    }

    fn on_exit(&mut self, session: &Arc<Session>) {
        // 커스텀 게임 대기실에서 플레이어를 제거합니다.
        if let Some(room) = self.room.upgrade() {
            room.exit(session);
        }
    }
}
