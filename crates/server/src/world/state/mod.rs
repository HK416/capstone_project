mod formation;
mod in_game;
mod queued;
mod room;

use std::{collections::VecDeque, fmt};

use tokio::time::Duration;

use crate::world::{get_pool, get_retires};

pub use self::{formation::*, in_game::*, queued::*, room::*};

use super::{GameWorld, GameWorldEvent};

const ALLOW_DUPLICATES: bool = true;

/// 게임 월드 상태가 구현해야 하는 `trait`입니다.
#[allow(unused_variables)]
pub trait GameWorldState: fmt::Debug + Send {
    /// 게임 월드 상태에 진입할 때 호출되는 함수입니다.
    fn on_enter(&mut self, world: &mut GameWorld) {}

    /// 게임 월드 상태에서 빠져나올 때 호출되는 함수입니다.
    fn on_exit(&mut self, world: &mut GameWorld) {}

    /// 게임 월드 상태가 일시정지할 때 호출되는 함수입니다.
    fn on_pause(&mut self, world: &mut GameWorld) {}

    /// 게임 월드 상태가 재개될 때 호출되는 함수입니다.
    fn on_resume(&mut self, world: &mut GameWorld) {}

    /// 게임 월드 이벤트를 처리합니다.
    fn handle_event(&mut self, world: &mut GameWorld, event: GameWorldEvent) {}

    /// 게임 월드 상태를 갱신할 때 호출되는 함수입니다.
    fn on_advanced(&mut self, world: &mut GameWorld, elapsed: Duration) {}
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
pub async fn world_state_loop(mut world: GameWorld, is_custom: bool) {
    // tick 초기화
    const TICK: Duration = Duration::from_millis(10);
    let mut interval = tokio::time::interval(TICK);
    interval.tick().await;

    // 상태 스텍 초기화
    let mut states: VecDeque<Box<dyn GameWorldState>> = VecDeque::with_capacity(8);
    let state: Box<dyn GameWorldState> = if is_custom {
        Box::new(GameWorldRoomState::new())
    } else {
        Box::new(GameWorldQueuedState::new())
    };
    let flow = GameWorldStateFlow::Reset(state);
    world.flows.push(flow);

    'running: while world.running {
        interval.tick().await;

        // 현재 게임 월드 상태에 대한 소유권을 가져옵니다.
        if let Some(mut state) = states.pop_back() {
            // 현재 게임 월드 상태에서 이벤트를 처리합니다.
            while let Some(event) = world.events.pop() {
                // 게임 월드가 비활성화된 경우 반복문을 탈출합니다.
                if !world.running {
                    states.push_back(state);
                    break 'running;
                }

                // 게임 월드 상태가 변경된 경우 이벤트 처리를 생략합니다
                if !world.flows.is_empty() {
                    break;
                }

                state.handle_event(&mut world, event);
            }

            if world.running && world.flows.is_empty() {
                // 현재 상태를 갱신합니다.
                state.on_advanced(&mut world, TICK);
            }

            // 가져온 게임 월드 상태에 대한 소유권을 돌려줍니다.
            states.push_back(state);
        }

        // 게임 월드 상태 흐름을 처리합니다.
        while let Some(flow) = world.flows.pop() {
            handle_world_state_flow(&mut states, &mut world, flow);
        }

        // 게임 월드 상태가 비어있는 경우 루프를 탈출합니다.
        if states.is_empty() {
            break 'running;
        }
    }

    if !is_custom {
        world.disabled();
    }
    log::info!("{} disabled.", &world);
    println!("{} disabled.", &world);

    handle_clear_world_state_flow(&mut states, &mut world);
    get_pool().remove(&world.id);
    get_retires().push(world);
}

/// 세션 상태 흐름을 처리합니다.
fn handle_world_state_flow(
    states: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &mut GameWorld,
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
    world: &mut GameWorld,
) {
    while let Some(mut state) = states.pop_back() {
        log::info!("{} exit GameWorldState({:?})", &world, &state);
        state.on_exit(world);
    }
}

/// [`GameWorldStateFlow::Change`]를 처리합니다.
fn handle_change_session_state_flow(
    states: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &mut GameWorld,
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
    world: &mut GameWorld,
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
    world: &mut GameWorld,
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
    world: &mut GameWorld,
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

impl fmt::Debug for GameWorldQueuedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldQueuedState))
    }
}

impl fmt::Debug for GameWorldFormationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldFormationState))
    }
}

impl fmt::Debug for GameWorldInGameReadyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameReadyState))
    }
}

impl fmt::Debug for GameWorldInGameEnterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameEnterState))
    }
}

impl fmt::Debug for GameWorldInGameRunState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameRunState))
    }
}

impl fmt::Debug for GameWorldInGameFinishState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameFinishState))
    }
}
