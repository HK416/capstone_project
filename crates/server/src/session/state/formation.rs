use std::{
    fmt,
    sync::{Arc, Weak},
};

use mod_network::protocol::{FormationSelectPacket, Packet, PacketType, RawPacket};

use crate::{
    session::{Session, SessionEvents},
    token::UserTokenMap,
    world::{GameWorld, GameWorldEvent},
};

use super::{SessionState, SessionStateFlow, in_game_sync::SessionInGameSyncState};

pub struct SessionFormationState {
    /// 세션 상태 실행 여부
    is_running: bool,
    /// 연결된 게임 월드
    world: Option<Weak<GameWorld>>,
}

impl SessionFormationState {
    /// 새로운 세션 상태를 생성합니다.
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
impl SessionFormationState {
    /// `EnterInGameSync`이벤트를 처리합니다.
    fn handle_enter_in_game_sync_event(&mut self, session: &Arc<Session>) {
        // 다음 세션 상태로 전환합니다.
        self.is_running = false;
        let world = self.world.take().unwrap();
        let next_state = Box::new(SessionInGameSyncState::new(world));
        let control_flow = SessionStateFlow::Change(next_state);
        let event = SessionEvents::SetControlFlow(control_flow);
        session.push_event(event);
    }

    fn handle_exit_formation_event(&mut self, session: &Arc<Session>) {
        // 다음 세션 상태로 전환합니다.
        self.is_running = false;
        let control_flow = SessionStateFlow::Pop;
        let event = SessionEvents::SetControlFlow(control_flow);
        session.push_event(event);
    }

    /// `FormationSelectPacket`을 처리합니다.
    fn handle_formation_select_packet(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet = match FormationSelectPacket::try_from_raw(packet) {
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

        // 커스텀 게임 대기실 객체를 가져옵니다.
        let world = self.world.as_ref().unwrap();
        if let Some(world) = world.upgrade() {
            // 캐릭터 선택 이벤트를 추가합니다.
            let event = GameWorldEvent::SelectCharacter {
                session: session.clone(),
                uid: packet.user_id,
                kind: packet.character_kind,
            };
            world.push_event(event);
        } else {
            log::warn!("{} accesses an invalid world", session);
            session.close();
            return;
        }
    }
}

impl SessionState for SessionFormationState {
    fn handle_event(&mut self, event: SessionEvents, session: &Arc<Session>) {
        // 세션 상태가 실행 중이 아닌 경우 함수 실행을 생략합니다.
        if !self.is_running {
            return;
        }

        match event {
            SessionEvents::EnterInGameSync => {
                self.handle_enter_in_game_sync_event(session);
            }
            SessionEvents::ExitFormation => {
                self.handle_exit_formation_event(session);
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
                PacketType::FormationSelect => {
                    self.handle_formation_select_packet(session, packet);
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

impl fmt::Debug for SessionFormationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionFormationState))
    }
}
