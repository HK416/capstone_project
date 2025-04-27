use std::{fmt, sync::Arc};

use ahash::HashMap;
use dashmap::mapref::multiple::RefMulti;
use mod_network::{
    components::{
        ActionState, ActionStateTimer, ExSkillCost, GamePlayData, HealthPoint, LatLon,
        MAX_IN_GAME_PLAYERS, MovementState, MovementStateTimer, PlayPhasePlayer, RemainingBullet,
        StageKind, Team, UserId, ViewState, ViewStateTimer,
    },
    protocol::{InitStagePacket, Packet},
};
use tokio::time::Instant;

use crate::{
    data::get_stage_attributes,
    entities::{PlayData, PlayerObject},
    session::{Session, SessionEvents},
    world::{GameWorld, GameWorldEvent},
};

use super::{GameWorldState, GameWorldStateFlow, in_game_prepare::GameWorldInGamePrepareState};

/// 최대 상태 지속 시간입니다.
const MAX_STATE_DURATION: f32 = 60.0;

pub struct GameWorldInGameSyncState {
    /// 게임 월드 상태 실행 여부
    is_running: bool,
    /// 이전 측정 시각
    previous_time_pt: Instant,
    /// 남은 상태 지속 시간
    remaining_time_sec: f32,

    /// 게임 스테이지 종류
    stage_kind: StageKind,

    /// 플레이어 스폰 위치 저장
    spawn_positions: Option<HashMap<UserId, (glam::Vec3A, glam::Quat, LatLon)>>,
    /// 플레이어 게임 플레이 데이터 저장
    play_data: Option<HashMap<UserId, PlayData>>,
}

impl GameWorldInGameSyncState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new<'a, I>(stage_kind: StageKind, iter: I) -> Self
    where
        I: Iterator<Item = RefMulti<'a, UserId, PlayerObject>>,
    {
        Self {
            is_running: true,
            previous_time_pt: Instant::now(),
            remaining_time_sec: MAX_STATE_DURATION,
            stage_kind,
            spawn_positions: Some(HashMap::default()),
            play_data: Some(
                iter.map(|player| {
                    (
                        player.account().uid,
                        PlayData {
                            connected: true,
                            loaded: false,
                            account: player.account().clone(),
                            character_kind: player.character_kind(),
                            team: player.team(),
                            team_index: player.team_index(),
                            kill_count: 0,
                            dead_count: 0,
                            damage_dealt: 0,
                            damage_taken: 0,
                            healing_given: 0,
                        },
                    )
                })
                .collect(),
            ),
        }
    }

    /// 게임 로드 완료 이벤트를 처리합니다.
    fn handle_game_load_finish_event(&mut self, session: &Session, uid: UserId) {
        // Safe: 플레이 데이터가 없는 경우 이벤트를 처리하지 않습니다.
        let play_data = unsafe { self.play_data.as_mut().unwrap_unchecked() };

        // 준비 완료 부울 플래그를 true로 설정합니다.
        if let Some(data) = play_data.get_mut(&uid) {
            data.loaded = true;
        } else {
            log::warn!("{} accesses an invalid game player", session);
            session.close();
        }
    }

    /// 플레이어 떠남 이벤트를 처리합니다.
    fn handle_player_leave_event(&mut self, uid: UserId) {
        // Safe: 플레이 데이터가 없는 경우 이벤트를 처리하지 않습니다.
        let play_data = unsafe { self.play_data.as_mut().unwrap_unchecked() };

        // 연결 상태 부울 플래그를 false로 설정합니다.
        if let Some(data) = play_data.get_mut(&uid) {
            data.connected = false;
        } else {
            log::warn!("unknown game player (UID:{})", uid);
        }
    }

    /// 남은 상태 지속 시간을 갱신합니다.
    fn update_remaining_time(&mut self) {
        let current_time_pt = Instant::now();
        let elapsed_time_sec = current_time_pt
            .saturating_duration_since(self.previous_time_pt)
            .as_secs_f32();
        self.previous_time_pt = current_time_pt;

        self.remaining_time_sec = (self.remaining_time_sec - elapsed_time_sec).max(0.0);
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &GameWorld) {
        // 락을 획득합니다.
        let num_players = world.num_players.lock();

        // Safe: 플레이 데이터가 없는 경우 이벤트를 처리하지 않습니다.
        let play_data = unsafe { self.play_data.as_mut().unwrap_unchecked() };

        // 모든 플레이어가 준비완료 되었는지 확인합니다.
        let mut all_player_loaded = true;
        let mut unloaded_sessions = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for item in world.sessions.iter() {
            if let Some(data) = play_data.get(item.value()) {
                if !data.loaded {
                    all_player_loaded = false;
                    unloaded_sessions.push((item.value().clone(), item.key().clone()));
                }
            }
        }

        // 모든 플레이어가 준비된 경우 다음 게임 월드 상태로 전환합니다.
        if all_player_loaded || self.remaining_time_sec <= 0.0 {
            self.is_running = false;

            // 아직 로드가 완료되지 않은 세션을 제거합니다.
            for (user_id, session) in unloaded_sessions {
                if let Some(data) = play_data.get_mut(&user_id) {
                    data.connected = false;
                    session.close();
                }
            }

            let next_state = GameWorldInGamePrepareState::new(
                self.stage_kind,
                unsafe { self.spawn_positions.take().unwrap_unchecked() },
                unsafe { self.play_data.take().unwrap_unchecked() },
            );
            let control_flow = GameWorldStateFlow::Change(Box::new(next_state));
            let event = GameWorldEvent::SetControlFlow(control_flow);
            world.push_event(event);

            for session in world.sessions.iter() {
                session.key().push_event(SessionEvents::EnterStage);
            }
        }

        drop(num_players);
    }
}

impl GameWorldState for GameWorldInGameSyncState {
    fn on_enter(&mut self, world: &Arc<GameWorld>) {
        // 스테이지 속성 데이터에서 스폰 위치 데이터를 가져옵니다.
        let attributes = get_stage_attributes(self.stage_kind);
        let blue_team_spawn = &attributes.blue_team_spawn;
        let red_team_spawn = &attributes.red_team_spawn;
        let mut blue_team_count = 0;
        let mut red_team_count = 0;

        // Safe: on_enter가 호출될 때 spawn_positions는 반드시 존재합니다.
        let spawn_positions = unsafe { self.spawn_positions.as_mut().unwrap_unchecked() };

        for mut player in world.players.iter_mut() {
            // 팀에 따라 적절한 스폰 위치에 스폰될 수 있도록 플레이어 위치와 방향을 초기화합니다.
            let team = player.team();
            let user_id = player.account().uid;
            let (index, position, direction, view_rotation) = match team {
                Team::Blue => {
                    let index = blue_team_count;
                    let pos = blue_team_spawn.pos[index];
                    let dir = blue_team_spawn.dir;
                    let view_dir = blue_team_spawn.view_dir;
                    blue_team_count += 1;

                    (index, pos, dir, view_dir)
                }
                Team::Red => {
                    let index = red_team_count;
                    let pos = red_team_spawn.pos[index];
                    let dir = red_team_spawn.dir;
                    let view_dir = red_team_spawn.view_dir;
                    red_team_count += 1;

                    (index, pos, dir, view_dir)
                }
            };

            spawn_positions.insert(user_id, (position, direction, view_rotation));
            player.reset_state();
            player
                .with_index(index)
                .with_translation(position)
                .with_rotation(direction)
                .with_view_rotation(view_rotation);
        }

        // 각 세션에 스테이지 초기화 패킷을 전송합니다.

        // Safe: 플레이 데이터가 없는 경우 이벤트를 처리하지 않습니다.
        let play_data = unsafe { self.play_data.as_ref().unwrap_unchecked() };
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (&user_id, data) in play_data.iter() {
            // 게임 월드에서 플레이어를 가져옵니다.
            let player = world.players.get(&user_id);
            players.push(match player {
                Some(player) => PlayPhasePlayer::new(
                    true,
                    player.account().clone(),
                    GamePlayData {
                        kill_count: data.kill_count,
                        dead_count: data.dead_count,
                    },
                    player.character_kind(),
                    player.remaining_bullet(),
                    player.health_point(),
                    player.translation().to_array(),
                    player.rotation().to_array(),
                    player.team(),
                    player.team_index(),
                    player.get_ex_skill_cost(),
                    player.action_state(),
                    player.action_state_timer(),
                    player.movement_state(),
                    player.movement_state_timer(),
                    player.view_state(),
                    player.view_state_timer(),
                    player.view_rotation(),
                ),
                None => PlayPhasePlayer::new(
                    false,
                    data.account,
                    GamePlayData {
                        kill_count: data.kill_count,
                        dead_count: data.dead_count,
                    },
                    data.character_kind,
                    RemainingBullet::default(),
                    HealthPoint::default(),
                    [0.0; 3],
                    [0.0; 4],
                    data.team,
                    data.team_index,
                    ExSkillCost::default(),
                    ActionState::default(),
                    ActionStateTimer::default(),
                    MovementState::default(),
                    MovementStateTimer::default(),
                    ViewState::default(),
                    ViewStateTimer::default(),
                    LatLon::default(),
                ),
            });
        }

        let packet = InitStagePacket::new(self.stage_kind, players);
        for session in world.sessions.iter() {
            session.key().tcp_write(packet.as_raw());
        }
    }

    fn handle_event(&mut self, event: GameWorldEvent, _world: &Arc<GameWorld>) {
        // 게임 월드 상태가 실행 중이 아닌 경우 함수를 빠져나옵니다.
        if !self.is_running {
            return;
        }

        match event {
            GameWorldEvent::GameLoadFinish { session, uid } => {
                self.handle_game_load_finish_event(&session, uid);
            }
            GameWorldEvent::PlayerLeave(uid) => {
                self.handle_player_leave_event(uid);
            }
            _ => {
                log::warn!(
                    "ignored >> unused world event (EVENT:{:?} STATE:{:?})",
                    &event,
                    &self
                );
            }
        }
    }

    fn on_advanced(&mut self, world: &Arc<GameWorld>) {
        // 게임 월드 상태가 실행 중이 아닌 경우 함수를 빠져나옵니다.
        if !self.is_running {
            return;
        }

        self.update_remaining_time();
        self.try_enter_next_state(world);
    }
}

impl fmt::Debug for GameWorldInGameSyncState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameSyncState))
    }
}
