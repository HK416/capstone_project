use std::{fmt, sync::Arc};

use mod_network::{
    components::{RecruitPhasePlayer, StageKind, StartFailedReason},
    protocol::{CustomGamePullPacket, CustomGameStartFailedPacket, Packet},
};
use tokio::time::Instant;

use crate::{
    session::{Session, SessionEvents},
    world::{GameWorld, GameWorldEvent},
};

use super::{GameWorldState, GameWorldStateFlow, formation::GameWorldFormationState};

/// 게임 시작 지연시간입니다.
const DEALY_TIME: f32 = 0.8;

/// 커스텀 대기실 상태 게임 월드입니다.
pub struct GameWorldRoomState {
    /// 게임 월드 상태 실행 여부
    is_running: bool,
    /// 이전 측정 시각
    previous_time_pt: Instant,

    /// 팀 밸런스 옵션
    is_balanced: bool,
    /// 게임 캐릭터 중복 옵션
    allow_duplicates: bool,
    /// 게임 스테이지 종류
    stage_kind: StageKind,

    /// 게임 시작 쿨타임
    delay_time: f32,
}

impl GameWorldRoomState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new() -> Self {
        Self {
            is_running: true,
            previous_time_pt: Instant::now(),
            is_balanced: true,
            allow_duplicates: true,
            stage_kind: StageKind::default(),
            delay_time: 0.0,
        }
    }
}

//--------------------------------------------------------------------------------------------
// 처리와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldRoomState {
    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, session: &Session, world: &GameWorld) {
        // 게임 시작 지연 시간이 남아있는 경우 함수 실행을 생략합니다.
        if self.delay_time > 0.0 {
            return;
        }
        self.delay_time = DEALY_TIME;

        // 락을 획득합니다.
        let num_players = world.num_players.lock();

        // 인원 수가 부족한 경우
        if *num_players < 2 {
            // 패킷을 전송합니다.
            let reason = StartFailedReason::NotEnoughPlayers;
            let packet = CustomGameStartFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 팀 균형이 맞지 않은 경우
        if self.is_balanced && *num_players % 2 != 0 {
            // 패킷을 전송합니다.
            let reason = StartFailedReason::UnbalancedTeams;
            let packet = CustomGameStartFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 게임 관리자를 제외한 전원이 준비되었는지 확인합니다.
        let admin = world.admin();
        let mut other_player_readys = true;
        for player in world.players.iter() {
            // 게임 관리자의 경우 스킵
            if *player.key() == admin {
                continue;
            }

            other_player_readys &= player.bool_flag();
        }

        if other_player_readys {
            // 다음 게임 월드 상태로 전환합니다.
            self.is_running = false;

            let next_state = GameWorldFormationState::new(self.allow_duplicates, self.stage_kind);
            let control_flow = GameWorldStateFlow::Push(Box::new(next_state));
            let event = GameWorldEvent::SetControlFlow(control_flow);
            world.push_event(event);

            // 게임 월드에 참여한 모든 세션에 이벤트를 보냅니다.
            for item in world.sessions.iter() {
                item.key().push_event(SessionEvents::EnterFormation);
            }
        } else {
            // 모든 플레이어가 준비되지 않은 경우
            // 패킷을 전송합니다.
            let reason = StartFailedReason::PlayersNotReady;
            let packet = CustomGameStartFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
        }
    }
}

//--------------------------------------------------------------------------------------------
// 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldRoomState {
    /// 게임 시작 쿨타임을 갱신합니다.
    fn update_cool_time(&mut self) {
        let current_time_pt = Instant::now();
        let elapsed_time_sec = current_time_pt
            .saturating_duration_since(self.previous_time_pt)
            .as_secs_f32();
        self.delay_time = (self.delay_time - elapsed_time_sec).max(0.0);
    }
}

//--------------------------------------------------------------------------------------------
// 패킷 전송과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldRoomState {
    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        // 패킷을 생성합니다.
        let packet = CustomGamePullPacket::new(
            self.allow_duplicates,
            self.stage_kind,
            world
                .players
                .iter()
                .map(|item| {
                    RecruitPhasePlayer::new(
                        item.account().clone(),
                        item.team(),
                        item.bool_flag(),
                        item.permission(),
                    )
                })
                .collect(),
        );

        // 패킷을 각 세션에 전송합니다.
        for session in world.sessions.iter() {
            session.key().tcp_write(packet.as_raw());
        }
    }
}

//--------------------------------------------------------------------------------------------

impl GameWorldState for GameWorldRoomState {
    fn on_pause(&mut self, world: &Arc<GameWorld>) {
        self.is_running = false;
        world.set_closed(true);
    }

    fn on_resume(&mut self, world: &Arc<GameWorld>) {
        self.is_running = true;
        self.previous_time_pt = Instant::now();
        self.delay_time = DEALY_TIME;

        world.set_closed(false);

        // 모든 플레이어의 부울 플래그를 `false`로 설정합니다.
        for mut player in world.players.iter_mut() {
            player.with_bool_flag(false);
        }
    }

    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
        // 게임 월드 상태가 실행 중이 아닌 경우 함수를 빠져나옵니다.
        if !self.is_running {
            return;
        }

        match event {
            GameWorldEvent::CustomRoomReady {
                session,
                uid,
                ready,
            } => {
                if uid == world.admin() {
                    self.try_enter_next_state(&session, world);
                } else {
                    if !world.access_mut(&session, |player| {
                        player.with_bool_flag(ready);
                    }) {
                        log::warn!("{} accesses an invalid game player", session);
                        session.close();
                    }
                }
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

        self.update_cool_time();
        self.broadcast(world);
    }
}

impl fmt::Debug for GameWorldRoomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldRoomState))
    }
}
