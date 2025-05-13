use std::{
    fmt,
    sync::{Arc, Weak},
};

use mod_network::protocol::{Packet, PacketType, PushSyncPacket, RawPacket};

use crate::{
    session::{Session, SessionEvents},
    token::UserTokenMap,
    world::{GameWorld, GameWorldEvent},
};

use super::{
    SessionState, SessionStateFlow, in_game::SessionInGameState,
    in_game_prepare::SessionInGamePrepareState,
};

pub struct SessionInGameSyncState {
    /// 세션 상태 실행 여부
    is_running: bool,
    /// 연결된 게임 월드
    world: Option<Weak<GameWorld>>,
}

impl SessionInGameSyncState {
    pub fn new(world: Weak<GameWorld>) -> Self {
        Self {
            is_running: true,
            world: Some(world),
        }
    }
}

//--------------------------------------------------------------------------------------------
// 처리와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl SessionInGameSyncState {
    /// `PrepareGame`이벤트를 처리합니다.
    fn handle_prepare_game_event(&mut self, session: &Arc<Session>) {
        // 다음 세션 상태로 전환합니다.
        self.is_running = false;
        let world = self.world.take().unwrap();
        let next_state = SessionInGamePrepareState::new(world);
        let control_flow = SessionStateFlow::Change(Box::new(next_state));
        let event = SessionEvents::SetControlFlow(control_flow);
        session.push_event(event);
    }

    /// `StartGamePlay`이벤트를 처리합니다.
    fn handle_start_game_play_event(&mut self, session: &Arc<Session>) {
        // 다음 세션 상태로 전환합니다.
        self.is_running = false;
        let world = self.world.take().unwrap();
        let next_state = SessionInGameState::new(world);
        let control_flow = SessionStateFlow::Change(Box::new(next_state));
        let event = SessionEvents::SetControlFlow(control_flow);
        session.push_event(event);
    }

    /// `PushSyncPacket`을 처리합니다.
    fn handle_push_sync_packet(&self, session: &Arc<Session>, packet: RawPacket) {
        let packet = match PushSyncPacket::try_from_raw(packet) {
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

        // 인게임 로드가 완료되지 않았을 경우 함수 실행을 생략합니다.
        if !packet.finish {
            return;
        }

        // 게임 월드 객체를 가져옵니다.
        let world = self.world.as_ref().unwrap();
        if let Some(world) = world.upgrade() {
            // 인게임 로드 완료 이벤트를 보냅니다.
            let event = GameWorldEvent::GameLoadFinish {
                session: session.clone(),
                uid: packet.user_id,
            };
            world.push_event(event);
        } else {
            log::warn!("{} accesses an invalid world", session);
            session.close();
            return;
        }
    }
}

impl SessionState for SessionInGameSyncState {
    fn handle_event(&mut self, event: SessionEvents, session: &Arc<Session>) {
        // 세션 상태가 실행 중이 아닌 경우 함수 실행을 생략합니다.
        if !self.is_running {
            return;
        }

        match event {
            SessionEvents::PrepareGame => {
                self.handle_prepare_game_event(session);
            }
            SessionEvents::StartGamePlay => {
                self.handle_start_game_play_event(session);
            }
            _ => {
                log::warn!(
                    "ignored >> unused session event (EVENT:{:?} STATE:{:?})",
                    &event,
                    &self
                );
            }
        }
    }

    fn handle_packets(&mut self, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            // 세션 상태가 실행 중이 아닌 경우 스킵합니다
            if !self.is_running {
                continue;
            }

            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::PushSync => {
                    self.handle_push_sync_packet(session, packet);
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

impl fmt::Debug for SessionInGameSyncState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionInGameSyncState))
    }
}
