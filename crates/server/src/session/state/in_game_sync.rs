use std::{
    fmt,
    sync::{Arc, Weak},
};

use mod_network::{
    components::UserAccount,
    protocol::{Packet, PacketType, PushSyncPacket, RawPacket},
};

use crate::{
    session::{Session, SessionEvents},
    token::UserTokenMap,
    world::{GameWorld, GameWorldEvent},
};

use super::{SessionState, SessionStateFlow, in_game_prepare::SessionInGamePrepareState};

pub struct SessionInGameSyncState {
    /// 세션 상태 실행 여부
    is_running: bool,

    /// 사용자 계정 데이터
    account: UserAccount,
    /// 연결된 게임 월드
    world: Weak<GameWorld>,
}

impl SessionInGameSyncState {
    pub fn new(account: UserAccount, world: &Weak<GameWorld>) -> Self {
        Self {
            is_running: true,
            account,
            world: world.clone(),
        }
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
        if let Some(world) = self.world.upgrade() {
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
        match event {
            SessionEvents::EnterStage => {
                // 다음 세션 상태로 전환합니다.
                self.is_running = false;
                let next_state = SessionInGamePrepareState::new(self.account, &self.world);
                let control_flow = SessionStateFlow::Change(Box::new(next_state));
                let event = SessionEvents::SetControlFlow(control_flow);
                session.push_event(event);
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
