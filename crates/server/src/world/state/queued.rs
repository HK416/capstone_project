use std::sync::Arc;

use ahash::{HashMap, RandomState};
use mod_network::{
    components::{
        ActionState, ActionStateTimer, BulletData, CharacterKind, CustomRoomPlayerData,
        FormationPlayerInitData, HealthData, HeldInput, InputStateTimer, LatLon,
        MAX_IN_GAME_PLAYERS, MovementState, MovementStateTimer, MovingDirection, NetworkState,
        SkillCostData, StageKind, Team, UserId, Velocity,
    },
    protocol::{FormationDataInitPacket, Packet},
};
use tokio::time::Duration;

use crate::{
    data::get_stage_attributes,
    session::{Session, SessionFormationState, SessionMultiplayState, SessionStateFlow},
    world::{
        GameWorld, GameWorldEvent, GameWorldFormationState, GameWorldStateFlow,
        GameWorldSystemEvent, MAX_FORMATION_TIME, state::ALLOW_DUPLICATES,
    },
};

use super::GameWorldState;

/// 커스텀 대기실 상태 게임 월드입니다.
pub struct GameWorldQueuedState {
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

impl GameWorldQueuedState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new() -> Self {
        Self {
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

        // // 패킷을 생성합니다.
        // let packet = RoomDataUpdatePacket::from_iter(
        //     world.id,
        //     self.stage_kind,
        //     self.allow_duplicates,
        //     self.allow_unbalanced,
        //     players,
        // );

        // // 패킷을 각 세션에 전송합니다.
        // for session in world.sessions.keys() {
        //     session.tcp_write(packet.as_raw());
        // }
    }
}

impl GameWorldState for GameWorldQueuedState {
    fn on_enter(&mut self, world: &mut GameWorld) {
        // 스테이지 속성 정보를 가져옵니다.
        let attribute = get_stage_attributes(self.stage_kind);

        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (&uid, data) in world.players.iter_mut() {
            // 플레이어 데이터를 설정합니다.
            let (index, translation, rotation, lon) = match data.team() {
                Team::Blue => {
                    let index = self.cnt_blue_players;
                    self.cnt_blue_players += 1;
                    self.blue_players.insert(uid, index);

                    let translation = attribute.blue_team_positions[index as usize];
                    let rotation = attribute.blue_team_rotation;
                    let lon = rotation.angle_between(glam::Quat::IDENTITY);

                    (index, translation, rotation, lon)
                }
                Team::Red => {
                    let index = self.cnt_red_players;
                    self.cnt_red_players += 1;
                    self.red_players.insert(uid, index);

                    let translation = attribute.red_team_positions[index as usize];
                    let rotation = attribute.red_team_rotation;
                    let lon = rotation.angle_between(glam::Quat::IDENTITY);

                    (index, translation, rotation, lon)
                }
            };
            data.set_team_index(index as usize);
            data.translation = translation;
            data.rotation = rotation;
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
            ALLOW_DUPLICATES,
            players,
        );

        // 게임 월드에 참가한 세션에 패킷을 전송하고, 세션의 상태를 변경합니다.
        for (session, &uid) in world.sessions.iter() {
            session.tcp_write(packet.as_raw());

            // 다음 세션 상태로 전환합니다.
            let state = SessionMultiplayState::new(uid, world.events.clone());
            session.add_flow(SessionStateFlow::Change(Box::new(state)));
            let state = SessionFormationState::new(uid, world.events.clone());
            session.add_flow(SessionStateFlow::Push(Box::new(state)));
        }

        // 게임 월드 상태를 변경합니다.
        let state = GameWorldFormationState::new(
            ALLOW_DUPLICATES,
            false,
            self.stage_kind,
            false,
            self.blue_players.len(),
            self.red_players.len(),
        );
        let flow = GameWorldStateFlow::Change(Box::new(state));
        world.flows.push(flow);
    }

    fn on_resume(&mut self, world: &mut GameWorld) {
        let mut blue_players =
            HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new());
        let mut red_players =
            HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new());

        // 플레이어 데이터를 초기화합니다.
        for (&uid, data) in world.players.iter_mut() {
            // 게임 월드 데이터를 초기화합니다.
            data.rotation = glam::Quat::IDENTITY;
            data.translation = glam::Vec3A::ZERO;
            data.velocity = Velocity::new();
            data.direction = MovingDirection::new();
            data.latlon = LatLon::default();
            data.damage_dealt = 0;
            data.damage_taken = 0;
            data.healing_given = 0;
            data.skill_cost_data = SkillCostData::splat(0);
            data.action_state_timer = ActionStateTimer::new(0);
            data.movement_state_timer = MovementStateTimer::new(0);
            data.skill_cost_timer = 0;
            data.input_state_timer = InputStateTimer::new(0);
            data.kill_count = 0;
            data.retreat_count = 0;
            data.held_input = HeldInput::empty();
            data.health_data = HealthData::splat(0);
            data.bullet_data = BulletData::splat(0);
            data.action_state = ActionState::Idle;
            data.movement_state = MovementState::Idle;
            data.set_character_kind(CharacterKind::default());
            data.set_team_index(0);
            data.set_ready_to_play(false);
            data.set_invincible(true);
            data.set_grounded(true);

            // 남아있는 플레이어 데이터를 재구축합니다.
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
                GameWorldSystemEvent::UpdatePing(state) => {
                    self.handle_update_ping_event(world, session, uid, state);
                }
                _ => {
                    log::warn!(
                        "ignored >> unused system event (EVENT:{:?}, STATE:{:?})",
                        &event,
                        &self
                    );
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
