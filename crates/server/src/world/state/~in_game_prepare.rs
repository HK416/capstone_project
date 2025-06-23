use std::{fmt, sync::Arc};

use ahash::HashMap;
use mod_network::{
    components::{
        ActionState, ActionStateTimer, ExSkillCost, GamePlayData, HealthPoint, LatLon,
        MAX_IN_GAME_PLAYERS, MovementState, MovementStateTimer, PlayPhasePlayer, RemainingBullet,
        StageKind, UserId, ViewState, ViewStateTimer,
    },
    protocol::{Packet, PrepareStagePacket},
};
use tokio::time::{Duration, Instant};

use crate::{
    entities::PlayData,
    session::{Session, SessionEvents},
    world::{GameWorld, GameWorldEvent, GameWorldSystemEvent},
};

use super::{GameWorldState, GameWorldStateFlow, in_game::GameWorldInGameState};

/// 게임 월드 상태의 최대 지속시간입니다.
const MAX_STATE_DURATION: f32 = 6.0;

pub struct GameWorldInGamePrepareState {
    /// 게임 월드 상태의 실행 여부
    is_running: bool,
    /// 이전 측정 시각
    previous_time_pt: Instant,
    /// 남은 게임 월드 상태의 지속시간
    remaining_time_sec: f32,

    /// 게임 스테이지 종류
    stage_kind: StageKind,

    /// 플레이어 스폰 위치
    spawn_positions: Option<HashMap<UserId, (glam::Vec3A, glam::Quat, LatLon)>>,
    /// 플레이어 게임 플레이 데이터
    play_data: Option<HashMap<UserId, PlayData>>,
}

impl GameWorldInGamePrepareState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(
        stage_kind: StageKind,
        spawn_positions: HashMap<UserId, (glam::Vec3A, glam::Quat, LatLon)>,
        play_data: HashMap<UserId, PlayData>,
    ) -> Self {
        Self {
            is_running: true,
            previous_time_pt: Instant::now(),
            remaining_time_sec: MAX_STATE_DURATION,
            stage_kind,
            spawn_positions: Some(spawn_positions),
            play_data: Some(play_data),
        }
    }

    /// [`GameWorldSystemEvent::PlayerJoin`] 이벤트를 처리합니다.
    fn handle_player_join_event(&mut self, world: &GameWorld, session: Arc<Session>, uid: UserId) {
        /* TODO */
    }

    /// [`GameWorldSystemEvent::PlayerLeave`] 이벤트를 처리합니다.
    fn handle_player_leave_event(
        &mut self,
        _world: &GameWorld,
        _session: Arc<Session>,
        uid: UserId,
    ) {
        let play_data = self
            .play_data
            .as_mut()
            .expect("the game play data must exist!");

        // 연결 상태 부울 플래그를 false로 설정합니다.
        if let Some(data) = play_data.get_mut(&uid) {
            data.connected = false;
        } else {
            log::warn!("unknown game player (UID:{})", uid);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 처리와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldInGamePrepareState {
    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &GameWorld) {
        // 게임 월드 상태의 지속시간이 남아있는 경우 함수 실행을 생략합니다.
        if self.remaining_time_sec > 0.0 {
            return;
        }

        // 다음 게임 월드 상태로 전환합니다.
        self.is_running = false;
        let spawn_positions = self.spawn_positions.take().unwrap();
        let play_data = self.play_data.take().unwrap();
        let next_state = GameWorldInGameState::new(
            self.stage_kind,
            spawn_positions,
            play_data,
            5.0 * 60.0, // 5분
        );
        let state_flow = GameWorldStateFlow::Change(Box::new(next_state));
        world.push_state_flow(state_flow);

        // 세션 상태를 갱신합니다.
        for item in world.sessions.iter() {
            item.key().push_event(SessionEvents::StartGamePlay);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldInGamePrepareState {
    /// 게임 월드 상태를 갱신합니다.
    fn on_update(&mut self, world: &GameWorld) {
        // 경과 시간을 측정합니다.
        let current_time_pt = Instant::now();
        let elapsed_time_sec = current_time_pt
            .saturating_duration_since(self.previous_time_pt)
            .as_secs_f32();
        self.previous_time_pt = current_time_pt;

        self.update_remaining_time_sec(elapsed_time_sec);
        self.update_player_state_timer(world, elapsed_time_sec);
    }

    /// 남은 게임 월드 상태 지속 시간을 갱신합니다.
    fn update_remaining_time_sec(&mut self, elapsed_time_sec: f32) {
        self.remaining_time_sec = (self.remaining_time_sec - elapsed_time_sec).max(0.0);
    }

    /// 플레이어 상태 타이머를 갱신합니다.
    fn update_player_state_timer(&self, world: &GameWorld, elapsed_time_sec: f32) {
        for mut player in world.players.iter_mut() {
            player.update_state_timer(world, elapsed_time_sec);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 패킷 전송과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldInGamePrepareState {
    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let play_data = self
            .play_data
            .as_ref()
            .expect("the game play data must exist!");

        // 플레이어 데이터를 수집합니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (user_id, data) in play_data.iter() {
            // 게임 월드에서 플레이어 데이터를 가져옵니다.
            let player = world.players.get(user_id);
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

        // 플레이어 데이터가 비어있는 경우 함수 실행을 중단합니다.
        if players.is_empty() {
            return;
        }

        // 패킷을 생성하고 전송합니다.
        let packet = PrepareStagePacket::new(players, self.remaining_time_sec);
        for item in world.sessions.iter() {
            item.key().tcp_write(packet.as_raw());
        }
    }
}

//--------------------------------------------------------------------------------------------

impl GameWorldState for GameWorldInGamePrepareState {
    fn on_enter(&mut self, _world: &Arc<GameWorld>) {
        self.previous_time_pt = Instant::now();
    }

    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
        // 게임 월드 상태가 실행 중이 아닌 경우 함수를 빠져나옵니다.
        if !self.is_running {
            return;
        }

        match event {
            GameWorldEvent::System {
                session,
                uid,
                event,
            } => match event {
                GameWorldSystemEvent::PlayerJoin => {
                    self.handle_player_join_event(world, session, uid);
                }
                GameWorldSystemEvent::PlayerLeave => {
                    self.handle_player_leave_event(world, session, uid);
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

    fn on_advanced(&mut self, world: &Arc<GameWorld>, elapsed_time_sec: f32) {
        // 게임 월드 상태가 실행 중이 아닌 경우 함수를 빠져나옵니다.
        if !self.is_running {
            return;
        }

        self.on_update(world);
        self.broadcast(world);
        self.try_enter_next_state(world);
    }
}

impl fmt::Debug for GameWorldInGamePrepareState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameSyncState))
    }
}
