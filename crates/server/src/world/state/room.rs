use std::{fmt, sync::Arc};

use mod_network::{
    components::{RecruitPhasePlayer, StageKind, StartFailedReason, Team, UserId},
    protocol::{CustomGamePullPacket, CustomGameStartFailedPacket, Packet},
};

use crate::{
    session::{Session, SessionEvents},
    world::{GameWorld, GameWorldEvent, GameWorldRoomStateEvent, GameWorldSystemEvent},
};

use super::{GameWorldState, GameWorldStateFlow};

/// 커스텀 대기실 상태 게임 월드입니다.
pub struct GameWorldRoomState {
    /// 팀 밸런스 옵션
    is_balanced: bool,
    /// 게임 캐릭터 중복 옵션
    allow_duplicates: bool,
    /// 게임 스테이지 종류
    stage_kind: StageKind,
}

impl GameWorldRoomState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new() -> Self {
        Self {
            is_balanced: true,
            allow_duplicates: true,
            stage_kind: StageKind::default(),
        }
    }

    /// [`GameWorldSystemEvent::PlayerJoin`] 이벤트를 처리합니다.
    fn handle_player_join_event(&mut self, world: &GameWorld, session: Arc<Session>, uid: UserId) {
        /* TODO */
    }

    /// [`GameWorldSystemEvent::PlayerLeave`] 이벤트를 처리합니다.
    fn handle_player_leave_event(&mut self, world: &GameWorld, session: Arc<Session>, uid: UserId) {
        /* TODO */
    }

    /// [`GameWorldRoomStateEvent::Ready`] 이벤트를 처리합니다.
    fn handle_ready_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
        ready: bool,
    ) {
        if uid == world.admin() {
            self.try_enter_next_state(&session, world);
        } else {
            if !world.access_mut(&session, |data| {
                data.with_ready_to_play(ready);
            }) {
                log::warn!("{} accesses an invalid game player", session);
                session.close();
            }
        }
    }

    /// [`GameWorldRoomStateEvent::ChangeTeam`] 이벤트를 처리합니다.
    fn handle_change_team_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
        team: Team,
    ) {
        /* TODO */
    }

    /// [`GameWorldRoomStateEvent::ChangeDuplicateOption`] 이벤트를 처리합니다.
    fn handle_change_duplicate_option_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
        duplicates: bool,
    ) {
        /* TODO */
    }

    /// [`GameWorldRoomStateEvent::ChangeBalanceOption`] 이벤트를 처리합니다.
    fn handle_change_balance_option_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
        balance: bool,
    ) {
        /* TODO */
    }
}

//--------------------------------------------------------------------------------------------
// 처리와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldRoomState {
    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, session: &Session, world: &GameWorld) {
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

        // 각 팀에 속한 인원과 게임 관리자를 제외한 전원이 준비되었는지 확인합니다.
        let admin = world.admin();
        let mut num_reds = 0;
        let mut num_blues = 0;
        let mut other_player_readys = true;
        for player in world.players.iter() {
            if player.team() == Team::Blue {
                num_blues += 1;
            } else {
                num_reds += 1;
            }

            if *player.key() != admin {
                other_player_readys &= player.is_ready_to_play();
            }
        }

        // 각 팀에 속한 인원이 1명 이상 존재하는지 확인합니다.
        if num_reds == 0 {
            let reason = StartFailedReason::EmptyRedTeam;
            let packet = CustomGameStartFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        } else if num_blues == 0 {
            let reason = StartFailedReason::EmptyBlueTeam;
            let packet = CustomGameStartFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 팀 밸런스를 확인합니다.
        if self.is_balanced && num_blues != num_reds {
            let reason = StartFailedReason::UnbalancedTeams;
            let packet = CustomGameStartFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        if other_player_readys {
            // world.set_closed(true);

            // let next_state = GameWorldFormationState::new(self.allow_duplicates, self.stage_kind);
            // let state_flow = GameWorldStateFlow::Push(Box::new(next_state));
            // world.push_state_flow(state_flow);

            // // 게임 월드에 참여한 모든 세션에 이벤트를 보냅니다.
            // for item in world.sessions.iter() {
            //     item.key().push_event(SessionEvents::EnterFormation);
            // }
        } else {
            let reason = StartFailedReason::PlayersNotReady;
            let packet = CustomGameStartFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
        }
    }
}

//--------------------------------------------------------------------------------------------
// 패킷 전송과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldRoomState {
    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let players: Vec<_> = world
            .players
            .iter()
            .map(|item| {
                RecruitPhasePlayer::new(
                    item.account().clone(),
                    item.team(),
                    item.is_ready_to_play(),
                    item.permission(),
                )
            })
            .collect();

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        // 패킷을 생성합니다.
        let packet = CustomGamePullPacket::new(self.allow_duplicates, self.stage_kind, players);

        // 패킷을 각 세션에 전송합니다.
        for session in world.sessions.iter() {
            session.key().tcp_write(packet.as_raw());
        }
    }
}

//--------------------------------------------------------------------------------------------

impl GameWorldState for GameWorldRoomState {
    fn on_resume(&mut self, world: &Arc<GameWorld>) {
        world.set_closed(false);

        // 모든 플레이어의 부울 플래그를 `false`로 설정합니다.
        for mut player in world.players.iter_mut() {
            player.with_ready_to_play(false);
        }
    }

    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
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
            GameWorldEvent::RoomState {
                session,
                uid,
                event,
            } => match event {
                GameWorldRoomStateEvent::Ready(ready) => {
                    self.handle_ready_event(world, session, uid, ready);
                }
                GameWorldRoomStateEvent::ChangeTeam(team) => {
                    self.handle_change_team_event(world, session, uid, team);
                }
                GameWorldRoomStateEvent::ChangeDuplicateOption(duplicates) => {
                    self.handle_change_duplicate_option_event(world, session, uid, duplicates);
                }
                GameWorldRoomStateEvent::ChangeBalanceOption(balance) => {
                    self.handle_change_balance_option_event(world, session, uid, balance);
                }
            },
            _ => {
                log::warn!(
                    "ignored >> unused world event (EVENT:{:?} STATE:{:?})",
                    &event,
                    &self
                );
            }
        }
    }

    fn on_advanced(&mut self, world: &Arc<GameWorld>, _elapsed_time_sec: f32) {
        self.broadcast(world);
    }
}

impl fmt::Debug for GameWorldRoomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldRoomState))
    }
}
