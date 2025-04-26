use std::{fmt, sync::Arc};

use ahash::HashMap;
use mod_network::{
    components::{LatLon, PlayPhasePlayer, StageKind, Team, UserId},
    protocol::{InitStagePacket, Packet},
};

use crate::{
    session::{Session, SessionEvents},
    world::{GameWorld, GameWorldEvent},
};

use super::{GameWorldState, GameWorldStateFlow, in_game::GameWorldInGameState};

pub struct GameWorldInGameSyncState {
    /// 게임 월드 상태 실행 여부
    is_running: bool,

    /// 게임 스테이지 종류
    stage_kind: StageKind,

    /// 플레이어 스폰 위치 저장
    spawn_positions: HashMap<UserId, (glam::Vec3A, glam::Quat, LatLon)>,
}

impl GameWorldInGameSyncState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(stage_kind: StageKind) -> Self {
        Self {
            is_running: true,
            stage_kind,
            spawn_positions: HashMap::default(),
        }
    }

    /// 게임 로드 완료 이벤트를 처리합니다.
    fn handle_game_load_finish_event(&self, session: &Session, uid: UserId, world: &GameWorld) {
        // 플레이어 캐릭터의 부울 플래그를 `true`로 변경합니다.
        if let Some(mut player) = world.players.get_mut(&uid) {
            player.with_bool_flag(true);
        } else {
            log::warn!("{} accesses an invalid game player", session);
            session.close();
        }
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &GameWorld) {
        // 락을 획득합니다.
        let num_players = world.num_players.lock();

        // 모든 플레이어가 준비완료 되었는지 확인합니다.
        let mut all_player_loaded = true;
        for player in world.players.iter() {
            all_player_loaded &= player.bool_flag();
        }

        // 모든 플레이어가 준비된 경우 다음 게임 월드 상태로 전환합니다.
        if all_player_loaded {
            self.is_running = false;

            let next_state = Box::new(GameWorldInGameState::new(
                self.stage_kind,
                self.spawn_positions.clone(),
                (5 * 60) as f32, // 5분
            ));
            let control_flow = GameWorldStateFlow::Change(next_state);
            let event = GameWorldEvent::SetControlFlow(control_flow);
            world.push_event(event);

            for session in world.sessions.iter() {
                session.key().push_event(SessionEvents::EnterInGame);
            }
        }

        drop(num_players);
    }
}

impl GameWorldState for GameWorldInGameSyncState {
    fn on_enter(&mut self, world: &Arc<GameWorld>) {
        let mut red_team_spawn_pos = vec![
            (4, glam::vec3a(-27.0, 0.0, -33.0)),
            (3, glam::vec3a(-33.0, 0.0, -33.0)),
            (2, glam::vec3a(-28.5, 0.0, -33.0)),
            (1, glam::vec3a(-31.5, 0.0, -33.0)),
            (0, glam::vec3a(-30.0, 0.0, -33.0)),
        ];
        let mut blue_team_spawn_pos = vec![
            (4, glam::vec3a(27.0, 0.0, 33.0)),
            (3, glam::vec3a(33.0, 0.0, 33.0)),
            (2, glam::vec3a(28.5, 0.0, 33.0)),
            (1, glam::vec3a(31.5, 0.0, 33.0)),
            (0, glam::vec3a(30.0, 0.0, 33.0)),
        ];

        for mut player in world.players.iter_mut() {
            // 모든 플레이어의 부울 플래그를 `false`로 설정합니다.
            player.with_bool_flag(false);

            // 팀에 따라 적절한 스폰 위치에 스폰될 수 있도록 플레이어 위치와 방향을 초기화합니다.
            let team = player.team();
            let user_id = player.account().uid;
            let ((index, position), direction, view_rotation) = match team {
                Team::Red => (
                    red_team_spawn_pos
                        .pop()
                        .unwrap_or((0, glam::vec3a(-30.0, 0.0, -33.0))),
                    glam::quat(0.0, 0.0, 0.0, 1.0),
                    LatLon {
                        lon: 0f32.to_radians(),
                        lat: 10f32.to_radians(),
                    },
                ),
                Team::Blue => (
                    blue_team_spawn_pos
                        .pop()
                        .unwrap_or((0, glam::vec3a(30.0, 0.0, 33.0))),
                    glam::quat(0.0, 1.0, 0.0, 0.0),
                    LatLon {
                        lon: 180f32.to_radians(),
                        lat: 10f32.to_radians(),
                    },
                ),
            };

            self.spawn_positions
                .insert(user_id, (position, direction, view_rotation));
            player
                .with_index(index)
                .with_translation(position)
                .with_rotation(direction)
                .with_view_rotation(view_rotation);
            player.reset_state();
        }

        // 각 세션에 스테이지 초기화 패킷을 전송합니다.
        let packet = InitStagePacket::new(
            self.stage_kind,
            world
                .players
                .iter()
                .map(|player| {
                    PlayPhasePlayer::new(
                        player.account().clone(),
                        player.kill_count(),
                        player.dead_count(),
                        player.assist_count(),
                        player.character_kind(),
                        player.remaining_bullet(),
                        player.health_point(),
                        player.translation().to_array(),
                        player.rotation().to_array(),
                        player.team(),
                        player.index(),
                        player.get_ex_skill_cost(),
                        // player.get_skill_cool_time(),
                        player.action_state(),
                        player.action_state_timer(),
                        player.movement_state(),
                        player.movement_state_timer(),
                        player.view_state(),
                        player.view_state_timer(),
                        player.view_rotation(),
                    )
                })
                .collect(),
        );
        for session in world.sessions.iter() {
            session.key().tcp_write(packet.as_raw());
        }
    }

    fn on_exit(&mut self, world: &Arc<GameWorld>) {
        for mut player in world.players.iter_mut() {
            // 모든 플레이어의 부울 플래그를 `false`로 설정합니다.
            player.with_bool_flag(false);
        }
    }

    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
        match event {
            GameWorldEvent::GameLoadFinish { session, uid } => {
                self.handle_game_load_finish_event(&session, uid, world);
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

        // 다음 게임 상태로 전환을 시도합니다.
        self.try_enter_next_state(world);
    }
}

impl fmt::Debug for GameWorldInGameSyncState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameSyncState))
    }
}
