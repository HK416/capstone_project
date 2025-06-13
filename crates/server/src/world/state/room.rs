use std::sync::Arc;

use mod_network::{
    components::{CustomRoomPlayerData, Permission, StageKind, Team, UserId},
    protocol::{Packet, RoomDataUpdatePacket, StartFailedReason, StartGameFailedPacket},
};
use rand::seq::SliceRandom;

use crate::{
    session::Session,
    world::{GameWorld, GameWorldEvent, GameWorldRoomStateEvent, GameWorldSystemEvent},
};

use super::GameWorldState;

/// 커스텀 대기실 상태 게임 월드입니다.
pub struct GameWorldRoomState {
    /// 팀 밸런스 옵션
    allow_unbalanced: bool,
    /// 게임 캐릭터 중복 옵션
    allow_duplicates: bool,
    /// 게임 스테이지 종류
    #[allow(dead_code)]
    stage_kind: StageKind,

    /// 블루팀 플레이어 수
    num_blue_players: usize,
    /// 레드팀 플레이어 수
    num_red_players: usize,
    /// 경과 시간
    elapsed_time_sec: f32,
}

impl GameWorldRoomState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new() -> Self {
        Self {
            allow_unbalanced: false,
            allow_duplicates: true,
            stage_kind: StageKind::default(),
            num_blue_players: 0,
            num_red_players: 0,
            elapsed_time_sec: 0.0,
        }
    }

    /// [`GameWorldSystemEvent::PlayerJoin`] 이벤트를 처리합니다.
    fn handle_player_join_event(&mut self, world: &GameWorld, session: Arc<Session>, uid: UserId) {
        // 플레이어 데이터를 가져옵니다.
        let mut player = match world.players.get_mut(&uid) {
            Some(guard) => guard,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 플레이어의 팀을 설정합니다.
        if self.num_red_players < self.num_blue_players {
            player.set_team(Team::Red);
            self.num_red_players += 1;
        } else {
            player.set_team(Team::Blue);
            self.num_blue_players += 1;
        }
    }

    /// [`GameWorldSystemEvent::PlayerLeave`] 이벤트를 처리합니다.
    fn handle_player_leave_event(&mut self, world: &GameWorld, session: Arc<Session>, uid: UserId) {
        // 플레이어 데이터를 제거합니다.
        let player = match world.players.remove(&uid) {
            Some((_, player)) => player,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 제거된 플레이어의 권한이 관리자인 경우
        // 남은 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
        if player.permission() == Permission::Admin {
            let mut remainings: Vec<_> = world
                .sessions
                .iter()
                .map(|guard| guard.value().clone())
                .collect();
            remainings.shuffle(&mut rand::rng());

            if let Some(uid) = remainings.pop() {
                match world.players.get_mut(&uid) {
                    Some(mut player) => {
                        world.set_admin(uid);
                        player.set_permission(Permission::Admin);
                        player.set_ready_to_play(false);
                    }
                    None => {
                        log::error!("Player({}) not found in {}!", &uid, &world);
                        eprintln!("Player({}) not found in {}!", &uid, &world);
                    }
                }
            }
        }
    }

    /// [`GameWorldRoomStateEvent::Ready`] 이벤트를 처리합니다.
    fn handle_ready_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
        ready: bool,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if uid == world.admin() {
            self.try_enter_next_state(world, &session);
        } else {
            // 플레이어 데이터를 가져옵니다.
            let mut player = match world.players.get_mut(&uid) {
                Some(guard) => guard,
                None => {
                    log::error!("Player({}) not found in {}!", &uid, &world);
                    eprintln!("Player({}) not found in {}!", &uid, &world);
                    session.close();
                    return;
                }
            };

            // 플레이어의 준비 상태를 설정합니다.
            player.set_ready_to_play(ready);
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
        // 플레이어 데이터를 가져옵니다.
        let mut player = match world.players.get_mut(&uid) {
            Some(guard) => guard,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 플레이어의 팀을 설정합니다.
        player.set_team(team);
    }

    /// [`GameWorldRoomStateEvent::ChangeDuplicateOption`] 이벤트를 처리합니다.
    fn handle_change_duplicate_option_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
        duplicates: bool,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if uid == world.admin() {
            self.allow_duplicates = duplicates;
        } else {
            log::error!("{} lacks permission in the {}!", &session, &world);
            eprintln!("{} lacks permission in the {}!", &session, &world);
            session.close();
        }
    }

    /// [`GameWorldRoomStateEvent::ChangeBalanceOption`] 이벤트를 처리합니다.
    fn handle_change_balance_option_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
        unbalanced: bool,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if uid == world.admin() {
            self.allow_unbalanced = unbalanced;
        } else {
            log::error!("{} lacks permission in the {}!", &session, &world);
            eprintln!("{} lacks permission in the {}!", &session, &world);
            session.close();
        }
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &GameWorld, session: &Arc<Session>) {
        // 락을 획득합니다. 락은 함수 종료 시점에 해제됩니다.
        // 주의: tokio에서 호출될 경우 스케쥴링 과정에서 데드락이 발생할 수 있습니다.
        let num_players = world.num_players.lock();

        // 인원 수가 부족한 경우
        if *num_players < 2 {
            // 패킷을 생성 후 전송합니다.
            let reason = StartFailedReason::NotEnoughPlayers;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 각 팀에 속한 인원이 1명 이상 존재하는지 확인합니다.
        if self.num_blue_players == 0 {
            let reason = StartFailedReason::EmptyBlueTeam;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        } else if self.num_red_players == 0 {
            let reason = StartFailedReason::EmptyRedTeam;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 팀 밸런스를 확인합니다.
        if !self.allow_unbalanced && self.num_blue_players != self.num_red_players {
            let reason = StartFailedReason::UnbalancedTeams;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 관리자를 제외한 모든 플레이어가 준비가 되었는지 확인합니다.
        let all_player_readys: bool = world
            .players
            .iter()
            .filter(|player| *player.key() != world.admin())
            .all(|player| player.is_ready_to_play());
        if all_player_readys {
            // 게임 월드를 닫습니다.
            world.set_closed(true);

            // TODO!
        } else {
            let reason = StartFailedReason::PlayersNotReady;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
        }

        drop(num_players);
    }

    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        // 플레이어 데이터를 수집합니다.
        let players: Vec<_> = world
            .players
            .iter()
            .map(|player| {
                CustomRoomPlayerData::new(
                    player.key().clone(),
                    player.name,
                    player.profile_icon,
                    player.permission(),
                    player.team(),
                    player.tier(),
                    player.is_ready_to_play(),
                )
            })
            .collect();

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.len() == 0 {
            return;
        }

        // 패킷을 생성합니다.
        let packet = RoomDataUpdatePacket::from_iter(
            world.world_id,
            self.stage_kind,
            self.allow_duplicates,
            self.allow_unbalanced,
            players,
        );

        // 패킷을 각 세션에 전송합니다.
        for guard in world.sessions.iter() {
            let session = guard.key().as_ref();
            session.tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldState for GameWorldRoomState {
    fn on_resume(&mut self, world: &Arc<GameWorld>) {
        world.set_closed(false);

        // 모든 플레이어의 준비 상태를 `false`로 설정합니다.
        for mut player in world.players.iter_mut() {
            player.set_ready_to_play(false);
        }
    }

    fn handle_event(&mut self, world: &Arc<GameWorld>, event: GameWorldEvent) {
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
                GameWorldRoomStateEvent::ChangeUnbalanceOption(unbalanced) => {
                    self.handle_change_balance_option_event(world, session, uid, unbalanced);
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

    fn on_advanced(&mut self, world: &Arc<GameWorld>, elapsed_time_sec: f32) {
        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        // 일정 시각마다 패킷을 전송합니다.
        const TICK: f32 = 1.0 / 30.0;
        if self.elapsed_time_sec >= TICK {
            self.elapsed_time_sec = 0.0;
            self.broadcast(world);
        }
    }
}
