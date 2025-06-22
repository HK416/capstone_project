use std::sync::Arc;

use ahash::{HashSet, RandomState};
use mod_network::{
    components::{
        CharacterKind, FormationPlayerUpdateData, MAX_IN_GAME_PLAYERS, MAX_IN_GAME_TEAM_PLAYERS,
        Permission, SelectResult, StageKind, Team, UserId,
    },
    protocol::{CharacterSelectResponsePacket, FormationDataUpdatePacket, Packet},
};
use rand::seq::SliceRandom;

use crate::{
    session::Session,
    world::{
        GameWorld, GameWorldEvent, GameWorldFormationStateEvent, GameWorldState,
        GameWorldSystemEvent,
    },
};

/// 최대 장면 지속 시간(초)
pub const MAX_FORMATION_TIME: f32 = 60.0;

pub struct GameWorldFormationState {
    /// 캐릭터 편성 완료까지 남은 시간
    remaining_time_sec: f32,
    /// 게임 캐릭터 중복 옵션
    allow_duplicates: bool,
    /// 게임 스테이지 종류
    #[allow(dead_code)]
    stage_kind: StageKind,

    /// 경과 시간
    elapsed_time_sec: f32,

    /// 블루 팀 캐릭터 집합
    blue_characters: HashSet<CharacterKind>,
    /// 레드 팀 캐릭터 집합
    red_characters: HashSet<CharacterKind>,

    /// 떠난 플레이어 식별자입니다.
    leaved_players: HashSet<UserId>,
}

impl GameWorldFormationState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(allow_duplicates: bool, stage_kind: StageKind) -> Self {
        Self {
            remaining_time_sec: MAX_FORMATION_TIME,
            allow_duplicates,
            stage_kind,
            elapsed_time_sec: 0.0,
            blue_characters: HashSet::with_capacity_and_hasher(
                MAX_IN_GAME_TEAM_PLAYERS,
                RandomState::new(),
            ),
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

        // 플레이어의 권한을 해제합니다.
        let permission = data.permission();
        data.set_permission(Permission::User);

        // 캐릭터 중복을 허용하지 않고, 플레이어가 캐릭터를 선택한 경우
        // 플레이어가 선택한 캐릭터를 해제합니다.
        if !self.allow_duplicates && data.is_ready_to_play() {
            let character_kind = data.character_kind;
            data.character_kind = CharacterKind::ArisOriginal;
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
        if player.is_ready_to_play() && player.character_kind == character_kind {
            return;
        }

        let result = if self.allow_duplicates {
            // 캐릭터 중복을 허용하는 경우 항상 성공을 전송합니다.
            player.character_kind = character_kind;
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
                        Team::Blue => self.blue_characters.remove(&player.character_kind),
                        Team::Red => self.red_characters.remove(&player.character_kind),
                    };
                }

                // 새로 선택한 캐릭터를 등록합니다.
                player.character_kind = character_kind;
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
            player.character_kind = CharacterKind::default();
            player.set_ready_to_play(false);
        } else {
            match player.team() {
                Team::Blue => self.blue_characters.remove(&player.character_kind),
                Team::Red => self.red_characters.remove(&player.character_kind),
            };

            player.character_kind = CharacterKind::default();
            player.set_ready_to_play(false);
        };
    }

    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for data in world.players.iter() {
            let uid = data.key().clone();
            let connected = !self.leaved_players.contains(&uid);
            let character_kind = data.is_ready_to_play().then_some(data.character_kind);
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
    }
}
