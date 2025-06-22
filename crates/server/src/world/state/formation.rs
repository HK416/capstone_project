use std::sync::{
    Arc,
    atomic::{self, Ordering as MemOrdering},
};

use ahash::{HashSet, RandomState};
use mod_network::{
    components::{
        CharacterKind, FormationPlayerUpdateData, InGamePlayerInitData, MAX_IN_GAME_PLAYERS,
        MAX_IN_GAME_TEAM_PLAYERS, NUM_CHARACTERS, NetworkState, Permission, SelectResult,
        StageKind, Team, UserId,
    },
    protocol::{
        CharacterSelectResponsePacket, EnterGameFailedPacket, EnterGameFailedResson,
        FormationDataUpdatePacket, InGameDataInitPacket, Packet,
    },
};
use rand::seq::SliceRandom;

use crate::{
    session::{Session, SessionInGameReadyState, SessionStateFlow},
    world::{
        GameWorld, GameWorldEvent, GameWorldFormationStateEvent, GameWorldInGameReadyState,
        GameWorldState, GameWorldStateFlow, GameWorldSystemEvent,
    },
};

/// 최대 장면 지속 시간(초)
pub const MAX_FORMATION_TIME: f32 = 60.0;

/// 캐릭터 편성 상태 게임 월드입니다.
/// 모든 플레이어의 캐릭터 선택이 완료될 때 까지 대기합니다.
pub struct GameWorldFormationState {
    /// 캐릭터 편성 완료까지 남은 시간
    remaining_time_sec: f32,
    /// 게임 캐릭터 중복 옵션
    allow_duplicates: bool,
    /// 게임 스테이지 종류
    #[allow(dead_code)]
    stage_kind: StageKind,

    /// 패킷을 보낸 후 경과 시간
    elapsed_time_sec: f32,

    /// 블루 팀 플레이어 수
    num_blue_players: usize,
    /// 블루 팀 캐릭터 집합
    blue_characters: HashSet<CharacterKind>,

    /// 레드 팀 플레이어 수
    num_red_players: usize,
    /// 레드 팀 캐릭터 집합
    red_characters: HashSet<CharacterKind>,

    /// 떠난 플레이어 식별자입니다.
    leaved_players: HashSet<UserId>,
}

impl GameWorldFormationState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(
        allow_duplicates: bool,
        stage_kind: StageKind,
        num_blue_players: usize,
        num_red_players: usize,
    ) -> Self {
        Self {
            remaining_time_sec: MAX_FORMATION_TIME,
            allow_duplicates,
            stage_kind,
            elapsed_time_sec: 0.0,
            num_blue_players,
            blue_characters: HashSet::with_capacity_and_hasher(
                MAX_IN_GAME_TEAM_PLAYERS,
                RandomState::new(),
            ),
            num_red_players,
            red_characters: HashSet::with_capacity_and_hasher(
                MAX_IN_GAME_TEAM_PLAYERS,
                RandomState::new(),
            ),
            leaved_players: HashSet::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::new(),
            ),
        }
    }

    /// [`GameWorldSystemEvent::PlayerJoin`] 이벤트를 처리합니다.
    fn handle_player_join_event(&mut self, world: &GameWorld, session: Arc<Session>, _uid: UserId) {
        log::error!("{} attempted unauthorized access in {}", &session, &world,);
        eprintln!("{} attempted unauthorized access in {}", &session, &world,);
        session.close();
    }

    /// [`GameWorldSystemEvent::PlayerLeave`] 이벤트를 처리합니다.
    fn handle_player_leave_event(&mut self, world: &GameWorld, session: Arc<Session>, uid: UserId) {
        // 플레이어 데이터를 가져옵니다.
        // 현재 상태에서 플레이어 데이터를 제거하지 않습니다.
        let mut data = match world.players.get_mut(&uid) {
            Some(data) => data,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 플레이어 네트워크 상태를 변경합니다.
        data.set_network_state(NetworkState::Critical);

        // 플레이어의 권한을 해제합니다.
        let permission = data.permission();
        data.set_permission(Permission::User);

        // 플레이어가 속한 팀의 인원 수를 감소시킵니다.
        let team = data.team();
        match team {
            Team::Blue => {
                self.num_blue_players -= 1;
            }
            Team::Red => {
                self.num_red_players -= 1;
            }
        }

        // 캐릭터 중복을 허용하지 않고, 플레이어가 캐릭터를 선택한 경우
        // 플레이어가 선택한 캐릭터를 해제합니다.
        if !self.allow_duplicates && data.is_ready_to_play() {
            let character_kind = data.character_kind();
            data.set_character_kind(CharacterKind::ArisOriginal);
            data.set_ready_to_play(false);
            match data.team() {
                Team::Blue => self.blue_characters.remove(&character_kind),
                Team::Red => self.red_characters.remove(&character_kind),
            };
        }

        // 떠난 플레이어 식별자를 추가합니다.
        let uid = data.key().clone();
        self.leaved_players.insert(uid);
        drop(data);

        // 제거된 플레이어의 권한이 관리자인 경우
        // 남은 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
        if permission == Permission::Admin {
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
                    }
                    None => {
                        log::error!("Player({}) not found in {}!", &uid, &world);
                        eprintln!("Player({}) not found in {}!", &uid, &world);
                    }
                }
            }
        }
    }

    /// [`GameWorldFormationStateEvent::CharacterSelect`] 이벤트를 처리합니다.
    fn handle_character_select_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
        character_kind: CharacterKind,
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

        // 플레이어 캐릭터와 선택한 캐릭터가 같은 경우 생략합니다.
        if player.is_ready_to_play() && player.character_kind() == character_kind {
            return;
        }

        let result = if self.allow_duplicates {
            // 캐릭터 중복을 허용하는 경우 항상 성공을 전송합니다.
            player.set_character_kind(character_kind);
            player.set_ready_to_play(true);
            SelectResult::Success
        } else {
            // 캐릭터 중복을 허용하지 않는 경우
            // 현재 사용 중인 캐릭터를 해제합니다.
            // 해당 캐릭터가 사용 중인지 판단합니다.
            let available = match player.team() {
                Team::Blue => self.blue_characters.insert(character_kind),
                Team::Red => self.red_characters.insert(character_kind),
            };

            if available {
                // 이미 선택한 캐릭터가 존재하는 경우 선택한 캐릭터를 해제합니다.
                if player.is_ready_to_play() {
                    match player.team() {
                        Team::Blue => self.blue_characters.remove(&player.character_kind()),
                        Team::Red => self.red_characters.remove(&player.character_kind()),
                    };
                }

                // 새로 선택한 캐릭터를 등록합니다.
                player.set_character_kind(character_kind);
                player.set_ready_to_play(true);
                SelectResult::Success
            } else {
                SelectResult::Duplicates
            }
        };

        // 패킷을 전송합니다.
        let packet = CharacterSelectResponsePacket::new(result);
        session.tcp_write(packet.as_raw());
    }

    /// [`GameWorldFormationStateEvent::CharacterRelease`] 이벤트를 처리합니다.
    fn handle_character_release_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
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

        // 플레이어 캐릭터와 선택한 캐릭터가 없는 경우 생략합니다.
        if !player.is_ready_to_play() {
            return;
        }

        if self.allow_duplicates {
            player.set_character_kind(CharacterKind::default());
            player.set_ready_to_play(false);
        } else {
            match player.team() {
                Team::Blue => self.blue_characters.remove(&player.character_kind()),
                Team::Red => self.red_characters.remove(&player.character_kind()),
            };

            player.set_character_kind(CharacterKind::default());
            player.set_ready_to_play(false);
        };
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &Arc<GameWorld>) {
        // 락을 획득합니다. 락은 함수 종료 시점에 해제됩니다.
        // 주의: tokio에서 호출될 경우 스케쥴링 과정에서 데드락이 발생할 수 있습니다.
        let num_players = world.num_players.lock();

        // 각 팀에 속한 인원이 1명 이상 존재하는지 확인합니다.
        if self.num_blue_players == 0 {
            // 패킷을 생성 후 모든 세션에 전송합니다.
            let reason = EnterGameFailedResson::BlueTeamEmpty;
            let packet = EnterGameFailedPacket::new(reason);
            for data in world.sessions.iter() {
                let session = data.key();
                session.tcp_write(packet.as_raw());

                // 이전 세션 상태로 전환합니다.
                session.add_flow(SessionStateFlow::Pop);
            }

            // 이전 월드 상태로 돌아갑니다.
            world.flows.push(super::GameWorldStateFlow::Pop);
            return;
        } else if self.num_red_players == 0 {
            // 패킷을 생성 후 모든 세션에 전송합니다.
            let reason = EnterGameFailedResson::RedTeamEmpty;
            let packet = EnterGameFailedPacket::new(reason);
            for data in world.sessions.iter() {
                let session = data.key();
                session.tcp_write(packet.as_raw());

                // 이전 세션 상태로 전환합니다.
                session.add_flow(SessionStateFlow::Pop);
            }

            // 이전 월드 상태로 돌아갑니다.
            world.flows.push(super::GameWorldStateFlow::Pop);
            return;
        }

        // 남은 시간이 없는 경우
        if self.remaining_time_sec <= 0.0 {
            if self.allow_duplicates {
                // 서버에 연결되어 있고, 캐릭터를 선택하지 않은 플레이어의 캐릭터를 무작위로 지정합니다.
                for mut data in world.players.iter_mut() {
                    let uid = data.key().clone();
                    let leaved = self.leaved_players.contains(&uid);
                    if !leaved && data.is_ready_to_play() {
                        data.set_character_kind(rand::random());
                        data.set_ready_to_play(true);
                    }
                }
            } else {
                // 서버에 연결되어 있고, 캐릭터를 선택하지 않은 플레이어의 캐릭터를 남은 캐릭터에서 무작위로 지정합니다.
                let mut val = 0;
                let mut total =
                    HashSet::with_capacity_and_hasher(NUM_CHARACTERS, RandomState::new());
                while let Some(character_kind) = CharacterKind::new(val) {
                    total.insert(character_kind);
                    val += 1;
                }

                let mut blue_diff: Vec<_> =
                    total.difference(&self.blue_characters).cloned().collect();
                let mut red_diff: Vec<_> =
                    total.difference(&self.red_characters).cloned().collect();
                for mut data in world.players.iter_mut() {
                    let uid = data.key().clone();
                    let leaved = self.leaved_players.contains(&uid);
                    if !leaved && data.is_ready_to_play() {
                        let character_kind = match data.team() {
                            Team::Blue => blue_diff.pop().unwrap_or(CharacterKind::default()),
                            Team::Red => red_diff.pop().unwrap_or(CharacterKind::default()),
                        };
                        data.set_character_kind(character_kind);
                        data.set_ready_to_play(true);
                    }
                }
            }
        }

        // 모든 플레이어가 준비되었는지 확인합니다.
        let all_player_readys: bool = world
            .players
            .iter()
            .filter(|data| {
                let uid = data.key().clone();
                !self.leaved_players.contains(&uid)
            })
            .all(|data| data.is_ready_to_play());
        if all_player_readys {
            // 인게임 초기화 패킷을 생성 후 각 세션에 패킷을 전송합니다.
            let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
            for data in world.players.iter() {
                let uid = data.key().clone();
                let connected = !self.leaved_players.contains(&uid);
                let character_kind = data.character_kind();
                players.push(InGamePlayerInitData::new(
                    uid,
                    data.name,
                    character_kind,
                    data.team(),
                    data.team_index(),
                    data.permission(),
                    connected,
                    data.network_state(),
                    data.maximum_health(),
                    data.maximum_bullet(),
                    data.maximum_skill_cost(),
                    data.translation.to_array(),
                    data.rotation.to_array(),
                    data.latlon,
                ));
            }

            let packet = InGameDataInitPacket::new(self.stage_kind, players);
            for data in world.sessions.iter() {
                let uid = data.value().clone();
                let session = data.key();
                session.tcp_write(packet.as_raw());

                // 다음 세션 상태로 전환합니다.
                let state = SessionInGameReadyState::new(uid, world);
                let flow = SessionStateFlow::Change(Box::new(state));
                session.add_flow(flow);
            }

            // 다음 게임 월드 상태로 전환합니다.
            let leaved_players = self.leaved_players.clone();
            self.leaved_players.clear();
            let state = GameWorldInGameReadyState::new(
                self.num_blue_players,
                self.num_red_players,
                leaved_players,
            );
            let flow = GameWorldStateFlow::Change(Box::new(state));
            world.flows.push(flow);
        }

        drop(num_players);
    }

    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for data in world.players.iter() {
            let uid = data.key().clone();
            let connected = !self.leaved_players.contains(&uid);
            let character_kind = data.is_ready_to_play().then_some(data.character_kind());
            players.push(FormationPlayerUpdateData::new(
                uid,
                connected,
                data.permission(),
                data.network_state(),
                character_kind,
            ));
        }

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        let packet = FormationDataUpdatePacket::new(self.remaining_time_sec, players);
        for data in world.sessions.iter() {
            let session = data.key();
            session.tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldState for GameWorldFormationState {
    fn on_enter(&mut self, world: &Arc<GameWorld>) {
        // 모든 플레이어의 준비 상태를 `false`로 설정합니다.
        for mut player in world.players.iter_mut() {
            player.set_ready_to_play(false);
        }
        atomic::fence(MemOrdering::SeqCst);
    }

    fn on_exit(&mut self, world: &Arc<GameWorld>) {
        // 떠난 플레이어 데이터를 정리합니다.
        for uid in self.leaved_players.iter() {
            world.players.remove(uid);
        }
        atomic::fence(MemOrdering::SeqCst);
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
            GameWorldEvent::RoomState { .. } => { /* empty */ }
            GameWorldEvent::FormationState {
                session,
                uid,
                event,
            } => match event {
                GameWorldFormationStateEvent::CharacterSelect(character_kind) => {
                    self.handle_character_select_event(world, session, uid, character_kind);
                }
                GameWorldFormationStateEvent::CharacterRelease => {
                    self.handle_character_release_event(world, session, uid);
                }
            },
            _ => {
                log::warn!(
                    "ignored >> unused world event (EVENT:{:?}, STATE:{:?})",
                    &event,
                    &self
                );
            }
        }
    }

    fn on_advanced(&mut self, world: &Arc<GameWorld>, elapsed_time_sec: f32) {
        // 남은 시간을 갱신합니다.
        self.remaining_time_sec = (self.remaining_time_sec - elapsed_time_sec).max(0.0);
        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        // 일정 시각마다 패킷을 전송합니다.
        const TICK: f32 = 1.0 / 30.0;
        if self.elapsed_time_sec >= TICK {
            self.elapsed_time_sec = 0.0;
            self.broadcast(world);
        }

        self.try_enter_next_state(world);
    }
}
