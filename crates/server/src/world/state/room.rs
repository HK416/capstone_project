use std::sync::{
    Arc,
    atomic::{self, Ordering as MemOrdering},
};

use ahash::{HashMap, RandomState};
use mod_network::{
    components::{
        CustomRoomPlayerData, FormationPlayerInitData, MAX_IN_GAME_PLAYERS,
        MAX_IN_GAME_TEAM_PLAYERS, Permission, StageKind, Team, UserId,
    },
    protocol::{
        FormationDataInitPacket, JoinFailedReason, JoinRoomFailedPacket, Packet,
        RoomDataUpdatePacket, StartFailedReason, StartGameFailedPacket,
    },
};
use rand::seq::SliceRandom;

use crate::{
    session::{Session, SessionFormationState, SessionStateFlow},
    world::{
        GameWorld, GameWorldEvent, GameWorldFormationState, GameWorldRoomStateEvent,
        GameWorldStateFlow, GameWorldSystemEvent, MAX_FORMATION_TIME,
    },
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

    /// 블루 팀 플레이어 카운터
    cnt_blue_players: u32,
    /// 블루 팀 플레이어
    blue_players: HashMap<UserId, u32>,

    /// 레드 팀 플레이어 카운터
    cnt_red_players: u32,
    /// 레드 팀 플레이어
    red_players: HashMap<UserId, u32>,

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
            cnt_blue_players: 0,
            blue_players: HashMap::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::new(),
            ),
            cnt_red_players: 0,
            red_players: HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new()),
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
        if self.red_players.len() < self.blue_players.len() {
            let index = self.cnt_red_players;
            self.red_players.insert(uid, index);
            self.cnt_red_players += 1;

            player.set_team(Team::Red);
        } else {
            let index = self.cnt_blue_players;
            self.blue_players.insert(uid, index);
            self.cnt_blue_players += 1;

            player.set_team(Team::Blue);
        }
    }

    /// [`GameWorldSystemEvent::PlayerLeave`] 이벤트를 처리합니다.
    fn handle_player_leave_event(&mut self, world: &GameWorld, session: Arc<Session>, uid: UserId) {
        // 플레이어 데이터를 제거합니다.
        let data = match world.players.remove(&uid) {
            Some((_, player)) => player,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 플레이어가 속한 팀의 인원 수를 갱신합니다.
        if data.team() == Team::Blue {
            self.blue_players.remove(&uid);
        } else {
            self.red_players.remove(&uid);
        }

        // 제거된 플레이어의 권한이 관리자인 경우
        // 남은 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
        if data.permission() == Permission::Admin {
            let mut remainings: Vec<_> = world
                .sessions
                .iter()
                .map(|data| data.value().clone())
                .collect();
            remainings.shuffle(&mut rand::rng());

            if let Some(uid) = remainings.pop() {
                match world.players.get_mut(&uid) {
                    Some(mut data) => {
                        world.set_admin(uid);
                        data.set_permission(Permission::Admin);
                        data.set_ready_to_play(false);
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
    fn handle_ready_event(&mut self, world: &Arc<GameWorld>, session: Arc<Session>, uid: UserId) {
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
            let ready = !player.is_ready_to_play();
            player.set_ready_to_play(ready);
        }
    }

    /// [`GameWorldRoomStateEvent::ChangeTeam`] 이벤트를 처리합니다.
    fn handle_change_team_event(
        &mut self,
        world: &Arc<GameWorld>,
        session: Arc<Session>,
        uid: UserId,
        target: UserId,
    ) {
        // 게임 월드 관리자 또는 자기 자신이 팀 변경 이벤트를 요청한 것이 아닌 경우
        if !(uid == world.admin() || uid == target) {
            log::error!("{} lacks permission in the {}!", &session, &world);
            eprintln!("{} lacks permission in the {}!", &session, &world);
            session.close();
            return;
        }

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

        // 커스텀 게임 관리자가 아니고, 플레이어가 준비상태인 경우 해당 이벤트를 무시합니다.
        if uid != world.admin() && player.is_ready_to_play() {
            return;
        }

        // 현재 팀 인덱스를 제거합니다.
        let team = player.team();
        match team {
            Team::Blue => {
                self.blue_players.remove(&target);

                let index = self.cnt_red_players;
                self.red_players.insert(target, index);
                self.cnt_red_players += 1;

                player.set_team(Team::Red);
            }
            Team::Red => {
                self.red_players.remove(&target);

                let index = self.cnt_blue_players;
                self.blue_players.insert(target, index);
                self.cnt_blue_players += 1;

                player.set_team(Team::Blue);
            }
        }
    }

    /// [`GameWorldRoomStateEvent::ChangeDuplicateOption`] 이벤트를 처리합니다.
    fn handle_change_duplicate_option_event(
        &mut self,
        world: &Arc<GameWorld>,
        session: Arc<Session>,
        uid: UserId,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if uid == world.admin() {
            self.allow_duplicates = !self.allow_duplicates;
        } else {
            log::error!("{} lacks permission in the {}!", &session, &world);
            eprintln!("{} lacks permission in the {}!", &session, &world);
            session.close();
        }
    }

    /// [`GameWorldRoomStateEvent::ChangeUnbalanceOption`] 이벤트를 처리합니다.
    fn handle_change_balance_option_event(
        &mut self,
        world: &Arc<GameWorld>,
        session: Arc<Session>,
        uid: UserId,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if uid == world.admin() {
            self.allow_unbalanced = !self.allow_unbalanced;
        } else {
            log::error!("{} lacks permission in the {}!", &session, &world);
            eprintln!("{} lacks permission in the {}!", &session, &world);
            session.close();
        }
    }

    /// [`GameWorldRoomStateEvent::PlayerBan`] 이벤트를 처리합니다.
    fn handle_player_ban_event(
        &mut self,
        world: &Arc<GameWorld>,
        session: Arc<Session>,
        uid: UserId,
        target: UserId,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if uid == world.admin() {
            if uid == target {
                log::warn!("ignored >> invalid data received!");
                return;
            };

            // 락을 획득합니다.
            let lock = world.num_players.lock();

            // 식별자에 해당하는 세션을 찾습니다.
            for guard in world.sessions.iter() {
                let session = guard.key();
                let uid = guard.value();

                if *uid == target {
                    // 패킷을 전송합니다.
                    let reason = JoinFailedReason::Banned;
                    let packet = JoinRoomFailedPacket::new(reason);
                    session.tcp_write(packet.as_raw());

                    // 세션 상태를 변경합니다.
                    session.add_flow(SessionStateFlow::Pop);
                    return;
                }
            }
            drop(lock);

            log::info!("Player({}) not found in {}", &target, &world);
            println!("Player({}) not found in {}", &target, &world);
        } else {
            log::error!("{} lacks permission in the {}!", &session, &world);
            eprintln!("{} lacks permission in the {}!", &session, &world);
            session.close();
        }
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &Arc<GameWorld>, session: &Arc<Session>) {
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
        let num_blue_players = self.blue_players.len();
        let num_red_players = self.red_players.len();
        if num_blue_players == 0 {
            let reason = StartFailedReason::EmptyBlueTeam;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        } else if num_red_players == 0 {
            let reason = StartFailedReason::EmptyRedTeam;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 팀 밸런스를 확인합니다.
        if !self.allow_unbalanced && num_blue_players != num_red_players {
            let reason = StartFailedReason::UnbalancedTeams;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 팀 정원을 초과했는지 확인합니다.
        if num_blue_players > MAX_IN_GAME_TEAM_PLAYERS {
            let reason = StartFailedReason::LimitExceededBlueTeam;
            let packet = StartGameFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        } else if num_red_players > MAX_IN_GAME_TEAM_PLAYERS {
            let reason = StartFailedReason::LimitExceededRedTeam;
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

            // 플레이어 인덱스를 설정합니다.
            let mut num_blue = 0;
            let mut num_red = 0;
            let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
            for mut data in world.players.iter_mut() {
                let index = match data.team() {
                    Team::Blue => {
                        let temp = num_blue;
                        num_blue += 1;
                        temp
                    }
                    Team::Red => {
                        let temp = num_red;
                        num_red += 1;
                        temp
                    }
                };
                data.set_team_index(index);

                let uid = data.key().clone();
                players.push(FormationPlayerInitData::new(
                    uid,
                    data.name,
                    data.profile_icon,
                    data.tier(),
                    data.team(),
                    data.team_index(),
                ));
            }

            // 패킷을 생성합니다.
            let packet = FormationDataInitPacket::new(
                MAX_FORMATION_TIME,
                self.stage_kind,
                self.allow_duplicates,
                players,
            );

            // 게임 월드에 참가한 세션에 패킷을 전송하고, 세션의 상태를 변경합니다.
            for guard in world.sessions.iter() {
                let uid = guard.value().clone();
                let session = guard.key();
                session.tcp_write(packet.as_raw());

                let state = SessionFormationState::new(uid, world.clone());
                session.add_flow(SessionStateFlow::Push(Box::new(state)));
            }

            // 게임 월드 상태를 변경합니다.
            let state = GameWorldFormationState::new(
                self.allow_duplicates,
                self.stage_kind,
                self.blue_players.len(),
                self.red_players.len(),
            );
            let flow = GameWorldStateFlow::Push(Box::new(state));
            world.flows.push(flow);
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
            .filter_map(|player| {
                let index = match player.team() {
                    Team::Blue => self.blue_players.get(player.key()).cloned(),
                    Team::Red => self.red_players.get(player.key()).cloned(),
                };

                index.map(|index| (index, player))
            })
            .map(|(index, player)| {
                CustomRoomPlayerData::new(
                    player.key().clone(),
                    player.name,
                    player.profile_icon,
                    index,
                    player.permission(),
                    player.team(),
                    player.tier(),
                    player.is_ready_to_play(),
                )
            })
            .collect();

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        // 패킷을 생성합니다.
        let packet = RoomDataUpdatePacket::from_iter(
            world.id,
            self.stage_kind,
            self.allow_duplicates,
            self.allow_unbalanced,
            players,
        );

        // 패킷을 각 세션에 전송합니다.
        for data in world.sessions.iter() {
            let session = data.key();
            session.tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldState for GameWorldRoomState {
    fn on_resume(&mut self, world: &Arc<GameWorld>) {
        let mut blue_players =
            HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new());
        let mut red_players =
            HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new());
        for mut data in world.players.iter_mut() {
            // 모든 플레이어의 인덱스를 0으로 설정합니다.
            data.set_team_index(0);
            // 모든 플레이어의 준비 상태를 `false`로 설정합니다.
            data.set_ready_to_play(false);
            let uid = data.key().clone();
            match data.team() {
                Team::Blue => {
                    let index = match self.blue_players.remove(&uid) {
                        Some(index) => index,
                        None => {
                            let temp = self.cnt_blue_players;
                            self.cnt_blue_players += 1;
                            temp
                        }
                    };
                    blue_players.insert(uid, index);
                }
                Team::Red => {
                    let index = match self.red_players.remove(&uid) {
                        Some(index) => index,
                        None => {
                            let temp = self.cnt_red_players;
                            self.cnt_red_players += 1;
                            temp
                        }
                    };
                    red_players.insert(uid, index);
                }
            }
        }
        self.blue_players = blue_players;
        self.red_players = red_players;

        atomic::fence(MemOrdering::SeqCst);
        world.set_closed(false);
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
                GameWorldRoomStateEvent::Ready => {
                    self.handle_ready_event(world, session, uid);
                }
                GameWorldRoomStateEvent::ChangeTeam(target) => {
                    self.handle_change_team_event(world, session, uid, target);
                }
                GameWorldRoomStateEvent::ChangeDuplicateOption => {
                    self.handle_change_duplicate_option_event(world, session, uid);
                }
                GameWorldRoomStateEvent::ChangeUnbalanceOption => {
                    self.handle_change_balance_option_event(world, session, uid);
                }
                GameWorldRoomStateEvent::PlayerBan(target) => {
                    self.handle_player_ban_event(world, session, uid, target);
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
