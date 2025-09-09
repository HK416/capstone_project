//! 클라이언트의 게임 장면에 따라 패킷을 다르게 처리해야합니다.   
//! 따라서 세션 상태에 따라 패킷을 다르게 처리합니다.
//!
//! ## 세션 상태 목록
//! - Verify: 클라이언트가 서버에 연결된 직후의 데이터 무결성을 검사하는 상태입니다.
//! - Login: 클라이언트가 게임 서버에 로그인을 시도하는 상태입니다.
//! - Lobby: 클라이언트가 게임 로비 장면에 있는 상태입니다.
//! - Multiplay: 클라이언트의 멀티플레이를 위한 상태입니다.
//! - Queued: 클라이언트가 일반 게임 대기열에 있는 상태입니다.
//! - Room: 클라이언트가 커스텀 게임 대기실 장면에 있는 상태입니다.
//! - Formation: 클라이언트가 각 팀의 캐릭터를 편성하는 장면에 있는 상태입니다.
//! - InGameEnter: 클라이언트가 인게임 장면에 진입하고 있는 상태입니다.
//! - InGame: 클라이언트가 인게임 장면에 있는 상태입니다.
//!

mod formation;
mod in_game_ready;
mod in_game_run;
mod lobby;
mod login;
mod multiplay;
mod queued;
mod room;
mod verify;

use std::{
    collections::VecDeque, fmt, io::ErrorKind, sync::{atomic::Ordering as MemOrdering, Arc}
};

use mod_network::protocol::{Packet, PacketParser, PacketType, PingTestPacket, RawPacket};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, time::{Duration, Instant}};

pub use self::{
    formation::*, in_game_ready::*, in_game_run::*, lobby::*, login::*, multiplay::*, queued::*,
    room::*, verify::*,
};

use super::Session;

/// 수신된 패킷 데이터 대기열의 최대 용량입니다.
pub const MAX_QUEUE_CAPACITY: usize = 64;

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

    /// 수신된 패킷을 처리합니다.
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

struct PingState {
    samples: VecDeque<u32>,
    recent: Option<(u64, Instant)>,
    epoch: u64,
}

impl PingState {
    const MAX_SAMPLES: usize = 20;
    const MAX_PING_TIME_MS: u16 = 1000;

    fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(Self::MAX_SAMPLES),
            recent: None,
            epoch: 0,
        }
    }

    fn add_sample(&mut self, sample: u32) {
        if self.samples.len() >= Self::MAX_SAMPLES {
            self.samples.pop_back();
        }
        self.samples.push_front(sample);
    }

    /// `samples.len()`이 0인 경우는 고려하지 않습니다.  
    fn average(&self) -> u32 {
        let total: u32 = self.samples.iter().sum();
        let ping = (total as f32 / self.samples.len() as f32).round();
        ping as u32
    }
}

/// 세션 상태를 실행하는 루프 함수입니다.
pub async fn session_state_loop(
    mut stream: TcpStream, 
    session: Arc<Session>
) -> Arc<Session> {
    // tick 초기화
    const TICK: Duration = Duration::from_millis(1);
    let mut interval = tokio::time::interval(TICK);
    interval.tick().await;

    // 상태 스텍 초기화
    let mut states: VecDeque<Box<dyn SessionState>> = VecDeque::with_capacity(8);
    let state = SessionVerifyState::new();
    let flow = SessionStateFlow::Reset(Box::new(state));
    session.flows.push(flow);

    // 핑 측정 초기화
    let mut ping_state = PingState::new();
    let mut elapsed_time_ms = 0;

    let mut read_buf = vec![0; 512];
    let mut packet_parser = PacketParser::new();

    'session_loop: while session.is_running() {
        tokio::select! {
            current_time_pt = interval.tick() => {
                elapsed_time_ms += TICK.as_millis() as u16;

                // 현재 세션 상태에 대한 소유권을 가져옵니다.
                if let Some(state) = states.back_mut() {
                    // 현재 상태를 갱신합니다.
                    state.on_advanced(&session, interval.period().as_secs_f32());
                }

                if elapsed_time_ms >= PingState::MAX_PING_TIME_MS {
                    elapsed_time_ms = 0;
                    send_ping_packet(&session, &mut ping_state, current_time_pt);
                }
            },
            read_result = stream.read(&mut read_buf) => {
                let current_time_pt = Instant::now();
                match read_result {
                    Ok(0) => {
                        log::debug!("{} connection closed.", &session);
                        session.close();
                        break;
                    }
                    Ok(n) => {
                        log::trace!("{} data received (SIZE:{}, BYTES:{:?})", &session, n, &read_buf);
                        if packet_parser.len() >= MAX_QUEUE_CAPACITY {
                            // 앞쪽 패킷들을 무시합니다.
                            log::warn!("the number of received packets exceeded the allowed capacity! clearing received packet data.");
                            packet_parser.clear();
                        }
                        packet_parser.push(&read_buf[..n]);
                    }
                    Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                        log::debug!("{} connection closed.", &session);
                        session.close();
                        break;
                    }
                    Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
                        log::debug!("{} connection closed.", &session);
                        session.close();
                        break;
                    }
                    Err(e) => {
                        log::error!("{} {}", &session, e);
                        session.close();
                        break;
                    }
                }

                // 현재 세션 상태에 대한 소유권을 가져옵니다.
                if let Some(state) = states.back_mut() {
                    while let Some(packet) = packet_parser.pop() {
                        log::debug!("{} packet received (PACKET:{:?})", &session, &packet);
                        
                        // 핑 측정 패킷을 처리합니다.
                        if packet.packet_type() == PacketType::Ping {
                            match PingTestPacket::try_from_raw(packet) {
                                Some(packet) => {
                                    handle_ping_packet(&session, &mut ping_state, packet, current_time_pt);
                                    continue;
                                }
                                None => {
                                    session.close();
                                    break;
                                }
                            };
                        }
                    
                        // 현재 세션 상태에서 패킷을 처리합니다.
                        state.handle_packets(&session, packet);

                        if !session.flows.is_empty() {
                            // 세션 상태가 변경된 경우 즉시 빠져나옵니다.
                            break;
                        }
                    }
                }
            }
            // _ = session.tcp_write_notify.notified() => { },
        }

        // 대기열에서 패킷을 가져온다.
        while let Some(packet) = session.tcp_sender.pop() {
            // 소켓에 데이터를 작성한다.
            let bytes = packet.as_bytes();
            let result = stream.write_all(&bytes).await;
            match result {
                Ok(_) => {
                    log::debug!("{} packet sent (PACKET:{:?})", &session, &packet);
                }
                Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                    log::debug!("{} connection closed.", &session);
                    session.close();
                    break 'session_loop;
                }
                Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
                    log::debug!("{} connection closed.", &session);
                    session.close();
                    break 'session_loop;
                }
                Err(e) => {
                    log::error!("{} {}", &session, e);
                    session.close();
                    break 'session_loop;
                }
            };
        }

        // 세션 상태 흐름을 처리합니다.
        while let Some(flow) = session.flows.pop() {
            handle_session_state_flow(&mut states, &session, flow);
        }

        // tokio::time::sleep(Duration::from_millis(1)).await;

        // let current_time_pt = interval.tick().await;
        // let elapsed = current_time_pt.saturating_duration_since(previous_time_pt);
        // previous_time_pt = current_time_pt;

        // // 핑 측정 경과 시간을 갱신합니다.
        // elapsed_time_ms += elapsed.as_millis() as u16;
        // if elapsed_time_ms >= PingState::MAX_PING_TIME {
        //     elapsed_time_ms = 0;
        //     send_ping_packet(&session, &mut ping_state, current_time_pt);
        // }

        // // 현재 세션 상태에 대한 소유권을 가져옵니다.
        // if let Some(mut state) = states.pop_back() {
        //     // 현재 세션 상태에서 패킷을 처리합니다.
        //     while let Some(packet) = session.received_packets.pop() {
        //         // 세션이 종료된 경우 반복문을 탈출합니다.
        //         if !session.is_running() {
        //             break;
        //         }

        //         // 핑 측정 패킷을 처리합니다.
        //         if packet.packet_type() == PacketType::Ping {
        //             match PingTestPacket::try_from_raw(packet) {
        //                 Some(packet) => {
        //                     handle_ping_packet(&session, &mut ping_state, packet, current_time_pt);
        //                     continue;
        //                 }
        //                 None => {
        //                     session.close();
        //                     break;
        //                 }
        //             };
        //         }

        //         // 패킷이 취소된 경우 처리를 생략합니다.
        //         if session.packet_canceled() {
        //             continue;
        //         }

        //         state.handle_packets(&session, packet);
        //     }

        //     // 현재 상태를 갱신합니다.
        //     state.on_advanced(&session, elapsed.as_secs_f32());

        //     // 가져온 세션 상태에 대한 소유권을 돌려줍니다.
        //     states.push_back(state);
        // }

        // // 세션 상태 흐름을 처리합니다.
        // while let Some(flow) = session.flows.pop() {
        //     handle_session_state_flow(&mut states, &session, flow);
        // }
    }

    handle_clear_session_state_flow(&mut states, &session);
    session
}

fn send_ping_packet(
    session: &Arc<Session>, 
    ping_state: &mut PingState, 
    current_time_pt: Instant
) {
    // MAX_PING_TIME 이후에도 처리되지 않은 핑 측정 이벤트는 제거합니다.
    if ping_state.recent.is_some() {
        update_ping(&session, ping_state, PingState::MAX_PING_TIME_MS as u32);
    }

    // 핑 측정 이벤트를 전송합니다.
    let packet = PingTestPacket::new(ping_state.epoch);
    session.tcp_write(packet.as_raw());
    ping_state.recent = Some((ping_state.epoch, current_time_pt));
    ping_state.epoch += 1;
}

fn handle_ping_packet(
    session: &Arc<Session>, 
    ping_state: &mut PingState, 
    packet: PingTestPacket,
    current_time_pt: Instant
) {
    if let Some((epoch, time_pt)) = ping_state.recent.take() {
        if packet.value == epoch {
            // 경과 시간을 측정합니다.
            let elapsed_time_ms = current_time_pt
                .saturating_duration_since(time_pt)
                .as_millis()
                .min(PingState::MAX_PING_TIME_MS as u128)
                as u32;

            // 핑을 갱신합니다.
            let ping = update_ping(&session, ping_state, elapsed_time_ms);

            log::debug!(
                "{} ping:{}ms (num samples:{})",
                &session,
                &ping,
                &ping_state.samples.len()
            );
        } else {
            log::debug!("{} ping: invalid epoch!", &session);
            ping_state.recent = Some((epoch, time_pt));
        }
    }
}

fn update_ping(session: &Arc<Session>, ping_state: &mut PingState, ping: u32) -> u32 {
    // 핑 측정 샘플에 추가합니다.
    ping_state.add_sample(ping);

    // 평균 핑을 계산합니다.
    let ping = ping_state.average();

    session.ping.store(ping as u16, MemOrdering::Release);

    ping
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

impl fmt::Debug for SessionMultiplayState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionMultiplayState))
    }
}

impl fmt::Debug for SessionQueuedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionQueuedState))
    }
}

impl fmt::Debug for SessionRoomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionRoomState))
    }
}

impl fmt::Debug for SessionFormationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionFormationState))
    }
}

impl fmt::Debug for SessionInGameReadyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionInGameReadyState))
    }
}

impl fmt::Debug for SessionInGameRunState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionInGameRunState))
    }
}
