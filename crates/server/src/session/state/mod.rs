//! 클라이언트의 게임 장면에 따라 패킷을 다르게 처리해야합니다.   
//! 따라서 세션 상태에 따라 패킷을 다르게 처리합니다.
//!
//! ## 세션 상태 목록
//! - Verify: 클라이언트가 서버에 연결된 직후의 데이터 무결성을 검사하는 상태입니다.
//! - Login: 클라이언트가 게임 서버에 로그인을 시도하는 상태입니다.
//! - Lobby: 클라이언트가 게임 로비 장면에 있는 상태입니다.
//! - Room: 클라이언트가 커스텀 게임 대기실 장면에 있는 상태입니다.
//! - Formation: 클라이언트가 각 팀의 캐릭터를 편성하는 장면에 있는 상태입니다.
//! - InGameEnter: 클라이언트가 인게임 장면에 진입하고 있는 상태입니다.
//! - InGame: 클라이언트가 인게임 장면에 있는 상태입니다.
//!
// mod finish;
// mod formation;
// mod in_game;
// mod in_game_prepare;
// mod in_game_sync;
mod lobby;
mod login;
// mod room;
mod verify;

use std::{
    collections::VecDeque,
    fmt,
    sync::{Arc, atomic::Ordering as MemOrdering},
};

use mod_network::protocol::{Packet, PacketType, PingPacket, RawPacket};
use tokio::time::{Duration, Instant};

use self::{lobby::*, login::*, verify::*};

use super::Session;

/// 세션 상태가 구현해야하는 기능을 모아놓은 trait입니다.
#[allow(unused_variables)]
pub trait SessionState: fmt::Debug + Send {
    /// 상태에 진입할 때 호출되는 콜백 함수입니다.
    fn on_enter(&mut self, session: &Arc<Session>) {}

    /// 상태에 빠져나올 때 호출되는 콜백 함수입니다.
    fn on_exit(&mut self, session: &Arc<Session>) {}

    /// 상태가 일지정지될 때 호출되는 콜백 함수입니다.
    fn on_pause(&mut self, session: &Arc<Session>) {}

    /// 상태가 재개될 때 호출되는 콜백 함수입니다.
    fn on_resume(&mut self, session: &Arc<Session>) {}

    /// 해당 상태에서 수신된 패킷을 처리합니다.
    fn handle_packets(&mut self, session: &Arc<Session>, packet: RawPacket) {}

    /// 상태를 갱신합니다.
    fn on_advanced(&mut self, session: &Arc<Session>, elapsed_time_sec: f32) {}
}

/// 세션 상태를 제어합니다.
#[derive(Debug)]
#[allow(dead_code)]
pub enum SessionStateFlow {
    /// 현재 세션 상태를 제거하고, 새로운 세션 상태를 추가합니다.
    Change(Box<dyn SessionState>),
    /// 모든 세션 상태를 제거합니다.
    Clear,
    /// 현재 세션 상태를 제거합니다.
    Pop,
    /// 새로운 세션 상태를 추가합니다.
    Push(Box<dyn SessionState>),
    /// 모든 세션 상태를 제거하고, 새로운 세션 상태를 추가합니다.
    Reset(Box<dyn SessionState>),
}

/// 세션 상태를 실행하는 루프 함수입니다.
pub async fn session_state_loop(mut session: Arc<Session>) -> Arc<Session> {
    // tick 초기화
    const TICK: Duration = Duration::from_millis(1);
    let mut interval = tokio::time::interval(TICK);
    let mut previous_time_pt = Instant::now();

    // 상태 스텍 초기화
    let mut states: VecDeque<Box<dyn SessionState>> = VecDeque::with_capacity(8);
    let state = Box::new(SessionVerifyState::new());
    let flow = SessionStateFlow::Reset(state);
    session.flows.push(flow);

    // 핑 측정 초기화
    const MAX_SAMPLES: usize = 50;
    const MAX_PING_TIME: u32 = 250;
    let mut samples = vec![0; MAX_SAMPLES];
    let mut num_samples = 0;
    let mut elapsed_time_ms = 0;
    let mut epoch = 0;
    let mut event = None;

    while session.is_running() {
        let current_time_pt = interval.tick().await;
        let elapsed = current_time_pt.saturating_duration_since(previous_time_pt);
        previous_time_pt = current_time_pt;

        // 핑 측정 경과 시간을 갱신합니다.
        elapsed_time_ms += elapsed.as_millis().min(MAX_PING_TIME as u128) as u32;
        if elapsed_time_ms >= MAX_PING_TIME {
            // 250ms 이후에도 처리되지 않은 핑 측정 이벤트는 제거합니다.
            if event.is_some() {
                // 데이터 추가
                samples.copy_within(1..MAX_SAMPLES, 1);
                samples[0] = 250;
                num_samples = (num_samples + 1).min(MAX_SAMPLES);

                // 평균 핑 시간 계산
                let total: u32 = (0..num_samples).map(|i| samples[i]).sum();
                let ping = (total as f32 / num_samples as f32).round() as u32;
                session.ping.store(ping, MemOrdering::Release);
            }

            // 핑 측정 이벤트를 전송합니다.
            let packet = PingPacket::new(epoch);
            session.tcp_write(packet.as_raw());
            event = Some((epoch, current_time_pt));
            epoch += 1;
        }

        // 현재 세션 상태에 대한 소유권을 가져옵니다.
        if let Some(mut state) = states.pop_back() {
            (state, session, samples, num_samples, event) =
                tokio::task::spawn_blocking(move || {
                    // 현재 세션 상태에서 패킷을 처리합니다.
                    while let Some(packet) = session.received_packets.pop() {
                        // 세션이 종료된 경우 반복문을 탈출합니다.
                        if !session.is_running() {
                            return (state, session, samples, num_samples, event);
                        }

                        // 핑 측정 패킷을 처리합니다.
                        if packet.packet_type() == PacketType::Ping {
                            if let Some((epoch, time_pt)) = event.take() {
                                let packet = match PingPacket::try_from_raw(packet) {
                                    Some(packet) => packet,
                                    None => {
                                        log::error!(
                                            "{} failed to convert packet! (PACKET:{:?})",
                                            &session,
                                            &PacketType::Ping,
                                        );
                                        session.close();
                                        return (state, session, samples, num_samples, event);
                                    }
                                };

                                if packet.send_time == epoch {
                                    let elapsed_time_ms = current_time_pt
                                        .saturating_duration_since(time_pt)
                                        .as_millis()
                                        .min(MAX_PING_TIME as u128)
                                        as u32;

                                    samples.copy_within(1..MAX_SAMPLES, 1);
                                    samples[0] = elapsed_time_ms;
                                    num_samples = (num_samples + 1).min(MAX_SAMPLES);

                                    let total: u32 = (0..num_samples).map(|i| samples[i]).sum();
                                    let ping = (total as f32 / num_samples as f32).round() as u32;
                                    session.ping.store(ping, MemOrdering::Release);
                                } else {
                                    event = Some((epoch, time_pt));
                                }
                            }

                            continue;
                        }

                        // 취소되었거나, 세션 상태가 변경된 경우 패킷 처리를 생략합니다.
                        if session.packet_canceled() || !session.flows.is_empty() {
                            continue;
                        }

                        state.handle_packets(&session, packet);
                    }

                    // 현재 상태를 갱신합니다.
                    state.on_advanced(&session, elapsed.as_secs_f32());

                    (state, session, samples, num_samples, event)
                })
                .await
                .unwrap();

            // 가져온 세션 상태에 대한 소유권을 돌려줍니다.
            states.push_back(state);
        }

        // 세션 상태 흐름을 처리합니다.
        while let Some(flow) = session.flows.pop() {
            handle_session_state_flow(&mut states, &session, flow);
        }
    }

    handle_clear_session_state_flow(&mut states, &session);
    session
}

/// 세션 상태 흐름을 처리합니다.
fn handle_session_state_flow(
    states: &mut VecDeque<Box<dyn SessionState>>,
    session: &Arc<Session>,
    flow: SessionStateFlow,
) {
    match flow {
        SessionStateFlow::Change(new) => handle_change_session_state_flow(states, session, new),
        SessionStateFlow::Clear => handle_clear_session_state_flow(states, session),
        SessionStateFlow::Pop => handle_pop_session_state_flow(states, session),
        SessionStateFlow::Push(new) => handle_push_session_state_flow(states, session, new),
        SessionStateFlow::Reset(new) => handle_reset_session_state_flow(states, session, new),
    }
}

/// [`SessionStateFlow::Clear`]를 처리합니다.
fn handle_clear_session_state_flow(
    states: &mut VecDeque<Box<dyn SessionState>>,
    session: &Arc<Session>,
) {
    while let Some(mut state) = states.pop_back() {
        log::info!("{} exit SessionState({:?})", &session, &state);
        state.on_exit(session);
    }
}

/// [`SessionStateFlow::Change`]를 처리합니다.
fn handle_change_session_state_flow(
    states: &mut VecDeque<Box<dyn SessionState>>,
    session: &Arc<Session>,
    mut new: Box<dyn SessionState>,
) {
    if let Some(mut state) = states.pop_back() {
        log::info!("{} exit SessionState({:?})", &session, &state);
        state.on_exit(session);
    }

    log::info!("{} enter SessionState({:?})", &session, &new);
    new.on_enter(session);
    states.push_back(new);
}

/// [`SessionStateFlow::Push`]를 처리합니다.
fn handle_push_session_state_flow(
    states: &mut VecDeque<Box<dyn SessionState>>,
    session: &Arc<Session>,
    mut new: Box<dyn SessionState>,
) {
    if let Some(state) = states.back_mut() {
        log::info!("{} pause SessionState({:?})", &session, &state);
        state.on_pause(session);
    }

    log::info!("{} enter SessionState({:?})", &session, &new);
    new.on_enter(session);
    states.push_back(new);
}

/// [`SessionStateFlow::Pop`]을 처리합니다.
fn handle_pop_session_state_flow(
    states: &mut VecDeque<Box<dyn SessionState>>,
    session: &Arc<Session>,
) {
    if let Some(mut state) = states.pop_back() {
        log::info!("{} exit SessionState({:?})", &session, &state);
        state.on_exit(session);
    }

    if let Some(state) = states.back_mut() {
        log::info!("{} resume SessionState({:?})", &session, &state);
        state.on_resume(session);
    }
}

/// [`SessionStateFlow::Reset`]을 처리합니다.
fn handle_reset_session_state_flow(
    states: &mut VecDeque<Box<dyn SessionState>>,
    session: &Arc<Session>,
    mut new: Box<dyn SessionState>,
) {
    while let Some(mut state) = states.pop_back() {
        log::info!("{} exit SessionState({:?})", &session, &state);
        state.on_exit(session);
    }

    log::info!("{} enter SessionState({:?})", &session, &new);
    new.on_enter(session);
    states.push_back(new);
}

impl fmt::Debug for SessionVerifyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(VerifyState))
    }
}

impl fmt::Debug for SessionLoginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LoginState))
    }
}

impl fmt::Debug for SessionLobbyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionLobbyState))
    }
}
