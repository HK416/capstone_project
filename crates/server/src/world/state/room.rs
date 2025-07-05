use std::sync::Arc;

use ahash::{HashMap, RandomState};
use mod_network::{
    components::{
        CustomRoomPlayerData, FormationPlayerInitData, GameTier, LatLon, MAX_IN_GAME_PLAYERS,
        MAX_IN_GAME_TEAM_PLAYERS, NetworkState, Permission, ProfileIcon, StageKind, Team, UserId,
        UserName,
    },
    protocol::{
        FormationDataInitPacket, JoinFailedReason, JoinRoomFailedPacket, Packet,
        RoomDataUpdatePacket, StartFailedReason, StartGameFailedPacket,
    },
};
use rand::seq::SliceRandom;
use tokio::time::Duration;

use crate::{
    data::get_stage_attributes,
    entities::Player,
    session::{Session, SessionFormationState, SessionRoomState, SessionStateFlow},
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
    fn handle_player_join_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        name: UserName,
        tier: GameTier,
        profile_icon: ProfileIcon,
    ) {
        // 현재 플레이어 인원을 확인합니다.
        if world.sessions.len() >= MAX_IN_GAME_PLAYERS {
            // 패킷을 전송합니다.
            let reason = JoinFailedReason::FullCapacity;
            let packet = JoinRoomFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 게임 월드가 닫혀있는지 확인합니다.
        if world.closed {
            // 패킷을 전송합니다.
            let reason = JoinFailedReason::InProgress;
            let packet = JoinRoomFailedPacket::new(reason);
            session.tcp_write(packet.as_raw());
            return;
        }

        // 세션의 상태를 변경합니다.
        let state = SessionRoomState::new(uid, world.events.clone());
        let flow = SessionStateFlow::Push(Box::new(state));
        session.add_flow(flow);

        // 세션을 추가합니다.
        world.sessions.insert(session, uid);

        // 플레이어 데이터를 생성합니다.
        let permission = if world.admin == uid {
            Permission::Admin
        } else {
            Permission::User
        };
        let mut player = Player::new(name, profile_icon, permission, tier);

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

        // 플레이어 데이터를 추가합니다.
        world.players.insert(uid, player);
    }

    /// [`GameWorldSystemEvent::PlayerLeave`] 이벤트를 처리합니다.
    fn handle_player_leave_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
    ) {
        // 세션을 제거합니다.
        if world.sessions.remove(&session).is_none() {
            log::error!("{} not found in {}!", &session, &world);
            eprintln!("{} not found in {}!", &session, &world);
            session.close();
            return;
        }

        // 게임 월드에 플레이어가 없는 경우 게임 월드를 비활성화합니다.
        if world.sessions.is_empty() {
            world.disabled();
            return;
        }

        // 플레이어 데이터를 제거합니다.
        let data = match world.players.remove(&uid) {
            Some(data) => data,
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
            let mut remainings: Vec<_> = world.sessions.values().cloned().collect();
            remainings.shuffle(&mut rand::rng());

            if let Some(uid) = remainings.pop() {
                match world.players.get_mut(&uid) {
                    Some(data) => {
                        world.admin = uid;
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

    /// [`GameWorldSystemEvent::UpdatePing`] 이벤트를 처리합니다.
    fn handle_update_ping_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        state: NetworkState,
    ) {
        // 플레이어 데이터를 가져옵니다.
        let data = match world.players.get_mut(&uid) {
            Some(data) => data,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        data.set_network_state(state);
    }

    /// [`GameWorldRoomStateEvent::Ready`] 이벤트를 처리합니다.
    fn handle_ready_event(&mut self, world: &mut GameWorld, session: Arc<Session>, uid: UserId) {
        // 게임 월드 관리자인지 확인합니다.
        if world.admin == uid {
            self.try_enter_next_state(world, &session);
        } else {
            // 플레이어 데이터를 가져옵니다.
            let data = match world.players.get_mut(&uid) {
                Some(data) => data,
                None => {
                    log::error!("Player({}) not found in {}!", &uid, &world);
                    eprintln!("Player({}) not found in {}!", &uid, &world);
                    session.close();
                    return;
                }
            };

            // 플레이어의 준비 상태를 설정합니다.
            data.set_ready_to_play(!data.is_ready_to_play());
        }
    }

    /// [`GameWorldRoomStateEvent::ChangeTeam`] 이벤트를 처리합니다.
    fn handle_change_team_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        target: UserId,
    ) {
        // 게임 월드 관리자 또는 자기 자신이 팀 변경 이벤트를 요청한 것이 아닌 경우
        if !(world.admin == uid || uid == target) {
            log::error!("{} lacks permission in the {}!", &session, &world);
            eprintln!("{} lacks permission in the {}!", &session, &world);
            session.close();
            return;
        }

        // 플레이어 데이터를 가져옵니다.
        let data = match world.players.get_mut(&uid) {
            Some(data) => data,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 커스텀 게임 관리자가 아니고, 플레이어가 준비상태인 경우 해당 이벤트를 무시합니다.
        if world.admin != uid && data.is_ready_to_play() {
            return;
        }

        // 현재 팀 인덱스를 제거합니다.
        let team = data.team();
        match team {
            Team::Blue => {
                self.blue_players.remove(&target);

                let index = self.cnt_red_players;
                self.red_players.insert(target, index);
                self.cnt_red_players += 1;

                data.set_team(Team::Red);
            }
            Team::Red => {
                self.red_players.remove(&target);

                let index = self.cnt_blue_players;
                self.blue_players.insert(target, index);
                self.cnt_blue_players += 1;

                data.set_team(Team::Blue);
            }
        }
    }

    /// [`GameWorldRoomStateEvent::ChangeDuplicateOption`] 이벤트를 처리합니다.
    fn handle_change_duplicate_option_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if world.admin == uid {
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
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if world.admin == uid {
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
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        target: UserId,
    ) {
        // 게임 월드 관리자인지 확인합니다.
        if world.admin == uid {
            if uid == target {
                log::warn!("ignored >> invalid data received!");
                return;
            };

            // 식별자에 해당하는 세션을 찾습니다.
            for (session, &uid) in world.sessions.iter() {
                if uid == target {
                    // 패킷을 전송합니다.
                    let reason = JoinFailedReason::Banned;
                    let packet = JoinRoomFailedPacket::new(reason);
                    session.tcp_write(packet.as_raw());

                    // 세션 상태를 변경합니다.
                    session.add_flow(SessionStateFlow::Pop);
                    return;
                }
            }

            log::info!("Player({}) not found in {}", &target, &world);
            println!("Player({}) not found in {}", &target, &world);
        } else {
            log::error!("{} lacks permission in the {}!", &session, &world);
            eprintln!("{} lacks permission in the {}!", &session, &world);
            session.close();
        }
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &mut GameWorld, session: &Arc<Session>) {
        // 인원 수가 부족한 경우
        if world.sessions.len() < 2 {
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
            .filter(|(uid, _data)| **uid != world.admin)
            .all(|(_uid, data)| data.is_ready_to_play());
        if all_player_readys {
            // 게임 월드를 닫습니다.
            world.closed = true;

            // 스테이지 속성 정보를 가져옵니다.
            let attribute = get_stage_attributes(self.stage_kind);

            let mut num_blue = 0;
            let mut num_red = 0;
            let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
            for (&uid, data) in world.players.iter_mut() {
                // 플레이어 데이터를 설정합니다.
                let (index, translation, roataion, lon) = match data.team() {
                    Team::Blue => {
                        let index = num_blue;
                        num_blue += 1;

                        let translation = attribute.blue_team_positions[index];
                        let rotation = attribute.blue_team_rotation;
                        let lon = rotation.angle_between(glam::Quat::IDENTITY);

                        (index, translation, rotation, lon)
                    }
                    Team::Red => {
                        let index = num_red;
                        num_red += 1;

                        let translation = attribute.red_team_positions[index];
                        let rotation = attribute.red_team_rotation;
                        let lon = rotation.angle_between(glam::Quat::IDENTITY);

                        (index, translation, rotation, lon)
                    }
                };
                data.set_team_index(index);
                data.translation = translation;
                data.rotation = roataion;
                data.latlon = LatLon::new(10f32.to_radians(), lon);

                // 플레이어 초기화 데이터를 생성합니다.
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
            for (session, &uid) in world.sessions.iter() {
                session.tcp_write(packet.as_raw());

                // 다음 세션 상태로 전환합니다.
                let state = SessionFormationState::new(uid, world.events.clone());
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
    }

    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        // 플레이어 데이터를 수집합니다.
        let players: Vec<_> = world
            .players
            .iter()
            .filter_map(|(uid, data)| {
                let index = match data.team() {
                    Team::Blue => self.blue_players.get(&uid).cloned(),
                    Team::Red => self.red_players.get(&uid).cloned(),
                };

                index.map(|index| (index, (uid, data)))
            })
            .map(|(index, (&uid, data))| {
                CustomRoomPlayerData::new(
                    uid,
                    data.name,
                    data.profile_icon,
                    index,
                    data.permission(),
                    data.team(),
                    data.tier(),
                    data.is_ready_to_play(),
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
        for session in world.sessions.keys() {
            session.tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldState for GameWorldRoomState {
    fn on_resume(&mut self, world: &mut GameWorld) {
        let mut blue_players =
            HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new());
        let mut red_players =
            HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new());
        for (&uid, data) in world.players.iter_mut() {
            // 모든 플레이어의 인덱스를 0으로 설정합니다.
            data.set_team_index(0);
            // 모든 플레이어의 준비 상태를 `false`로 설정합니다.
            data.set_ready_to_play(false);
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
        world.closed = false;
    }

    fn handle_event(&mut self, world: &mut GameWorld, event: GameWorldEvent) {
        match event {
            GameWorldEvent::System {
                session,
                uid,
                event,
            } => match event {
                GameWorldSystemEvent::PlayerJoin {
                    name,
                    tier,
                    profile_icon,
                } => {
                    self.handle_player_join_event(world, session, uid, name, tier, profile_icon);
                }
                GameWorldSystemEvent::PlayerLeave => {
                    self.handle_player_leave_event(world, session, uid);
                }
                GameWorldSystemEvent::UpdatePing(state) => {
                    self.handle_update_ping_event(world, session, uid, state);
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

    fn on_advanced(&mut self, world: &mut GameWorld, elapsed: Duration) {
        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed.as_secs_f32();

        // 일정 시각마다 패킷을 전송합니다.
        const TICK: f32 = 1.0 / 30.0;
        if self.elapsed_time_sec >= TICK {
            self.elapsed_time_sec = 0.0;
            self.broadcast(world);
        }
    }
}
