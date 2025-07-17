use std::{num::NonZeroU32, sync::Arc};

use ahash::HashSet;
use mod_network::{
    components::{MAX_IN_GAME_PLAYERS, NetworkState, Permission, StageKind, Team, UserId},
    protocol::{
        InGameReadyStatusPacket, JoinFailedReason, JoinRoomFailedPacket, Packet, PlayerReadyStatus,
    },
};
use rand::seq::SliceRandom;
use tokio::time::Duration;

use crate::{
    session::Session,
    world::{
        GameWorld, GameWorldEvent, GameWorldInGameEnterState, GameWorldInGameReadyStateEvent,
        GameWorldState, GameWorldStateFlow, GameWorldSystemEvent,
    },
};

/// 최대 게임 로드 시간 (단위: ms)
pub const MAX_LOAD_TIME: u16 = 45_000;

/// 인게임 상태 게임 월드입니다.
/// 모든 플레이어의 로딩이 완료될 때 까지 대기합니다.
pub struct GameWorldInGameReadyState {
    /// 게임 스테이지 종류
    stage_kind: StageKind,
    /// 커스텀 게임 여부
    custom_game: bool,
    /// 게임 로드 완료까지 남은 시간
    remaining_time_ms: u16,

    /// x축 방향의 게임 월드 절반 크기
    half_size_x: NonZeroU32,
    /// y축 방향의 게임 월드 절반 크기
    half_size_y: NonZeroU32,
    /// z축 방향의 게임 월드 절반 크기
    half_size_z: NonZeroU32,

    /// 패킷을 보낸 후 경과 시간
    elapsed_time_sec: f32,

    /// 블루 팀 플레이어 수
    num_blue_players: usize,
    /// 레드 팀 플레이어 수
    num_red_players: usize,
    /// 떠난 플레이어 식별자입니다.
    leaved_players: HashSet<UserId>,
}

impl GameWorldInGameReadyState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(
        stage_kind: StageKind,
        custom_game: bool,
        half_size_x: NonZeroU32,
        half_size_y: NonZeroU32,
        half_size_z: NonZeroU32,
        num_blue_players: usize,
        num_red_players: usize,
        leaved_players: HashSet<UserId>,
    ) -> Self {
        Self {
            stage_kind,
            custom_game,
            half_size_x,
            half_size_y,
            half_size_z,
            remaining_time_ms: MAX_LOAD_TIME,
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

    /// [`GameWorldInGameReadyStateEvent::ReadyToPlay`] 이벤트를 처리합니다.
    fn handle_ready_to_play_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
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

        // 플레이어를 준비 상태로 전환합니다.
        data.set_ready_to_play(true);
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &mut GameWorld) {
        // 남은 시간이 없는 경우
        if self.remaining_time_ms <= 0 {
            // 준비되지 않은 플레이어의 서버 연결을 해제합니다.
            for (session, &uid) in world.sessions.iter() {
                match world.players.get_mut(&uid) {
                    Some(data) => {
                        if !data.is_ready_to_play() {
                            self.leaved_players.insert(uid);
                            session.close();
                        }
                    }
                    None => {
                        log::error!("player data for the {} not found in {}!", &session, &world);
                        eprintln!("player data for the {} not found in {}!", &session, &world);
                        session.close();
                        continue;
                    }
                };
            }
        }

        // 모든 플레이어가 준비되었는지 확인합니다.
        let all_player_readys: bool = world
            .players
            .iter()
            .filter(|(uid, _data)| !self.leaved_players.contains(&uid))
            .all(|(_uid, data)| data.is_ready_to_play());

        // 플레이어가 없는 경우 함수 실행을 생략합니다.
        if world.sessions.is_empty() {
            return;
        }

        if all_player_readys {
            // 다음 게임 상태로 전환합니다.
            let leaved_players = self.leaved_players.clone();
            self.leaved_players.clear();
            let state = GameWorldInGameEnterState::new(
                self.stage_kind,
                self.custom_game,
                self.half_size_x,
                self.half_size_y,
                self.half_size_z,
                self.num_blue_players,
                self.num_red_players,
                leaved_players,
            );
            let flow = GameWorldStateFlow::Change(Box::new(state));
            world.flows.push(flow);
        }
    }

    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (&uid, data) in world.players.iter() {
            let connected = !self.leaved_players.contains(&uid);
            players.push(PlayerReadyStatus::new(
                uid,
                connected,
                data.network_state(),
                data.is_ready_to_play(),
            ));
        }

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        let packet = InGameReadyStatusPacket::new(self.remaining_time_ms, players);
        for session in world.sessions.keys() {
            session.tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldState for GameWorldInGameReadyState {
    fn on_enter(&mut self, world: &mut GameWorld) {
        // 모든 플레이어의 준비 상태를 `false`로 설정합니다.
        for data in world.players.values_mut() {
            data.set_ready_to_play(false);
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
            GameWorldEvent::FormationState { .. } => { /* empty */ }
            GameWorldEvent::InGameReadyState {
                session,
                uid,
                event,
            } => match event {
                GameWorldInGameReadyStateEvent::ReadyToPlay => {
                    self.handle_ready_to_play_event(world, session, uid);
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
        let elapsed_time_ms = elapsed.as_millis().min(MAX_LOAD_TIME as u128) as u16;
        // 남은 시간을 갱신합니다.
        self.remaining_time_ms = self.remaining_time_ms.saturating_sub(elapsed_time_ms);
        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed.as_secs_f32();

        // 일전 시각마다 패킷을 전송합니다.
        const TICK: f32 = 1.0 / 30.0;
        if self.elapsed_time_sec >= TICK {
            self.elapsed_time_sec = 0.0;
            self.broadcast(world);
        }

        self.try_enter_next_state(world);
    }
}
