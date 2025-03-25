use std::{fmt, sync::Arc};

use mod_network::{
    components::{
        CharacterKind, FormationPhasePlayer, GamePlayStopReason, SelectResult, StageKind, Team,
        UserId,
    },
    protocol::{FormationPullPacket, FormationSelectResponsePacket, GamePlayStopPacket, Packet},
};
use tokio::time::Instant;

use crate::{
    session::{Session, SessionEvents},
    world::{GameWorld, GameWorldEvent},
};

use super::{GameWorldState, GameWorldStateFlow, in_game_sync::GameWorldInGameSyncState};

/// 최대 상태 지속 시간(초)
const MAX_STATE_DURATION: f32 = 60.0;

pub struct GameWorldFormationState {
    /// 게임 월드 상태 실행 여부
    is_running: bool,
    /// 이전 측정 시각
    previous_time_pt: Instant,
    /// 캐릭터 편성완료까지 남은 시간
    remaining_time_sec: f32,

    /// 게임 캐릭터 중복 옵션
    allow_duplicates: bool,
    /// 게임 스테이지 종류
    stage_kind: StageKind,
}

impl GameWorldFormationState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(allow_duplicates: bool, stage_kind: StageKind) -> Self {
        Self {
            is_running: true,
            previous_time_pt: Instant::now(),
            remaining_time_sec: MAX_STATE_DURATION,
            allow_duplicates,
            stage_kind,
        }
    }

    /// 남은 시간을 갱신합니다.
    fn update_remaining_time(&mut self) {
        let current_time_pt = Instant::now();
        let elapsed_time_sec = current_time_pt
            .saturating_duration_since(self.previous_time_pt)
            .as_secs_f32();
        self.previous_time_pt = current_time_pt;
        self.remaining_time_sec = (self.remaining_time_sec - elapsed_time_sec).max(0.0);
    }

    /// 캐릭터 선택 이벤트를 처리합니다.
    fn handle_select_character_event(
        &self,
        session: &Session,
        uid: UserId,
        kind: CharacterKind,
        world: &GameWorld,
    ) {
        if self.allow_duplicates {
            // 캐릭터 중복을 허용하는 경우 플레이어 캐릭터를 선택처리합니다.
            if let Some(mut player) = world.players.get_mut(&uid) {
                player.with_character_kind(kind).with_bool_flag(true);

                // 패킷을 전송합니다.
                let result = SelectResult::Success;
                let packet = FormationSelectResponsePacket::new(result);
                session.tcp_write(packet.as_raw());
            } else {
                log::warn!("{} accesses an invalid game player", session);
                session.close();
            }
        } else {
            // 캐릭터 중복을 허용하지 않는 경우 플레이어 캐릭터가 중복되는지 확인합니다.
            if !self.is_duplicates(world, kind) {
                // 캐릭터가 중복되지 않은 경우 플레이어 캐릭터를 선택 처리합니다.
                if let Some(mut player) = world.players.get_mut(&uid) {
                    player.with_character_kind(kind).with_bool_flag(true);

                    // 패킷을 전송합니다.
                    let result = SelectResult::Success;
                    let packet = FormationSelectResponsePacket::new(result);
                    session.tcp_write(packet.as_raw());
                } else {
                    log::warn!("{} accesses an invalid game player", session);
                    session.close();
                }
            }
        }
    }

    /// 캐릭터가 중복되는지 여부를 반환합니다.
    fn is_duplicates(&self, world: &GameWorld, kind: CharacterKind) -> bool {
        for player in world.players.iter() {
            if player.character_kind() == kind && player.bool_flag() {
                return true;
            }
        }
        return false;
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &GameWorld) {
        // 락을 획득합니다.
        let num_players = world.num_players.lock();

        if self.remaining_time_sec <= 0.0 {
            // 캐릭터 편성 시간이 끝난 경우
            // 캐릭터를 선택하지 않은 플레이어는 무작위로 선택합니다.
            if self.allow_duplicates {
                for mut player in world.players.iter_mut() {
                    if !player.bool_flag() {
                        player
                            .with_character_kind(rand::random())
                            .with_bool_flag(true);
                    }
                }
            } else {
                // TODO: 중복을 허용하지 않는 경우 남은 캐릭터를 무작위로 할당합니다.
                //
            }
        }

        // 인원 수가 부족한 경우
        if *num_players < 2 {
            self.is_running = false;

            // 모든 세션에 게임 플레이 중단 패킷을 전송합니다.
            let reason = GamePlayStopReason::NotEnughPlayers;
            let packet = GamePlayStopPacket::new(reason);
            for session in world.sessions.iter() {
                session.key().tcp_write(packet.as_raw());
                session.key().push_event(SessionEvents::ExitFormation);
            }

            // 게임 월드 상태를 변경합니다.
            let control_flow = GameWorldStateFlow::Pop;
            let event = GameWorldEvent::SetControlFlow(control_flow);
            world.push_event(event);

            return;
        }

        let mut num_red_team = 0;
        let mut num_blue_team = 0;
        let mut all_player_selected = true;
        for player in world.players.iter() {
            all_player_selected &= player.bool_flag();
            if player.team() == Team::Blue {
                num_blue_team += 1;
            } else {
                num_red_team += 1;
            }
        }

        // 한쪽 팀의 인원이 비어있는 경우
        if num_red_team == 0 || num_blue_team == 0 {
            self.is_running = false;

            // 모든 세션에 게임 플레이 중단 패킷을 전송합니다.
            let reason = GamePlayStopReason::OneTeamEmpty;
            let packet = GamePlayStopPacket::new(reason);
            for session in world.sessions.iter() {
                session.key().tcp_write(packet.as_raw());
                session.key().push_event(SessionEvents::ExitFormation);
            }

            // 게임 월드 상태를 변경합니다.
            let control_flow = GameWorldStateFlow::Pop;
            let event = GameWorldEvent::SetControlFlow(control_flow);
            world.push_event(event);

            return;
        }

        // 모든 플레이어가 준비된 경우 다음 게임 월드 상태로 전환합니다.
        if all_player_selected {
            self.is_running = false;

            let next_state = Box::new(GameWorldInGameSyncState::new(self.stage_kind));
            let control_flow = GameWorldStateFlow::Change(next_state);
            let event = GameWorldEvent::SetControlFlow(control_flow);
            world.push_event(event);

            for session in world.sessions.iter() {
                session.key().push_event(SessionEvents::EnterInGameSync);
            }
        }

        drop(num_players);
    }
}

impl GameWorldState for GameWorldFormationState {
    fn on_enter(&mut self, world: &Arc<GameWorld>) {
        // 게임 월드에 포함된 모든 플레이어의 부울 플래그를 `false`로 설정합니다.
        for mut item in world.players.iter_mut() {
            item.with_bool_flag(false);
        }
    }

    fn on_exit(&mut self, world: &Arc<GameWorld>) {
        // 게임 월드에 포함된 모든 플레이어의 부울 플래그를 `false`로 설정합니다.
        for mut item in world.players.iter_mut() {
            item.with_bool_flag(false);
        }
    }

    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
        match event {
            GameWorldEvent::SelectCharacter { session, uid, kind } => {
                self.handle_select_character_event(&session, uid, kind, world);
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

        // 남은 시간을 갱신합니다.
        self.update_remaining_time();

        // 다음 게임 상태로 전환을 시도합니다.
        self.try_enter_next_state(world);

        // 패킷을 생성합니다.
        let packet = FormationPullPacket::new(
            self.allow_duplicates,
            self.stage_kind,
            self.remaining_time_sec,
            world
                .players
                .iter()
                .map(|item| {
                    FormationPhasePlayer::new(
                        item.account().clone(),
                        item.character_kind(),
                        item.bool_flag(),
                        item.team(),
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

impl fmt::Debug for GameWorldFormationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(CharacterFormationState))
    }
}
