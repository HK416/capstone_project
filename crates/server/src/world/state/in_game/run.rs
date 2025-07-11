use std::{
    collections::VecDeque,
    f32::consts::TAU,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ahash::{HashMap, HashSet, RandomState};
use mod_network::{
    components::{
        ActionEvent, ActionEventDetail, BulletKind, DamageLogData, HeldInput, InGamePlayerPullData,
        InputEvent, InputSnapshot, LatLon, MAX_IN_GAME_PLAYERS, MAX_LATITUDE, MAX_PLAYER_SNAPSHOTS,
        MIN_LATITUDE, NetworkState, ObjectId, Permission, PlayerSnapshot, StageAttributes,
        StageKind, Team, UserId, update_action_state, update_action_state_timer,
        update_movement_state, update_movement_state_timer, update_player_rotation,
        update_player_translation,
    },
    protocol::{
        InGamePullPacket, JoinFailedReason, JoinRoomFailedPacket, MAX_INPUT_SNAPSHOTS, Packet,
    },
};
use mod_physics::{
    collision::{Collider, ColliderTreeIterator, DynamicCollision},
    object3d::{BoundingBox, Capsule, Sphere},
};
use rand::seq::SliceRandom;
use tokio::time::Duration;

use crate::{
    data::get_stage_attributes,
    entities::{Bullet, Player},
    session::Session,
    world::{
        GameWorld, GameWorldEvent, GameWorldInGameRunStateEvent, GameWorldState,
        GameWorldSystemEvent,
    },
};

/// 최대 게임 진행 시간 (단위: ms)
pub const MAX_GAME_TIME: u32 = 1_000 * 60 * 5;
/// 1 스킬 코스트가 오르는데 걸리는 시간 (단위: ms)
pub const SKILL_COST_TICK: u16 = 100;

/// 인게임 상태 게임 월드입니다.
/// 게임을 진행합니다.
pub struct GameWorldInGameRunState {
    /// 게임 스테이지 종류
    stage_kind: StageKind,
    /// 게임 플레이 경과 시간
    play_elapsed_time_ms: u32,
    /// 마지막 패킷을 전송 경과 시간
    packet_send_elapsed_time_ms: u32,

    /// 블루 팀 플레이어 수
    num_blue_players: usize,
    /// 레드 팀 플레이어 수
    num_red_players: usize,
    /// 떠난 플레이어 식별자입니다.
    leaved_players: HashSet<UserId>,

    /// 데미지 로그 데이터 목록
    damage_log_data: Vec<DamageLogData>,

    /// 오브젝트 식별자 생성에 사용됩니다.
    counter: u32,
    /// 총알 오브젝트
    bullets: HashMap<ObjectId, Bullet>,
    /// 플레이어 총알 맵
    player_bullet_map: HashMap<UserId, HashMap<u16, ObjectId>>,

    /// 플레이어 스냅샷 데이터
    player_snapshots: HashMap<UserId, VecDeque<PlayerSnapshot>>,
    /// 플레이어 이벤트 스냅샷 데이터
    input_snapshots: HashMap<UserId, VecDeque<InputSnapshot>>,
}

impl GameWorldInGameRunState {
    pub fn new(
        stage_kind: StageKind,
        num_blue_players: usize,
        num_red_players: usize,
        leaved_players: HashSet<UserId>,
    ) -> Self {
        Self {
            stage_kind,
            play_elapsed_time_ms: 0,
            packet_send_elapsed_time_ms: 0,
            num_blue_players,
            num_red_players,
            leaved_players,
            damage_log_data: Vec::with_capacity(128),
            counter: 0,
            bullets: HashMap::with_capacity_and_hasher(1024, RandomState::new()),
            player_bullet_map: HashMap::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::new(),
            ),
            player_snapshots: HashMap::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::new(),
            ),
            input_snapshots: HashMap::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::new(),
            ),
        }
    }

    /// [`GameWorldSystemEvent::PlayerJoin`] 이벤트를 처리합니다.
    fn handle_player_join_event(&mut self, session: Arc<Session>, _uid: UserId) {
        // 패킷을 전송합니다.
        let reason = JoinFailedReason::InProgress;
        let packet = JoinRoomFailedPacket::new(reason);
        session.tcp_write(packet.as_raw());
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

        // 플레이어 데이터를 가져옵니다.
        // 현재 상태에서 플레이어 데이터를 제거하지 않습니다.
        let data = match world.players.get_mut(&uid) {
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

        // 떠난 플레이어 식별자를 추가합니다.
        self.leaved_players.insert(uid);

        // 제거된 플레이어의 권한이 관리자인 경우
        // 남은 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
        if permission == Permission::Admin {
            let mut remainings: Vec<_> = world.sessions.values().cloned().collect();
            remainings.shuffle(&mut rand::rng());

            if let Some(uid) = remainings.pop() {
                match world.players.get_mut(&uid) {
                    Some(data) => {
                        world.admin = uid;
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

        // 네트워크 상태를 설정합니다.
        data.set_network_state(state);
    }

    /// [`GameWorldInGameRunStateEvent::InputSnapshot`] 이벤트를 처리합니다.
    fn handle_input_snapshot_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        client_play_elapsed_time_ms: u32,
        snapshots: Vec<InputSnapshot>,
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

        // 오프셋 시간을 계산 후 저장합니다.
        let offset_time = (session.ping() / 2) as i32;
        if self.play_elapsed_time_ms < client_play_elapsed_time_ms {
            data.offset_time = -offset_time;
        } else {
            data.offset_time = offset_time;
        }

        // 오프셋이 너무 큰 경우 무시합니다.
        if offset_time.abs() > 250 {
            return;
        }

        // 속성 데이터를 가져옵니다.
        let character_attributes = data.character_attributes();
        let stage_attributes = get_stage_attributes(self.stage_kind);

        // 플레이어 데이터 스냅샷 버퍼의 소유권을 가져옵니다.
        let mut data_snapshots = match self.player_snapshots.remove(&uid) {
            Some(data_snapshots) => data_snapshots,
            None => {
                log::error!(
                    "Player({}) data snapshot data not found in {}!",
                    &uid,
                    &world
                );
                eprintln!(
                    "Player({}) data snapshot data not found in {}!",
                    &uid, &world
                );
                session.close();
                return;
            }
        };

        // 플레이어 입력 스냅샷 버퍼의 소유권을 가져옵니다.
        let mut input_snapshots = match self.input_snapshots.remove(&uid) {
            Some(event_snapshots) => event_snapshots,
            None => VecDeque::with_capacity(MAX_INPUT_SNAPSHOTS + 1),
        };

        // 전달된 입력 스냅샷을 스냅샷 버퍼에 추가합니다.
        let mut first_input_snapshot_time = None;
        for mut snapshot in snapshots.into_iter() {
            // 전달된 입력 스냅샷의 보정된 게임 플레이 경과 시간을 계산합니다.
            let new_play_elapsed_time_ms = snapshot
                .play_elapsed_time_ms()
                .saturating_add_signed(offset_time);
            snapshot.set_play_elapsed_time_ms(new_play_elapsed_time_ms);

            // 스냅샷 버퍼 마지막 스냅샷의 게임 플레이 시간보다 커야 하며 서버 게임 플레이 시간보다 작아야합니다.
            let last_snapshot_time = input_snapshots
                .back()
                .map(|snapshot| snapshot.play_elapsed_time_ms())
                .unwrap_or(0);
            if last_snapshot_time <= snapshot.play_elapsed_time_ms()
                && snapshot.play_elapsed_time_ms() <= self.play_elapsed_time_ms
            {
                // 전달받은 첫 번째 스냅샷의 게임 플레이 경과 시간을 초기화합니다.
                if first_input_snapshot_time.is_none() {
                    first_input_snapshot_time = Some(snapshot.play_elapsed_time_ms());
                }

                // 입력 스냅샷을 추가합니다.
                input_snapshots.push_back(snapshot);

                // 오랜된 스냅샷을 제거합니다.
                while input_snapshots.len() > MAX_INPUT_SNAPSHOTS {
                    input_snapshots.pop_front();
                }
            }
        }

        // 과거 데이터 스냅샷 부터 최근 데이터 스냅샷 까지 순회하면서
        // 전달된 맨 처음 입력 스냅샷과 근사한 데이터 스냅샷을 선정합니다.
        let mut selected = None;
        if let Some(first_input_snapshot_time) = first_input_snapshot_time {
            for (index, data_snapshot) in data_snapshots.iter().enumerate() {
                if data_snapshot.play_elapsed_time_ms > first_input_snapshot_time {
                    break;
                }

                selected = Some((index, data_snapshot));
            }
        }

        // 선택된 데이터 스냅샷 정보를 가져옵니다.
        let (mut data_index, data_snapshot) = match selected {
            Some(pair) => pair,
            None => {
                log::info!("no suitable Player({}) snapshot found!", &uid);

                // 플레이어 이벤트 스냅샷의 소유권을 돌려놓습니다.
                self.input_snapshots.insert(uid, input_snapshots);

                // 플레이어 데이터 스냅샷의 소유권을 돌려놓습니다.
                self.player_snapshots.insert(uid, data_snapshots);
                return;
            }
        };

        // 선택한 데이터 스냅샷과 가까운 입력 스냅샷을 선정합니다.
        let mut selected = None;
        for (i, input_snapshot) in input_snapshots.iter().enumerate().rev() {
            if input_snapshot.play_elapsed_time_ms() < data_snapshot.play_elapsed_time_ms {
                break;
            }
            selected = Some((i, input_snapshot));
        }

        // 선택된 입력 스냅샷 정보를 가져옵니다.
        let (mut input_index, mut input_snapshot) = match selected {
            Some(pair) => pair,
            None => {
                log::info!("no suitable Player({}) snapshot found!", &uid);

                // 플레이어 이벤트 스냅샷의 소유권을 돌려놓습니다.
                self.input_snapshots.insert(uid, input_snapshots);

                // 플레이어 데이터 스냅샷의 소유권을 돌려놓습니다.
                self.player_snapshots.insert(uid, data_snapshots);
                return;
            }
        };

        // 데이터 스냅샷과 입력 스냅샷간의 시간 간격을 계산합니다.
        let mut interval =
            input_snapshot.play_elapsed_time_ms() - data_snapshot.play_elapsed_time_ms;
        // 시간 간격이 큰 경우 처리하지 않습니다.
        if interval > 250 {
            log::info!("no suitable Player({}) snapshot found!", &uid);

            // 플레이어 이벤트 스냅샷의 소유권을 돌려놓습니다.
            self.input_snapshots.insert(uid, input_snapshots);

            // 플레이어 데이터 스냅샷의 소유권을 돌려놓습니다.
            self.player_snapshots.insert(uid, data_snapshots);
            return;
        }

        // 재 시뮬레이션을 진행합니다.
        let mut current_play_elapsed_time_ms = data_snapshot.play_elapsed_time_ms;
        data.action_state = data_snapshot.action_state;
        data.movement_state = data_snapshot.movement_state;
        data.action_state_timer = data_snapshot.action_state_timer;
        data.movement_state_timer = data_snapshot.movement_state_timer;
        data.bullet_data = data_snapshot.bullet_data;
        data.skill_cost_data = data_snapshot.skill_cost_data;
        data.latlon = data_snapshot.latlon;
        data.translation = data_snapshot.translation;
        data.rotation = data_snapshot.rotation;
        data.velocity = data_snapshot.velocity;
        data.direction = data_snapshot.direction;
        data.input_state_timer = data_snapshot.input_state_timer;
        data.held_input = data_snapshot.held_input;
        data.set_invincible(data_snapshot.is_invincible);
        data.set_grounded(data_snapshot.is_grounded);

        'input: while current_play_elapsed_time_ms < self.play_elapsed_time_ms {
            // ------------------------------------//
            // 입력 스냅샷 까지 경과 시간의 데이터를 갱신합니다.
            if interval > 0 {
                let elapsed_time_ms = interval as u16;
                let action_events = Self::update_player(stage_attributes, data, elapsed_time_ms);
                // 행동 상태 이벤트를 처리합니다.
                for event in action_events {
                    match event {
                        ActionEvent::Respawn { timing } => {}
                        ActionEvent::Reload => {
                            data.bullet_data.remaining = data.bullet_data.num_maximum_bullets();
                        }
                        ActionEvent::Attack { timing } => {}
                        ActionEvent::Skill { timing } => {}
                    }
                }
                current_play_elapsed_time_ms += interval;
            }

            // ------------------------------------//
            // 입력 스냅샷을 적용합니다.
            match input_snapshot {
                InputSnapshot::CameraOrientation {
                    delta_lat,
                    delta_lon,
                    ..
                } => {
                    // 카메라 회전 각도를 더합니다.
                    data.latlon.lat =
                        (data.latlon.lat + delta_lat).clamp(MIN_LATITUDE, MAX_LATITUDE);
                    data.latlon.lon = (data.latlon.lon + delta_lon) % TAU;
                }
                InputSnapshot::KeyEvent { events, .. } => {
                    for event in events {
                        match event {
                            InputEvent::KeyPress(input_kind) => {
                                data.held_input |= input_kind.into_bits();
                            }
                            InputEvent::KeyRelease(input_kind) => {
                                data.held_input &= !input_kind.into_bits();
                            }
                        }
                    }

                    // 행동 상태를 갱신합니다.
                    update_action_state(
                        data.held_input,
                        &mut data.action_state,
                        &mut data.action_state_timer,
                        character_attributes,
                        &mut data.bullet_data,
                        &mut data.skill_cost_data,
                    );
                    update_movement_state(
                        data.held_input,
                        data.action_state,
                        &mut data.movement_state,
                        &mut data.movement_state_timer,
                    );
                }
            };

            // ------------------------------------//
            // 다음 입력 스냅샷 또는 다음 데이터 스냅샷 중
            // 시간 간격이 가까운 스냅샷까지 데이터를 갱신합니다.
            while current_play_elapsed_time_ms < self.play_elapsed_time_ms {
                let next_input_snapshot = input_snapshots.get(input_index + 1);
                let next_data_snapshot = data_snapshots.get_mut(data_index + 1);
                match (next_input_snapshot, next_data_snapshot) {
                    (Some(next_input_snapshot), Some(next_data_snapshot)) => {
                        if next_input_snapshot.play_elapsed_time_ms()
                            < next_data_snapshot.play_elapsed_time_ms
                        {
                            // 다음 입력 스냅샷이 더 가까운 미래인 경우
                            // 다음 입력 스냅샷까지 경과 시간을 갱신합니다.
                            input_index += 1;
                            input_snapshot = next_input_snapshot;
                            interval = input_snapshot.play_elapsed_time_ms()
                                - current_play_elapsed_time_ms;
                            continue 'input;
                        } else {
                            // 다음 데이터 스냅샷이 더 가까운 미래인 경우
                            // 다음 데이터 스냅샷 까지 경과 시간의 데이터를 갱신합니다.
                            interval = next_data_snapshot.play_elapsed_time_ms
                                - current_play_elapsed_time_ms;
                            if interval > 0 {
                                let elapsed_time_ms = interval as u16;
                                Self::update_player(stage_attributes, data, elapsed_time_ms);
                                current_play_elapsed_time_ms += interval;
                            }

                            // 데이터 스냅샷을 갱신합니다.
                            next_data_snapshot.action_state = data.action_state;
                            next_data_snapshot.movement_state = data.movement_state;
                            next_data_snapshot.action_state_timer = data.action_state_timer;
                            next_data_snapshot.movement_state_timer = data.movement_state_timer;
                            next_data_snapshot.latlon = data.latlon;
                            next_data_snapshot.translation = data.translation;
                            next_data_snapshot.rotation = data.rotation;
                            next_data_snapshot.velocity = data.velocity;
                            next_data_snapshot.direction = data.direction;
                            next_data_snapshot.input_state_timer = data.input_state_timer;
                            next_data_snapshot.held_input = data.held_input;
                            next_data_snapshot.is_invincible = data.is_invincible();
                            next_data_snapshot.is_grounded = data.is_grounded();
                            data_index += 1;
                        }
                    }
                    (Some(next_input_snapshot), None) => {
                        // 다음 입력 스냅샷까지 경과 시간을 갱신합니다.
                        input_index += 1;
                        input_snapshot = next_input_snapshot;
                        interval =
                            input_snapshot.play_elapsed_time_ms() - current_play_elapsed_time_ms;
                        continue 'input;
                    }
                    (None, Some(next_data_snapshot)) => {
                        // 다음 데이터 스냅샷 까지 경과 시간의 데이터를 갱신합니다.
                        interval =
                            next_data_snapshot.play_elapsed_time_ms - current_play_elapsed_time_ms;
                        if interval > 0 {
                            let elapsed_time_ms = interval as u16;
                            Self::update_player(stage_attributes, data, elapsed_time_ms);
                            current_play_elapsed_time_ms += interval;
                        }

                        // 데이터 스냅샷을 갱신합니다.
                        next_data_snapshot.action_state = data.action_state;
                        next_data_snapshot.movement_state = data.movement_state;
                        next_data_snapshot.action_state_timer = data.action_state_timer;
                        next_data_snapshot.movement_state_timer = data.movement_state_timer;
                        next_data_snapshot.latlon = data.latlon;
                        next_data_snapshot.translation = data.translation;
                        next_data_snapshot.rotation = data.rotation;
                        next_data_snapshot.velocity = data.velocity;
                        next_data_snapshot.direction = data.direction;
                        next_data_snapshot.input_state_timer = data.input_state_timer;
                        next_data_snapshot.held_input = data.held_input;
                        next_data_snapshot.is_invincible = data.is_invincible();
                        next_data_snapshot.is_grounded = data.is_grounded();
                        data_index += 1;
                    }
                    (None, None) => {
                        // 남은 시간까지 경과 시간을 갱신합니다.
                        interval = self.play_elapsed_time_ms - current_play_elapsed_time_ms;
                        if interval > 0 {
                            let elapsed_time_ms = interval as u16;
                            Self::update_player(stage_attributes, data, elapsed_time_ms);
                            current_play_elapsed_time_ms += interval;
                        }
                    }
                }
            }
        }

        // 플레이어 이벤트 스냅샷의 소유권을 돌려놓습니다.
        self.input_snapshots.insert(uid, input_snapshots);

        // 플레이어 데이터 스냅샷의 소유권을 돌려놓습니다.
        self.player_snapshots.insert(uid, data_snapshots);
    }

    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&mut self, world: &GameWorld) {
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (&uid, data) in world.players.iter() {
            let connected = !self.leaved_players.contains(&uid);
            players.push(InGamePlayerPullData::new(
                uid,
                data.kill_count,
                data.dead_count,
                data.health_data.shield,
                data.health_data.remaining,
                data.bullet_data.sequence,
                data.bullet_data.remaining,
                data.skill_cost_data.remaining,
                data.translation.to_array(),
                data.rotation.to_array(),
                data.velocity.0.to_array(),
                data.direction.0.to_array(),
                data.held_input,
                data.permission(),
                connected,
                data.is_grounded(),
                data.is_invincible(),
                data.network_state(),
                data.player_states(),
                data.action_state_timer,
                data.movement_state_timer,
                data.input_state_timer,
                data.latlon,
            ));

            if connected {
                // 플레이어 데이터 스냅샷의 소유권을 가져옵니다.
                let mut data_snapshots = match self.player_snapshots.remove(&uid) {
                    Some(data_snapshots) => data_snapshots,
                    None => VecDeque::with_capacity(MAX_PLAYER_SNAPSHOTS + 1),
                };

                // 데이터 스냅샷을 추가합니다.
                data_snapshots.push_back(PlayerSnapshot {
                    play_elapsed_time_ms: self.play_elapsed_time_ms,
                    action_state: data.action_state,
                    movement_state: data.movement_state,
                    action_state_timer: data.action_state_timer,
                    movement_state_timer: data.movement_state_timer,
                    bullet_data: data.bullet_data,
                    skill_cost_data: data.skill_cost_data,
                    latlon: data.latlon,
                    translation: data.translation,
                    rotation: data.rotation,
                    velocity: data.velocity,
                    direction: data.direction,
                    input_state_timer: data.input_state_timer,
                    held_input: data.held_input,
                    is_invincible: data.is_invincible(),
                    is_grounded: data.is_grounded(),
                });

                // 오래된 데이터 스냅샷을 제거합니다.
                while data_snapshots.len() > MAX_PLAYER_SNAPSHOTS {
                    data_snapshots.pop_front();
                }

                // 플레이어 데이터 스냅샷의 소유권을 돌려놓습니다.
                self.player_snapshots.insert(uid, data_snapshots);
            }
        }

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        // 상태 변경 이벤트를 가져옵니다.
        let mut packet = InGamePullPacket::new(self.play_elapsed_time_ms, players);
        for session in world.sessions.keys() {
            packet.ping = session.ping();
            session.tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldInGameRunState {
    /// 오브젝트 식별자를 생성합니다.
    pub fn generate_object_id(&mut self) -> ObjectId {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

        self.counter += 1;
        let cnt = self.counter & 0xFFFF;
        let time = duration.subsec_nanos() & 0xFFFF;

        ObjectId::new((time << 16) | cnt)
    }

    /// 게임 월드를 갱신합니다.
    fn update(&mut self, world: &mut GameWorld, elapsed: Duration) {
        let stage_attributes = get_stage_attributes(self.stage_kind);
        let elapsed_time_ms = elapsed.as_millis().min(u16::MAX as u128) as u16;
        let mut event_details = Vec::with_capacity(72);

        // 1. 플레이어 행동 이벤트를 수집합니다.
        for (&uid, data) in world.players.iter_mut() {
            // 서버와 연결이 끊어진 경우 건너뜁니다.
            if self.leaved_players.contains(&uid) {
                continue;
            }

            let character_attributes = data.character_attributes();

            // 플레이어 이동 방향을 갱신합니다.
            data.direction.update(data.held_input, data.latlon);

            // 행동 상태 타이머를 갱신합니다.
            let mut bullet_data = data.bullet_data;
            let mut skill_cost_data = data.skill_cost_data;
            let mut action_state = data.action_state;
            let mut action_state_timer = data.action_state_timer;
            let mut events = Vec::default();
            update_action_state_timer(
                data.held_input,
                &mut bullet_data,
                &mut skill_cost_data,
                &mut action_state,
                &mut action_state_timer,
                character_attributes,
                elapsed_time_ms,
                &mut events,
            );

            event_details.extend(events.drain(..).map(|e| match e {
                ActionEvent::Respawn { timing } => ActionEventDetail::Respawn { uid, timing },
                ActionEvent::Reload => ActionEventDetail::Reload { uid },
                ActionEvent::Attack { timing } => ActionEventDetail::Attack { uid, timing },
                ActionEvent::Skill { timing } => ActionEventDetail::Skill { uid, timing },
            }));
        }

        // 2. 행동 이벤트를 시간 순으로 정렬합니다.
        event_details.sort();

        // 3. 시간 순서에 따라 행동 이벤트를 처리합니다.
        let mut curr_elapsed_time_ms = 0;
        for event in event_details {
            let timing = event.timing();
            let elapsed_time_ms = timing.saturating_sub(curr_elapsed_time_ms);
            let elapsed_time_sec = elapsed_time_ms as f32 / 1000.0;
            curr_elapsed_time_ms = curr_elapsed_time_ms.max(timing);

            // 3.1. 모든 플레이어와 오브젝트 데이터를 현재 경과 시간 만큼 갱신합니다.
            if elapsed_time_ms > 0 {
                for data in world.players.values_mut() {
                    // 입력 상태 타이머를 갱신합니다.
                    data.input_state_timer
                        .update(data.held_input, elapsed_time_ms);

                    // 행동 상태 타이머를 갱신합니다.
                    let character_attributes = data.character_attributes();
                    update_action_state_timer(
                        data.held_input,
                        &mut data.bullet_data,
                        &mut data.skill_cost_data,
                        &mut data.action_state,
                        &mut data.action_state_timer,
                        character_attributes,
                        elapsed_time_ms,
                        &mut vec![],
                    );

                    // 움직임 상태 타이머를 갱신합니다.
                    update_movement_state_timer(
                        data.action_state,
                        &mut data.movement_state,
                        &mut data.movement_state_timer,
                        character_attributes,
                        elapsed_time_ms,
                    );

                    // 플레이어 캐릭터 방향을 갱신합니다.
                    let mut look = data.rotation.mul_vec3a(glam::Vec3A::Z);
                    look = update_player_rotation(
                        look,
                        data.action_state,
                        data.movement_state,
                        data.direction,
                        data.latlon,
                    );
                    let z = look.normalize_or(glam::Vec3A::Z);
                    let x = glam::Vec3A::Y.cross(z);
                    let y = z.cross(x);
                    data.rotation = glam::Quat::from_mat3a(&glam::mat3a(x, y, z));

                    // 플레이어 캐릭터 위치를 갱신합니다.
                    let team = data.team();
                    let mut is_grounded = data.is_grounded();
                    let mut is_invincible = data.is_invincible();
                    update_player_translation(
                        stage_attributes,
                        character_attributes,
                        data.action_state,
                        &mut data.movement_state,
                        &mut data.movement_state_timer,
                        &mut data.velocity,
                        &mut data.translation,
                        data.direction,
                        data.held_input,
                        team,
                        &mut is_grounded,
                        &mut is_invincible,
                        Some(&mut data.health_data),
                        data.input_state_timer,
                        elapsed_time_sec,
                    );
                    data.set_grounded(is_grounded);
                    data.set_invincible(is_invincible);
                }
            }

            // 3.2. 행동 이벤트를 처리합니다.
            match event {
                ActionEventDetail::Respawn { uid, .. } => {
                    // 해당 플레이어 데이터를 가져옵니다.
                    let data = match world.players.get_mut(&uid) {
                        Some(data) => data,
                        None => {
                            log::error!("Player({}) not found in {}!", &uid, &world);
                            eprintln!("Player({}) not found in {}!", &uid, &world);
                            continue;
                        }
                    };

                    // 플레이어 위치와 상태를 초기화합니다.
                    let team = data.team();
                    let team_index = data.team_index();
                    let (rotation, translation) = match team {
                        Team::Blue => (
                            stage_attributes.blue_team_rotation,
                            stage_attributes.blue_team_positions[team_index],
                        ),
                        Team::Red => (
                            stage_attributes.red_team_rotation,
                            stage_attributes.red_team_positions[team_index],
                        ),
                    };
                    let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                    let longitude = glam::Quat::IDENTITY.angle_between(rotation);

                    data.health_data.shield = 0;
                    data.health_data.remaining = data.health_data.num_maximum_health();
                    data.bullet_data.remaining = data.bullet_data.num_maximum_bullets();
                    data.skill_cost_data.remaining = 0;
                    data.skill_cost_timer = 0;
                    data.input_state_timer.0 = 0;
                    data.rotation = rotation;
                    data.velocity.0 = glam::Vec3A::ZERO;
                    data.direction.0 = direction;
                    data.translation = translation;
                    data.dead_count += 1;
                    data.set_grounded(true);
                    data.set_invincible(true);
                    data.latlon = LatLon::new(10f32.to_radians(), longitude);
                }
                ActionEventDetail::Reload { uid } => {
                    // 플레이어 데이터를 가져옵니다.
                    let data = match world.players.get_mut(&uid) {
                        Some(data) => data,
                        None => {
                            log::error!("Player({}) not found in {}!", &uid, &world);
                            eprintln!("Player({}) not found in {}!", &uid, &world);
                            continue;
                        }
                    };

                    // 플레이어의 현재 남은 총알을 최대치로 설정합니다.
                    data.bullet_data.remaining = data.bullet_data.num_maximum_bullets();
                }
                ActionEventDetail::Attack { uid, .. } => {}
                ActionEventDetail::Skill { uid, .. } => {}
            }
        }

        // 4. 모든 플레이어와 오브젝트 데이터를 현재 경과 시간 만큼 갱신합니다.
        let elapsed_time_ms = elapsed_time_ms.saturating_sub(curr_elapsed_time_ms);
        let elapsed_time_sec = elapsed_time_ms as f32 / 1000.0;
        if elapsed_time_ms > 0 {
            for data in world.players.values_mut() {
                // 입력 상태 타이머를 갱신합니다.
                data.input_state_timer
                    .update(data.held_input, elapsed_time_ms);

                // 행동 상태 타이머를 갱신합니다.
                let character_attributes = data.character_attributes();
                update_action_state_timer(
                    data.held_input,
                    &mut data.bullet_data,
                    &mut data.skill_cost_data,
                    &mut data.action_state,
                    &mut data.action_state_timer,
                    character_attributes,
                    elapsed_time_ms,
                    &mut vec![],
                );

                // 움직임 상태 타이머를 갱신합니다.
                update_movement_state_timer(
                    data.action_state,
                    &mut data.movement_state,
                    &mut data.movement_state_timer,
                    character_attributes,
                    elapsed_time_ms,
                );

                // 플레이어 캐릭터 방향을 갱신합니다.
                let mut look = data.rotation.mul_vec3a(glam::Vec3A::Z);
                look = update_player_rotation(
                    look,
                    data.action_state,
                    data.movement_state,
                    data.direction,
                    data.latlon,
                );
                let z = look.normalize_or(glam::Vec3A::Z);
                let x = glam::Vec3A::Y.cross(z);
                let y = z.cross(x);
                data.rotation = glam::Quat::from_mat3a(&glam::mat3a(x, y, z));

                // 플레이어 캐릭터 위치를 갱신합니다.
                let team = data.team();
                let mut is_grounded = data.is_grounded();
                let mut is_invincible = data.is_invincible();
                update_player_translation(
                    stage_attributes,
                    character_attributes,
                    data.action_state,
                    &mut data.movement_state,
                    &mut data.movement_state_timer,
                    &mut data.velocity,
                    &mut data.translation,
                    data.direction,
                    data.held_input,
                    team,
                    &mut is_grounded,
                    &mut is_invincible,
                    Some(&mut data.health_data),
                    data.input_state_timer,
                    elapsed_time_sec,
                );
                data.set_grounded(is_grounded);
                data.set_invincible(is_invincible);
            }
        }
    }

    /// 0.1m 마다 바닥과의 충돌을 검사하여 바닥과의 충돌을 확인합니다.
    fn check_bullet_ground_collision(
        &self,
        bullet_position: &glam::Vec3A,
        bullet_radius: f32,
        move_v_normalized: &glam::Vec3A,
        move_distance: f32,
    ) -> Option<f32> {
        let stage_attributes = get_stage_attributes(self.stage_kind);
        let mut nearest_distance = None;
        let mut position = bullet_position.clone();
        let mut moved = 0.0;
        let v = move_v_normalized * 0.1;
        while moved < move_distance {
            if let Some(height) = stage_attributes.get_area_height(position.x, position.z) {
                if position.y <= height + bullet_radius {
                    nearest_distance = Some(moved);
                    break;
                }
            }
            position += v;
            moved += 0.1;
        }

        nearest_distance
    }

    /// 총알과 충돌하는 건물과의 거리를 리턴합니다.
    fn check_bullet_building_collision(
        &self,
        bullet_collider: &Sphere,
        move_v: &glam::Vec3A,
    ) -> Option<f32> {
        let mut nearest_distance = f32::MAX;
        let stage_attributes = get_stage_attributes(self.stage_kind);
        let colliders = &stage_attributes.collider;
        for collider in ColliderTreeIterator::new(colliders) {
            // broadphase 검사 - 시작지점과 도착지점을 포함하는 AABB를 생성
            let rad_box = glam::Vec3A::new(
                bullet_collider.radius * move_v.x.signum(),
                bullet_collider.radius * move_v.y.signum(),
                bullet_collider.radius * move_v.z.signum(),
            );
            let center = glam::Vec3A::from(bullet_collider.center);
            let start = center - rad_box;
            let end = center + move_v + rad_box;
            let swept_aabb = BoundingBox::from_start_end(start.into(), end.into());

            if collider.check_aabb_collision(&swept_aabb) {
                // narrowphase 검사 - 총알과 충돌체의 충돌 검사
                let details = match collider {
                    Collider::Aabb(aabb) => {
                        bullet_collider.check_dynamic_collision_details(move_v, aabb)
                    }
                    Collider::Obb(obb) => {
                        bullet_collider.check_dynamic_collision_details(move_v, obb)
                    }
                    Collider::Capsule(capsule) => {
                        bullet_collider.check_dynamic_collision_details(move_v, capsule)
                    }
                    Collider::OrientedCapsule(obb) => {
                        bullet_collider.check_dynamic_collision_details(move_v, obb)
                    }
                    Collider::Sphere(sphere) => {
                        bullet_collider.check_dynamic_collision_details(move_v, sphere)
                    }
                };
                if let Some(details) = details {
                    // 충돌체와의 충돌이 발생한 경우, 총알의 남은 거리와 충돌체의 거리 비교
                    let distance = details.distance;
                    if distance < nearest_distance {
                        nearest_distance = distance;
                    }
                }
            }
        }

        if nearest_distance == f32::MAX {
            None
        } else {
            Some(nearest_distance)
        }
    }

    /// 총알과 충돌하는 플레이어를 확인합니다.  
    /// 건물, 바닥 등과 충돌시에는 총알의 남은 거리를 0.0으로 설정하고 None을 리턴합니다.  
    /// 움직일 벡터(move_v)는 0이 아니어야 합니다.
    fn check_bullet_collision(
        &self,
        world: &GameWorld,
        id: ObjectId,
        data: &mut Bullet,
        move_v: glam::Vec3A,
    ) -> Option<UserId> {
        let mut nearest_distance = f32::MAX;
        let mut move_v = move_v.clone();
        let move_v_normalized = move_v.normalize();

        // 지형 충돌 검사
        if let Some(collision_distance) = self.check_bullet_ground_collision(
            &data.translation,
            data.radius,
            &move_v_normalized,
            move_v.length(),
        ) {
            nearest_distance = collision_distance;
            move_v = move_v_normalized * nearest_distance;
            data.remaining_distance = 0.0;
            log::debug!("Bullet({}) hit ground (distance: {})", id, nearest_distance);
        }

        if nearest_distance == 0.0 {
            return None;
        }

        // 건물 충돌 검사
        let bullet_collider = Sphere {
            center: data.translation.into(),
            radius: data.radius,
        };
        if let Some(collision_distance) =
            self.check_bullet_building_collision(&bullet_collider, &move_v)
        {
            nearest_distance = collision_distance;
            move_v = move_v_normalized * nearest_distance;
            data.remaining_distance = 0.0;
            log::debug!(
                "Bullet({}) hit building (distance: {})",
                id,
                nearest_distance
            );
        }

        if nearest_distance == 0.0 {
            return None;
        }

        // 플레이어 충돌 검사
        let mut nearest_player_id = None;
        for (&uid, player) in world.players.iter() {
            if uid == data.shooter_id
                || player.health_data.remaining == 0
                || player.team() == data.shooter_team
                || player.is_invincible()
            {
                continue;
            }

            let character_attributes = player.character_attributes();
            let mut player_collider = character_attributes.collider.clone();
            player_collider.center = player.translation.into();

            // 충돌 처리: 플레이어 - 총알
            if let Some(info) =
                bullet_collider.check_dynamic_collision_details(&move_v, &player_collider)
            {
                if info.distance <= move_v.length() {
                    if info.distance < nearest_distance {
                        nearest_distance = info.distance;
                        nearest_player_id = Some(uid);
                    }
                }
            }
        }

        nearest_player_id
    }

    /// 주어진 시간 만큼 플레이어 데이터를 갱신합니다.
    fn update_player(
        stage_attributes: &StageAttributes,
        data: &mut Player,
        elapsed_time_ms: u16,
    ) -> Vec<ActionEvent> {
        let elapsed_time_sec = elapsed_time_ms as f32 / 1000.0;
        let character_attributes = data.character_attributes();
        let mut action_events = Vec::default();

        // 입력 상태 타이머를 갱신합니다.
        data.input_state_timer
            .update(data.held_input, elapsed_time_ms);

        // 행동 상태 타이머를 갱신합니다.
        update_action_state_timer(
            data.held_input,
            &mut data.bullet_data,
            &mut data.skill_cost_data,
            &mut data.action_state,
            &mut data.action_state_timer,
            character_attributes,
            elapsed_time_ms,
            &mut action_events,
        );

        // 움직임 상태 타이머를 갱신합니다.
        update_movement_state_timer(
            data.action_state,
            &mut data.movement_state,
            &mut data.movement_state_timer,
            character_attributes,
            elapsed_time_ms,
        );

        // 이동 방향을 갱신합니다.
        data.direction.update(data.held_input, data.latlon);

        // 플레이어 캐릭터 방향을 갱신합니다.
        let mut look = data.rotation.mul_vec3a(glam::Vec3A::Z);
        look = update_player_rotation(
            look,
            data.action_state,
            data.movement_state,
            data.direction,
            data.latlon,
        );
        let z = look.normalize_or(glam::Vec3A::Z);
        let x = glam::Vec3A::Y.cross(z);
        let y = z.cross(x);
        data.rotation = glam::Quat::from_mat3a(&glam::mat3a(x, y, z)).normalize();

        // 플레이어 캐릭터 위치를 갱신합니다.
        let team = data.team();
        let mut is_grounded = data.is_grounded();
        let mut is_invincible = data.is_invincible();
        update_player_translation(
            stage_attributes,
            character_attributes,
            data.action_state,
            &mut data.movement_state,
            &mut data.movement_state_timer,
            &mut data.velocity,
            &mut data.translation,
            data.direction,
            data.held_input,
            team,
            &mut is_grounded,
            &mut is_invincible,
            Some(&mut data.health_data),
            data.input_state_timer,
            elapsed_time_sec,
        );
        data.set_grounded(is_grounded);
        data.set_invincible(is_invincible);

        action_events
    }
}

impl GameWorldState for GameWorldInGameRunState {
    fn on_enter(&mut self, world: &mut GameWorld) {
        self.broadcast(world);
    }

    fn on_exit(&mut self, world: &mut GameWorld) {
        // 떠난 플레이어 데이터를 정리합니다.
        for uid in self.leaved_players.iter() {
            world.players.remove(uid);
        }
    }

    fn handle_event(&mut self, world: &mut GameWorld, event: GameWorldEvent) {
        match event {
            GameWorldEvent::System {
                session,
                uid,
                event,
            } => match event {
                GameWorldSystemEvent::PlayerJoin { .. } => {
                    self.handle_player_join_event(session, uid);
                }
                GameWorldSystemEvent::PlayerLeave => {
                    self.handle_player_leave_event(world, session, uid);
                }
                GameWorldSystemEvent::UpdatePing(state) => {
                    self.handle_update_ping_event(world, session, uid, state);
                }
            },
            GameWorldEvent::InGameRunState(event) => match event {
                GameWorldInGameRunStateEvent::InputSnapshot {
                    session,
                    uid,
                    client_play_elapsed_time_ms,
                    snapshots,
                } => self.handle_input_snapshot_event(
                    world,
                    session,
                    uid,
                    client_play_elapsed_time_ms,
                    snapshots,
                ),
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

    fn on_advanced(&mut self, world: &mut GameWorld, elapsed: Duration) {
        let elapsed_time_ms = elapsed.as_millis().min(MAX_GAME_TIME as u128) as u32;

        // 게임 월드를 갱신합니다.
        self.update(world, elapsed);

        // 플레이 경과 시간을 갱신합니다.
        self.play_elapsed_time_ms = self
            .play_elapsed_time_ms
            .saturating_add(elapsed_time_ms)
            .min(MAX_GAME_TIME);
        // 패킷 전송 경과 시간을 갱신합니다.
        self.packet_send_elapsed_time_ms = self
            .packet_send_elapsed_time_ms
            .saturating_add(elapsed_time_ms);

        // 일전 시각마다 패킷을 전송합니다.
        const TICK: u32 = 16;
        if self.packet_send_elapsed_time_ms >= TICK {
            self.packet_send_elapsed_time_ms = 0;
            self.broadcast(world);
        }

        // self.try_enter_next_state(world);
    }
}
