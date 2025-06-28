use std::sync::Arc;

use ahash::HashSet;
use mod_network::{
    components::{InGamePlayerPullData, NetworkState, Permission, Team, UserId},
    protocol::{InGamePullPacket, JoinFailedReason, JoinRoomFailedPacket, Packet},
};
use rand::seq::SliceRandom;
use tokio::time::Duration;

use crate::{
    session::Session,
    world::{GameWorld, GameWorldEvent, GameWorldState, GameWorldSystemEvent},
};

/// 최대 게임 진행 시간 (단위: ms)
pub const MAX_GAME_TIME: u32 = 60_000 * 5; // 5분

/// 인게임 상태 게임 월드입니다.
/// 게임을 진행합니다.
pub struct GameWorldInGameRunState {
    /// 남은 게임 진행 시간
    remaining_time_ms: u32,

    /// 패킷을 보낸 후 경과 시간
    elapsed_time_sec: f32,

    /// 블루 팀 플레이어 수
    num_blue_players: usize,
    /// 레드 팀 플레이어 수
    num_red_players: usize,
    /// 떠난 플레이어 식별자입니다.
    leaved_players: HashSet<UserId>,
}

impl GameWorldInGameRunState {
    pub fn new(
        num_blue_players: usize,
        num_red_players: usize,
        leaved_players: HashSet<UserId>,
    ) -> Self {
        Self {
            remaining_time_ms: MAX_GAME_TIME,
            elapsed_time_sec: 0.0,
            num_blue_players,
            num_red_players,
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

        // 플레이어가 속한 팀의 인원 수를 감소시킵니다.
        let team = data.team();
        match team {
            Team::Blue => {
                self.num_blue_players -= 1;
            }
            Team::Red => {
                self.num_red_players -= 1;
            }
        }

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

impl GameWorldState for GameWorldInGameRunState {
    fn on_enter(&mut self, world: &mut GameWorld) {
        let mut players = Vec::with_capacity(world.players.len());
        for (&uid, data) in world.players.iter() {
            let connected = !self.leaved_players.contains(&uid);
            players.push(InGamePlayerPullData::new(
                uid,
                data.kill_count,
                data.dead_count,
                data.guard_health,
                data.current_health,
                data.current_bullet,
                data.current_skill_cost,
                data.translation.to_array(),
                data.rotation.to_array(),
                data.velocity.to_array(),
                connected,
                data.is_invincible(),
                data.permission(),
                true,
                data.network_state(),
                data.player_states,
                data.action_state_timer,
                data.movement_state_timer,
            ));
        }

        if players.is_empty() {
            return;
        }

        let packet = InGamePullPacket::new(self.remaining_time_ms, players);
        for session in world.sessions.keys() {
            session.tcp_write(packet.as_raw());
        }
    }

    fn on_exit(&mut self, world: &mut GameWorld) {
        // 떠난 플레이어 데이터를 정리합니다.
        for uid in self.leaved_players.iter() {
            world.players.remove(uid);
        }
    }

    fn handle_event(&mut self, world: &mut GameWorld, event: GameWorldEvent) {
        match event {
            GameWorldEvent::System {
                session,
                uid,
                event,
            } => match event {
                GameWorldSystemEvent::PlayerJoin { .. } => {
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
        let elapsed_time_ms = elapsed.as_millis().min(MAX_GAME_TIME as u128) as u32;
        // 남은 시간을 갱신합니다.
        self.remaining_time_ms = self.remaining_time_ms.saturating_sub(elapsed_time_ms);
        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed.as_secs_f32();

        // 일전 시각마다 패킷을 전송합니다.
        const TICK: f32 = 1.0 / 60.0;
        if self.elapsed_time_sec >= TICK {
            self.elapsed_time_sec = 0.0;
            // self.broadcast(world);
        }

        // self.try_enter_next_state(world);
    }
}
