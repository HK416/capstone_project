// mod formation;
// mod in_game;
// mod in_game_prepare;
// mod in_game_sync;
mod room;

use std::{collections::VecDeque, fmt, sync::Arc};

use tokio::time::{Duration, Instant};

use crate::world::get_retires;

pub use self::room::*;

use super::{GameWorld, GameWorldEvent};

/// 게임 월드 상태가 구현해야 하는 `trait`입니다.
#[allow(unused_variables)]
pub trait GameWorldState: fmt::Debug + Send {
    /// 게임 월드 상태에 진입할 때 호출되는 함수입니다.
    fn on_enter(&mut self, world: &Arc<GameWorld>) {}

    /// 게임 월드 상태에서 빠져나올 때 호출되는 함수입니다.
    fn on_exit(&mut self, world: &Arc<GameWorld>) {}

    /// 게임 월드 상태가 일시정지할 때 호출되는 함수입니다.
    fn on_pause(&mut self, world: &Arc<GameWorld>) {}

    /// 게임 월드 상태가 재개될 때 호출되는 함수입니다.
    fn on_resume(&mut self, world: &Arc<GameWorld>) {}

    /// 게임 월드 이벤트를 처리합니다.
    fn handle_event(&mut self, world: &Arc<GameWorld>, event: GameWorldEvent) {}

    /// 게임 월드 상태를 갱신할 때 호출되는 함수입니다.
    fn on_advanced(&mut self, world: &Arc<GameWorld>, elapsed_time_sec: f32) {}
}

/// 게임 월드의 상태를 제어합니다.
#[allow(dead_code)]
#[derive(Debug)]
pub enum GameWorldStateFlow {
    Clear,
    Change(Box<dyn GameWorldState>),
    Push(Box<dyn GameWorldState>),
    Pop,
    Reset(Box<dyn GameWorldState>),
}

/// 게임 월드 상태를 실행하는 루프 함수입니다.
pub async fn world_state_loop(mut world: Arc<GameWorld>) {
    // tick 초기화
    const TICK: Duration = Duration::from_millis(1);
    let mut interval = tokio::time::interval(TICK);
    let mut previous_time_pt = Instant::now();

    // 상태 스텍 초기화
    let mut states: VecDeque<Box<dyn GameWorldState>> = VecDeque::with_capacity(8);
    let state = Box::new(GameWorldRoomState::new());
    let flow = GameWorldStateFlow::Reset(state);
    world.flows.push(flow);

    while world.is_running() {
        let current_time_pt = interval.tick().await;
        let elapsed = current_time_pt.saturating_duration_since(previous_time_pt);
        previous_time_pt = current_time_pt;

        // 현재 게임 월드 상태에 대한 소유권을 가져옵니다.
        if let Some(mut state) = states.pop_back() {
            (state, world) = tokio::task::spawn_blocking(move || {
                // 현재 게임 월드 상태에서 이벤트를 처리합니다.
                while let Some(event) = world.received_events.pop() {
                    // 게임 월드가 비활성화된 경우 반복문을 탈출합니다.
                    if !world.is_running() {
                        return (state, world);
                    }

                    // 게임 월드 상태가 변경된 경우 이벤트 처리를 생략합니다
                    if !world.flows.is_empty() {
                        return (state, world);
                    }

                    state.handle_event(&world, event);
                }

                // 게임 월드가 비활성화 되었거나 게임 월드 상태가 변경된 경우
                // 현재 상태 갱신을 생략합니다.
                if !world.is_running() || !world.flows.is_empty() {
                    return (state, world);
                }

                // 현재 상태를 갱신합니다.
                state.on_advanced(&world, elapsed.as_secs_f32());

                (state, world)
            })
            .await
            .unwrap();

            // 가져온 게임 월드 상태에 대한 소유권을 돌려줍니다.
            states.push_back(state);
        }

        // 게임 월드 상태 흐름을 처리합니다.
        while let Some(flow) = world.flows.pop() {
            handle_world_state_flow(&mut states, &world, flow);
        }

        // 게임 월드 상태가 비어있는 경우 루프를 탈출합니다.
        if states.is_empty() {
            break;
        }
    }

    handle_clear_world_state_flow(&mut states, &world);
    get_retires().push(world);
}

/// 세션 상태 흐름을 처리합니다.
fn handle_world_state_flow(
    states: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
    flow: GameWorldStateFlow,
) {
    match flow {
        GameWorldStateFlow::Change(new) => handle_change_session_state_flow(states, world, new),
        GameWorldStateFlow::Clear => handle_clear_world_state_flow(states, world),
        GameWorldStateFlow::Pop => handle_pop_session_state_flow(states, world),
        GameWorldStateFlow::Push(new) => handle_push_session_state_flow(states, world, new),
        GameWorldStateFlow::Reset(new) => handle_reset_session_state_flow(states, world, new),
    }
}

/// [`GameWorldStateFlow::Clear`]를 처리합니다.
fn handle_clear_world_state_flow(
    states: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
) {
    while let Some(mut state) = states.pop_back() {
        log::info!("{} exit GameWorldState({:?})", &world, &state);
        state.on_exit(world);
    }
}

/// [`GameWorldStateFlow::Change`]를 처리합니다.
fn handle_change_session_state_flow(
    states: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
    mut new: Box<dyn GameWorldState>,
) {
    if let Some(mut state) = states.pop_back() {
        log::info!("{} exit GameWorldState({:?})", &world, &state);
        state.on_exit(world);
    }

    log::info!("{} enter GameWorldState({:?})", &world, &new);
    new.on_enter(world);
    states.push_back(new);
}

/// [`GameWorldStateFlow::Push`]를 처리합니다.
fn handle_push_session_state_flow(
    states: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
    mut new: Box<dyn GameWorldState>,
) {
    if let Some(state) = states.back_mut() {
        log::info!("{} pause GameWorldState({:?})", &world, &state);
        state.on_pause(world);
    }

    log::info!("{} enter GameWorldState({:?})", &world, &new);
    new.on_enter(world);
    states.push_back(new);
}

/// [`GameWorldStateFlow::Pop`]을 처리합니다.
fn handle_pop_session_state_flow(
    states: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
) {
    if let Some(mut state) = states.pop_back() {
        log::info!("{} exit GameWorldState({:?})", &world, &state);
        state.on_exit(world);
    }

    if let Some(state) = states.back_mut() {
        log::info!("{} resume GameWorldState({:?})", &world, &state);
        state.on_resume(world);
    }
}

/// [`GameWorldStateFlow::Reset`]을 처리합니다.
fn handle_reset_session_state_flow(
    states: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
    mut new: Box<dyn GameWorldState>,
) {
    while let Some(mut state) = states.pop_back() {
        log::info!("{} exit GameWorldState({:?})", &world, &state);
        state.on_exit(world);
    }

    log::info!("{} enter GameWorldState({:?})", &world, &new);
    new.on_enter(world);
    states.push_back(new);
}

impl fmt::Debug for GameWorldRoomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldRoomState))
    }
}
