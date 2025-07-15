use std::sync::Arc;

use ahash::HashSet;
use mod_network::{
    components::{NetworkState, Permission, Team, UserId},
    protocol::{JoinFailedReason, JoinRoomFailedPacket, Packet},
};
use rand::seq::SliceRandom;
use tokio::time::Duration;

use crate::{
    session::Session,
    world::{GameWorld, GameWorldEvent, GameWorldState, GameWorldSystemEvent},
};

/// 최대 게임 대기 시간 (단위: ms)
const MAX_WAIT_TIME: u16 = 16_000;

pub struct GameWorldInGameFinishState {
    /// 남은 게임 상태 시간
    remaining_time_ms: u16,

    /// 우승 팀 데이터
    winner: Option<Team>,
    /// 총 게임 플레이 시간
    play_time_ms: u32,

    /// 떠난 플레이어 식별자
    leaved_players: HashSet<UserId>,
}

impl GameWorldInGameFinishState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(winner: Option<Team>, play_time_ms: u32, leaved_players: HashSet<UserId>) -> Self {
        Self {
            remaining_time_ms: MAX_WAIT_TIME,
            winner,
            play_time_ms,
            leaved_players,
        }
    }

    /// [`GameWorldSystemEvent::PlayerJoin`] 이벤트를 처리합니다.
    fn handle_player_join_event(&mut self, session: Arc<Session>, _uid: UserId) {
        // 패킷을 전송합니다.
        let reason = JoinFailedReason::InProgress;
        let packet = JoinRoomFailedPacket::new(reason);
        session.tcp_write(packet.as_raw());
    }

    /// [`GameWorldSystemEvent::PlayerLeave`] 이벤트를 처리합니다.
    fn handle_player_leave_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
    ) {
        // 세션을 제거합니다.
        if world.sessions.remove(&session).is_none() {
            log::error!("{} not found in {}!", &session, &world);
            eprintln!("{} not found in {}!", &session, &world);
            session.close();
            return;
        }

        // 게임 월드에 플레이어가 없는 경우 게임 월드를 비활성화합니다.
        if world.sessions.is_empty() {
            world.disabled();
            return;
        }

        // 플레이어 데이터를 가져옵니다.
        // 현재 상태에서 플레이어 데이터를 제거하지 않습니다.
        let data = match world.players.get_mut(&uid) {
            Some(data) => data,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 플레이어 네트워크 상태를 변경합니다.
        data.set_network_state(NetworkState::Critical);

        // 플레이어의 권한을 해제합니다.
        let permission = data.permission();
        data.set_permission(Permission::User);

        // 떠난 플레이어 식별자를 추가합니다.
        self.leaved_players.insert(uid);

        // 제거된 플레이어의 권한이 관리자인 경우
        // 남은 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
        if permission == Permission::Admin {
            let mut remainings: Vec<_> = world.sessions.values().cloned().collect();
            remainings.shuffle(&mut rand::rng());

            if let Some(uid) = remainings.pop() {
                match world.players.get_mut(&uid) {
                    Some(data) => {
                        world.admin = uid;
                        data.set_permission(Permission::Admin);
                    }
                    None => {
                        log::error!("Player({}) not found in {}!", &uid, &world);
                        eprintln!("Player({}) not found in {}!", &uid, &world);
                    }
                }
            }
        }
    }

    /// [`GameWorldSystemEvent::UpdatePing`] 이벤트를 처리합니다.
    fn handle_update_ping_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        state: NetworkState,
    ) {
        // 플레이어 데이터를 가져옵니다.
        let data = match world.players.get_mut(&uid) {
            Some(data) => data,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 네트워크 상태를 설정합니다.
        data.set_network_state(state);
    }
}

impl GameWorldState for GameWorldInGameFinishState {
    fn on_enter(&mut self, world: &mut GameWorld) {}

    fn on_exit(&mut self, world: &mut GameWorld) {}

    fn handle_event(&mut self, world: &mut GameWorld, event: GameWorldEvent) {
        match event {
            GameWorldEvent::System {
                session,
                uid,
                event,
            } => match event {
                GameWorldSystemEvent::PlayerJoin {
                    name,
                    tier,
                    profile_icon,
                } => {
                    self.handle_player_join_event(session, uid);
                }
                GameWorldSystemEvent::PlayerLeave => {
                    self.handle_player_leave_event(world, session, uid);
                }
                GameWorldSystemEvent::UpdatePing(state) => {
                    self.handle_update_ping_event(world, session, uid, state);
                }
            },
            _ => {
                log::warn!(
                    "ignored >> unused world event (EVENT:{:?}, STATE:{:?})",
                    &event,
                    &self
                );
            }
        }
    }

    fn on_advanced(&mut self, world: &mut GameWorld, elapsed: Duration) {
        // 남은 시간을 갱신합니다.
        let elapsed_time_ms = elapsed.as_millis().min(MAX_WAIT_TIME as u128) as u16;
        self.remaining_time_ms = self.remaining_time_ms.saturating_sub(elapsed_time_ms);
    }
}
