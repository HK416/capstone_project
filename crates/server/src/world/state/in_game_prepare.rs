use std::{fmt, sync::Arc};

use ahash::HashMap;
use mod_network::{
    components::{LatLon, StageKind, UserId},
    protocol::{Packet, PrepareStagePacket},
};
use tokio::time::Instant;

use crate::{
    entities::PlayData,
    session::SessionEvents,
    world::{GameWorld, GameWorldEvent},
};

use super::{GameWorldState, GameWorldStateFlow, in_game::GameWorldInGameState};

/// 최대 상태 지속시간 (단위: 초)
const MAX_STATE_DURATION: f32 = 10.0;

/// 게임 시작 전에 대기하는 상태입니다.
pub struct GameWorldInGamePrepareState {
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
        // 상태 지속 시간이 남아있는 경우 함수 실행을 생략합니다.
        if self.remaining_time_sec > 0.0 {
            return;
        }

        // 다음 게임 월드 상태로 전환합니다.
        self.is_running = false;
        let spawn_positions = unsafe { self.spawn_positions.take().unwrap_unchecked() };
        let play_data = unsafe { self.play_data.take().unwrap_unchecked() };
        let next_state =
            GameWorldInGameState::new(self.stage_kind, spawn_positions, play_data, 5.0 * 60.0);
        let control_flow = GameWorldStateFlow::Change(Box::new(next_state));
        let event = GameWorldEvent::SetControlFlow(control_flow);
        world.push_event(event);

        // 세션 상태를 갱신합니다.
        for session in world.sessions.iter() {
            session.key().push_event(SessionEvents::EnterInGame);
        }
    }

    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let elapsed_time_sec = MAX_STATE_DURATION - self.remaining_time_sec;
        let packet = PrepareStagePacket::new(elapsed_time_sec);
        for session in world.sessions.iter() {
            session.key().tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldState for GameWorldInGamePrepareState {
    fn on_enter(&mut self, _world: &Arc<GameWorld>) {
        self.previous_time_pt = Instant::now();
    }

    fn handle_event(&mut self, event: GameWorldEvent, _world: &Arc<GameWorld>) {
        // 게임 월드 상태가 실행 중이 아닌 경우 함수를 빠져나옵니다.
        if !self.is_running {
            return;
        }

        match event {
            GameWorldEvent::PlayerLeave(uid) => {
                self.handle_player_leave_event(uid);
            }
            _ => {
                log::warn!(
                    "ignored >> unused world event (EVENT:{:?}, STATE:{:?})",
                    &event,
                    &self
                )
            }
        }
    }

    fn on_advanced(&mut self, world: &Arc<GameWorld>) {
        // 게임 월드 상태가 실행 중이 아닌 경우 함수를 빠져나옵니다.
        if !self.is_running {
            return;
        }

        self.update_remaining_time();
        self.broadcast(world);
        self.try_enter_next_state(world);
    }
}

impl fmt::Debug for GameWorldInGamePrepareState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGamePrepareState))
    }
}
