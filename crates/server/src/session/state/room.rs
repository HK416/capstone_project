use std::{
    fmt,
    sync::{Arc, Weak},
};

use mod_network::{
    components::UserAccount,
    protocol::{CustomGameLeavePacket, CustomGameReadyPacket, Packet, PacketType, RawPacket},
};

use crate::{
    session::{Session, SessionEvents},
    token::UserTokenMap,
    world::{GameWorld, GameWorldEvent},
};

use super::{SessionState, SessionStateFlow, formation::SessionFormationState};

pub struct SessionRoomState {
    /// 세션 상태 실행 여부
    is_running: bool,

    /// 사용자 계정 데이터
    account: UserAccount,
    /// 연결된 게임 월드
    world: Weak<GameWorld>,
}

impl SessionRoomState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(account: UserAccount, world: &Arc<GameWorld>) -> Self {
        Self {
            is_running: true,
            account,
            world: Arc::downgrade(world),
        }
    }

    /// `CustomGameLeavePacket`을 처리합니다.
    fn handle_custom_game_leave_packet(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet = match CustomGameLeavePacket::try_from_raw(packet) {
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
        if let Some(world) = self.world.upgrade() {
            // 커스텀 게임 대기실에서 플레이어 정보를 제거합니다.
            world.exit(session);

            // 다음 세션 상태로 전환합니다.
            let control_flow = SessionStateFlow::Pop;
            let event = SessionEvents::SetControlFlow(control_flow);
            session.push_event(event);
        } else {
            log::warn!("{} accesses an invalid custom game", session);
            session.close();
            return;
        }
    }

    /// `CustomGameReadyPacket`을 처리합니다.
    fn handle_custom_game_ready_packet(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet = match CustomGameReadyPacket::try_from_raw(packet) {
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
        if let Some(world) = self.world.upgrade() {
            // 게임 준비 요청을 보냅니다.
            let event = GameWorldEvent::CustomRoomReady {
                session: session.clone(),
                uid: packet.user_id,
                ready: packet.ready,
            };
            world.push_event(event);
        } else {
            log::warn!("{} accesses an invalid custom game", session);
            session.close();
            return;
        }
    }
}

impl SessionState for SessionRoomState {
    fn on_resume(&mut self, _session: &Arc<Session>) {
        self.is_running = true;
    }

    fn handle_event(&mut self, event: SessionEvents, session: &Arc<Session>) {
        match event {
            SessionEvents::EnterFormation => {
                // 다음 세션 상태로 전환합니다.
                self.is_running = false;
                let next_state =
                    Box::new(SessionFormationState::new(self.account, self.world.clone()));
                let control_flow = SessionStateFlow::Push(next_state);
                let event = SessionEvents::SetControlFlow(control_flow);
                session.push_event(event);
            }
            _ => {
                log::warn!("ignored >> unused session event (STATE:{:?})", &self);
            }
        }
    }

    fn handle_packets(&mut self, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            // 세션 상태가 실행 중이 아닌 경우 스킵합니다.
            if !self.is_running {
                continue;
            }

            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::CustomGameLeave => {
                    self.handle_custom_game_leave_packet(session, packet);
                }
                PacketType::CustomGameReady => {
                    self.handle_custom_game_ready_packet(session, packet);
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

    fn on_exit(&mut self, session: &Arc<Session>) {
        // 커스텀 게임 대기실에서 플레이어를 제거합니다.
        if let Some(world) = self.world.upgrade() {
            world.exit(session);
        }
    }
}

impl fmt::Debug for SessionRoomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionRoomState))
    }
}
