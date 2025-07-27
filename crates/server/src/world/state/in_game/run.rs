use std::{
    f32::{EPSILON, consts::TAU},
    num::NonZeroU32,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ahash::{HashMap, HashSet, RandomState};
use mod_network::{
    components::{
        ActionEvent, ActionEventDetail, ActionNotify, ActionState, BulletKind, CharacterKind,
        Damage, DamageLogData, HeldInput, InGameBulletPullData, InGamePlayerPullData,
        InGamePlayerStatusPullData, InputEvent, InputSnapshot, LatLon, MAX_IN_GAME_BULLETS,
        MAX_IN_GAME_LOGS, MAX_IN_GAME_PLAYERS, MAX_LATITUDE, MIN_LATITUDE, MovementState,
        NetworkState, ObjectId, Permission, StageAttributes, StageKind, Team, UserId,
        get_camera_transform, update_action_state, update_action_state_timer,
        update_movement_state, update_movement_state_timer, update_player_rotation,
        update_player_translation, update_view_state, update_view_state_timer,
    },
    protocol::{
        InGamePullPacket, InGameStatusPacket, JoinFailedReason, JoinRoomFailedPacket, Packet,
    },
};
use mod_physics::{
    collision::{Collider, ColliderTreeIterator, DynamicCollision},
    object3d::{BoundingBox, Frustum, Sphere},
};
use rand::{
    distr::{Distribution, Uniform},
    seq::SliceRandom,
};
use tokio::time::Duration;

use crate::{
    data::get_stage_attributes,
    entities::{Bullet, CapturePointObject, Player},
    session::Session,
    world::{
        GameWorld, GameWorldEvent, GameWorldInGameFinishState, GameWorldInGameRunStateEvent,
        GameWorldState, GameWorldStateFlow, GameWorldSystemEvent,
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
    /// 커스텀 게임 여부
    custom_game: bool,
    /// 게임 플레이 경과 시간
    play_elapsed_time_ms: u32,
    /// 마지막 Pull 패킷 전송 경과 시간
    pull_send_elapsed_time_ms: u32,
    /// 마지막 Status 패킷 전송 경과 시간
    status_send_elapsed_time_ms: u32,

    /// x축 방향의 게임 월드 절반 크기
    half_size_x: NonZeroU32,
    /// y축 방향의 게임 월드 절반 크기
    half_size_y: NonZeroU32,
    /// z축 방향의 게임 월드 절반 크기
    half_size_z: NonZeroU32,

    /// 블루 팀 플레이어 수
    num_blue_players: usize,
    /// 레드 팀 플레이어 수
    num_red_players: usize,
    /// 떠난 플레이어 식별자입니다.
    leaved_players: HashSet<UserId>,

    /// 제거된 총알 오브젝트 목록
    removed_bullets: HashSet<ObjectId>,
    /// 데미지 로그 데이터 목록
    damage_logs: Vec<DamageLogData>,

    /// 오브젝트 식별자 생성에 사용됩니다.
    counter: u32,
    /// 총알 오브젝트
    bullets: HashMap<ObjectId, Bullet>,

    /// 점령지 관리 오브젝트
    capture_point: CapturePointObject,
}

impl GameWorldInGameRunState {
    pub fn new(
        stage_kind: StageKind,
        custom_game: bool,
        half_size_x: NonZeroU32,
        half_size_y: NonZeroU32,
        half_size_z: NonZeroU32,
        num_blue_players: usize,
        num_red_players: usize,
        leaved_players: HashSet<UserId>,
    ) -> Self {
        // 스테이지 속성 데이터를 가져옵니다.
        let stage_attributes = get_stage_attributes(stage_kind);
        let capture_point = CapturePointObject::new(stage_attributes.capture_zone.clone());

        Self {
            stage_kind,
            custom_game,
            play_elapsed_time_ms: 0,
            pull_send_elapsed_time_ms: 0,
            status_send_elapsed_time_ms: 0,
            half_size_x,
            half_size_y,
            half_size_z,
            num_blue_players,
            num_red_players,
            leaved_players,
            removed_bullets: HashSet::with_capacity_and_hasher(
                MAX_IN_GAME_BULLETS,
                RandomState::new(),
            ),
            damage_logs: Vec::with_capacity(MAX_IN_GAME_LOGS),
            counter: 0,
            bullets: HashMap::with_capacity_and_hasher(MAX_IN_GAME_BULLETS, RandomState::new()),
            capture_point,
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

    /// [`GameWorldInGameRunStateEvent::Input`] 이벤트를 처리합니다.
    fn handle_input_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        client_play_elapsed_time: u32,
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

        // 시간 차이가 큰 경우 무시합니다.
        let diff_t = self.play_elapsed_time_ms as i32 - client_play_elapsed_time as i32;
        if diff_t.abs() > 250 {
            return;
        }

        // 입력을 처리합니다.
        let character_attributes = data.character_attributes();
        let mut latest_input_time = 0;
        for snapshot in snapshots {
            if latest_input_time <= snapshot.play_elapsed_time_ms() {
                latest_input_time = snapshot.play_elapsed_time_ms();
                match snapshot {
                    // 카메라 이동 입력을 처리합니다.
                    InputSnapshot::CameraOrientation {
                        delta_lat,
                        delta_lon,
                        ..
                    } => {
                        data.latlon.lat =
                            (data.latlon.lat + delta_lat).clamp(MIN_LATITUDE, MAX_LATITUDE);
                        data.latlon.lon = (data.latlon.lon + delta_lon) % TAU;
                    }
                    // 키 입력을 처리합니다.
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
                    }
                }
            }
        }

        // 행동 상태와 움직임 상태를 처리합니다.
        let mut action_events = Vec::default();
        update_action_state(
            data.held_input,
            &mut data.action_state,
            &mut data.action_state_timer,
            character_attributes,
            &mut data.bullet_data,
            &mut data.skill_cost_data,
            &mut action_events,
        );
        update_movement_state(
            data.held_input,
            data.action_state,
            &mut data.movement_state,
            &mut data.movement_state_timer,
        );
        update_view_state(
            data.action_state,
            &mut data.view_state,
            &mut data.view_state_timer,
            character_attributes,
            data.held_input,
        );

        // 행동 이벤트를 처리합니다.
        for event in action_events {
            match event {
                ActionEvent::Changed(action_state) => match action_state {
                    ActionState::Attack => data.action_notify = ActionNotify::StartAttack,
                    ActionState::Retreat => {
                        data.action_notify = ActionNotify::Retreat;
                    }
                    ActionState::Reload => {
                        data.action_notify = ActionNotify::Reload;
                    }
                    ActionState::Skill => {
                        data.action_notify = ActionNotify::StartSkill;
                        data.skill_cost_data.remaining = data
                            .skill_cost_data
                            .remaining
                            .saturating_sub(character_attributes.skill_cost);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    /// [`GameWorldInGameRunStateEvent::InputReset`] 이벤트를 처리합니다.
    fn handle_input_reset_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
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

        // 입력을 초기화합니다.
        data.held_input = HeldInput::empty();
    }

    /// 모든 세션에 Pull 패킷 데이터를 전송합니다.
    fn broadcast_pull_packet(&mut self, world: &mut GameWorld) {
        // 플레이어 데이터를 수집합니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (&uid, data) in world.players.iter_mut() {
            players.push(InGamePlayerPullData::new(
                uid,
                self.half_size_x,
                self.half_size_y,
                self.half_size_z,
                data.translation,
                data.rotation,
                data.action_state,
                data.action_notify.take(),
                data.action_state_timer,
                match data.movement_state {
                    MovementState::Landing => MovementState::Jumping,
                    _ => data.movement_state,
                },
                data.movement_state_timer,
                data.character_attributes(),
                data.latlon,
            ));
        }

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        // 총알 데이터를 수집합니다.
        let mut bullets = Vec::with_capacity(MAX_IN_GAME_BULLETS);
        for (&id, data) in self.bullets.iter() {
            bullets.push(InGameBulletPullData::new(
                id,
                data.kind,
                self.half_size_x,
                self.half_size_y,
                self.half_size_z,
                data.translation,
                data.rotation,
            ));
        }

        // 상태 변경 이벤트를 가져옵니다.
        let mut packet = InGamePullPacket::new(self.play_elapsed_time_ms, players, bullets);
        for session in world.sessions.keys() {
            packet.ping = session.ping();
            session.tcp_write(packet.as_raw());
        }
    }

    /// 모든 세션에 Status 패킷 데이터를 전송합니다.
    fn broadcast_status_packet(&mut self, world: &GameWorld) {
        // 플레이어 데이터를 수집합니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (&uid, data) in world.players.iter() {
            let connected = !self.leaved_players.contains(&uid);
            players.push(InGamePlayerStatusPullData::new(
                uid,
                data.kill_count,
                data.retreat_count,
                data.health_data.shield,
                data.health_data.remaining,
                data.bullet_data.remaining,
                data.skill_cost_data.remaining,
                data.permission(),
                connected,
                data.is_invincible(),
                data.network_state(),
            ));
        }

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        // 상태 변경 패킷을 생성합니다.
        let mut packet =
            InGameStatusPacket::new(self.capture_point.as_ref().clone(), players, vec![], vec![]);

        // 패킷을 전송합니다.
        let mut damage_logs: Vec<_> = self.damage_logs.drain(..).collect();
        let mut removed_bullets: Vec<_> = self.removed_bullets.drain().collect();
        loop {
            let count = damage_logs.len().min(MAX_IN_GAME_LOGS);
            packet.damage_logs = damage_logs.drain(..count).collect();

            let count = removed_bullets.len().min(MAX_IN_GAME_BULLETS);
            packet.removed_bullets = removed_bullets.drain(..count).collect();

            for session in world.sessions.keys() {
                session.tcp_write(packet.as_raw());
            }

            if damage_logs.is_empty() && removed_bullets.is_empty() {
                break;
            }
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
        let mut events = Vec::with_capacity(64);

        // 1. 플레이어 행동 이벤트를 수집합니다.
        for (&uid, player) in world.players.iter_mut() {
            // 서버와 연결이 끊어진 경우 건너뜁니다.
            if self.leaved_players.contains(&uid) {
                continue;
            }

            // 캐릭터 속성 데이터를 가져옵니다.
            let character_attributes = player.character_attributes();

            // 플레이어 이동 방향을 갱신합니다.
            player.direction.update(player.held_input, player.latlon);

            // 행동 상태 타이머를 갱신합니다.
            let mut bullet_data = player.bullet_data;
            let mut skill_cost_data = player.skill_cost_data;
            let mut action_state = player.action_state;
            let mut action_state_timer = player.action_state_timer;
            update_action_state_timer(
                uid,
                player.held_input,
                &mut bullet_data,
                &mut skill_cost_data,
                &mut action_state,
                &mut action_state_timer,
                character_attributes,
                elapsed_time_ms,
                &mut events,
            );
        }

        // 2. 행동 이벤트를 시간 순으로 정렬합니다.
        events.sort();

        // 3. 시간 순서에 따라 행동 이벤트를 처리합니다.
        let mut curr_elapsed_time_ms = 0;
        for ActionEventDetail { uid, timing, event } in events {
            // 서버와 연결이 끊어진 경우 건너뜁니다.
            if self.leaved_players.contains(&uid) {
                continue;
            }

            let elapsed_time_ms = timing.saturating_sub(curr_elapsed_time_ms);
            curr_elapsed_time_ms = curr_elapsed_time_ms.max(timing);

            // 3.1. 모든 플레이어와 오브젝트 데이터를 현재 경과 시간 만큼 갱신합니다.
            if elapsed_time_ms > 0 {
                self.update_player(world, elapsed_time_ms);
                self.update_bullet(world, elapsed_time_ms);
                self.capture_point
                    .update(world.players.values(), elapsed_time_ms);
            }

            // 3.2. 행동 이벤트를 처리합니다.
            match event {
                ActionEvent::Attack => {
                    // 발사한 플레이어 데이터의 소유권을 가져옵니다.
                    let mut shooter = match world.players.remove(&uid) {
                        Some(shooter) => shooter,
                        None => {
                            log::error!("Player({}) not found in {}!", &uid, &world);
                            eprintln!("Player({}) not found in {}!", &uid, &world);
                            continue;
                        }
                    };
                    if shooter.bullet_data.fires_per_attack <= 1 {
                        shooter.action_notify = ActionNotify::FirstAttack;
                    } else {
                        shooter.action_notify = ActionNotify::Attack;
                    }

                    // 발사한 플레이어의 무기 데이터를 가져옵니다.
                    let character_kind = shooter.character_kind();
                    let character_attributes = shooter.character_attributes();
                    let weapon_attributes = match &character_attributes.right_weapon {
                        Some(weapon_attributes) => weapon_attributes,
                        None => {
                            // 발사한 플레이어 데이터의 소유권을 돌려줍니다.
                            world.players.insert(uid, shooter);

                            log::error!("{} weapon attribute not found!", &character_kind);
                            eprintln!("{} weapon attribute not found!", &character_kind);
                            continue;
                        }
                    };

                    // 총알을 발사한 시점의 총알의 위치와 방향을 계산합니다.
                    let (translation, rotation, new_rotation) = weapon_attributes
                        .get_position_and_direction(
                            shooter.view_state,
                            shooter.view_state_timer,
                            character_attributes,
                            shooter.translation,
                            shooter.rotation,
                            shooter.latlon,
                        );

                    // 캐릭터의 회전 방향을 보정합니다.
                    shooter.rotation = new_rotation;

                    // 총알을 생성후 추가합니다.
                    let id = self.generate_object_id();
                    let shooter_id = uid;
                    let shooter_team = shooter.team();
                    let bullet_kind: BulletKind = match character_kind {
                        CharacterKind::ArisOriginal => BulletKind::EnergyBoll,
                        _ => BulletKind::Common,
                    };
                    let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                    let velocity = direction * bullet_kind.speed();
                    let remaining_distance = character_attributes.attack_range as f32;
                    let radius = character_attributes.bullet_radius;
                    self.bullets.insert(
                        id,
                        Bullet::new(
                            shooter_id,
                            shooter_team,
                            bullet_kind,
                            translation,
                            rotation,
                            velocity,
                            remaining_distance,
                            radius,
                        ),
                    );

                    // 발사한 플레이어 데이터의 소유권을 돌려줍니다.
                    world.players.insert(uid, shooter);
                }
                ActionEvent::Changed(action_state) => match action_state {
                    ActionState::Attack => {
                        // 플레이어 데이터를 가져옵니다.
                        let data = match world.players.get_mut(&uid) {
                            Some(data) => data,
                            None => {
                                log::error!("Player({}) not found in {}!", &uid, &world);
                                eprintln!("Player({}) not found in {}!", &uid, &world);
                                continue;
                            }
                        };

                        data.action_notify = ActionNotify::StartAttack
                    }
                    ActionState::Retreat => {
                        // 플레이어 데이터를 가져옵니다.
                        let data = match world.players.get_mut(&uid) {
                            Some(data) => data,
                            None => {
                                log::error!("Player({}) not found in {}!", &uid, &world);
                                eprintln!("Player({}) not found in {}!", &uid, &world);
                                continue;
                            }
                        };

                        data.action_notify = ActionNotify::Retreat;
                    }
                    ActionState::Reload => {
                        // 플레이어 데이터를 가져옵니다.
                        let data = match world.players.get_mut(&uid) {
                            Some(data) => data,
                            None => {
                                log::error!("Player({}) not found in {}!", &uid, &world);
                                eprintln!("Player({}) not found in {}!", &uid, &world);
                                continue;
                            }
                        };

                        data.action_notify = ActionNotify::Reload;
                    }
                    ActionState::Skill => {
                        // 플레이어 데이터를 가져옵니다.
                        let data = match world.players.get_mut(&uid) {
                            Some(data) => data,
                            None => {
                                log::error!("Player({}) not found in {}!", &uid, &world);
                                eprintln!("Player({}) not found in {}!", &uid, &world);
                                continue;
                            }
                        };

                        let character_attributes = data.character_attributes();
                        data.action_notify = ActionNotify::StartSkill;
                        data.skill_cost_data.remaining = data
                            .skill_cost_data
                            .remaining
                            .saturating_sub(character_attributes.skill_cost);
                    }
                    _ => {}
                },
                ActionEvent::Reload => {
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
                ActionEvent::Respawn => {
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
                    data.set_grounded(true);
                    data.set_invincible(true);
                    data.latlon = LatLon::new(10f32.to_radians(), longitude);
                }
                ActionEvent::Skill => {
                    self.use_player_skill(uid, world);
                }
            }
        }

        // 4. 모든 플레이어와 오브젝트 데이터를 현재 경과 시간 만큼 갱신합니다.
        let elapsed_time_ms = elapsed_time_ms.saturating_sub(curr_elapsed_time_ms);
        if elapsed_time_ms > 0 {
            self.update_player(world, elapsed_time_ms);
            self.update_bullet(world, elapsed_time_ms);
            self.capture_point
                .update(world.players.values(), elapsed_time_ms);
        }
    }

    /// 플레이어 데이터를 주어진 시간 만큼 갱신합니다.
    fn update_player(&mut self, world: &mut GameWorld, elapsed_time_ms: u16) {
        let elapsed_time_sec = elapsed_time_ms as f32 / 1000.0;
        let stage_attributes = get_stage_attributes(self.stage_kind);

        // 플레이어를 갱신합니다.
        for (&uid, player) in world.players.iter_mut() {
            // 서버와 연결이 끊어진 경우 건너뜁니다.
            if self.leaved_players.contains(&uid) {
                continue;
            }

            // 플레이어 스킬 코스트 타이머를 증가시킵니다.
            player.skill_cost_timer = player.skill_cost_timer + elapsed_time_ms;
            if player.skill_cost_timer >= SKILL_COST_TICK {
                player.skill_cost_timer -= SKILL_COST_TICK;
                player.skill_cost_data.remaining = (player.skill_cost_data.remaining + 1)
                    .min(player.skill_cost_data.num_maximum_cost());
            }

            // 입력 상태 타이머를 갱신합니다.
            player
                .input_state_timer
                .update(player.held_input, elapsed_time_ms);

            // 행동 상태 타이머를 갱신합니다.
            let character_attributes = player.character_attributes();
            let mut events = Vec::default();
            update_action_state_timer(
                uid,
                player.held_input,
                &mut player.bullet_data,
                &mut player.skill_cost_data,
                &mut player.action_state,
                &mut player.action_state_timer,
                character_attributes,
                elapsed_time_ms,
                &mut events,
            );
            // 움직임 상태 타이머를 갱신합니다.
            update_movement_state_timer(
                player.action_state,
                &mut player.movement_state,
                &mut player.movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            );
            // 시야 상태 타이머를 갱신합니다.
            update_view_state_timer(
                player.action_state,
                &mut player.view_state,
                &mut player.view_state_timer,
                character_attributes,
                elapsed_time_ms,
            );

            // 플레이어 캐릭터 방향을 갱신합니다.
            let mut look = player.rotation.mul_vec3a(glam::Vec3A::Z);
            look = update_player_rotation(
                look,
                player.action_state,
                player.movement_state,
                player.direction,
                player.latlon,
            );
            let z = look.normalize_or(glam::Vec3A::Z);
            let x = glam::Vec3A::Y.cross(z);
            let y = z.cross(x);
            player.rotation = glam::Quat::from_mat3a(&glam::mat3a(x, y, z));

            // 플레이어 캐릭터 위치를 갱신합니다.
            let team = player.team();
            let mut is_grounded = player.is_grounded();
            let mut is_invincible = player.is_invincible();
            update_player_translation(
                stage_attributes,
                character_attributes,
                player.action_state,
                &mut player.movement_state,
                &mut player.movement_state_timer,
                &mut player.velocity,
                &mut player.translation,
                player.direction,
                player.held_input,
                team,
                &mut is_grounded,
                &mut is_invincible,
                Some(&mut player.health_data),
                player.input_state_timer,
                elapsed_time_sec,
            );
            player.set_grounded(is_grounded);
            player.set_invincible(is_invincible);

            // 자신 팀의 진영인 경우 체력을 회복시킵니다.
            if stage_attributes.is_safe_area(team, player.translation.x, player.translation.z) {
                let healing = 2 * elapsed_time_ms;
                player.health_data.remaining = (player.health_data.remaining + healing)
                    .min(player.health_data.num_maximum_health());
            }
        }
    }

    /// 총알 오브젝트를 갱신합니다.
    fn update_bullet(&mut self, world: &mut GameWorld, elapsed_time_ms: u16) {
        let elapsed_time_sec = elapsed_time_ms as f32 / 1000.0;
        let stage_attributes = get_stage_attributes(self.stage_kind);

        // 총알 오브젝트를 갱신합니다.
        let mut removed = HashSet::default();
        for (&id, bullet) in self.bullets.iter_mut() {
            let velocity = bullet.velocity * elapsed_time_sec;
            let target_id =
                Self::check_bullet_collision(stage_attributes, &world.players, bullet, velocity);
            match target_id {
                Some(target_id) => {
                    // 발사자의 소유권을 가져옵니다.
                    let mut shooter = match world.players.remove(&bullet.shooter_id) {
                        Some(data) => data,
                        None => {
                            log::error!("Player({}) not found in {}!", &bullet.shooter_id, &world,);
                            eprintln!("Player({}) not found in {}!", &bullet.shooter_id, &world,);
                            continue;
                        }
                    };
                    // 피격자를 가져옵니다.
                    let hitted = match world.players.get_mut(&target_id) {
                        Some(data) => data,
                        None => {
                            // 발사자의 소유권을 돌려놓습니다.
                            world.players.insert(bullet.shooter_id, shooter);

                            log::error!("Player({}) not found in {}!", &target_id, &world,);
                            eprintln!("Player({}) not found in {}!", &target_id, &world,);
                            continue;
                        }
                    };

                    // 데미지 처리 및 로그를 추가합니다.
                    match bullet.kind {
                        BulletKind::ArisOriginalSkill => {
                            let skill_multi = 2.5;
                            let accuracy_multi = 2.0;
                            let s = &mut shooter;
                            let s_character_attributes = s.character_attributes();
                            let s_accuracy = s_character_attributes.accuracy_stat as f32;
                            let s_attack_pow = s_character_attributes.attack_power as f32;
                            let s_crit_rate = s_character_attributes.critical_rate as f32;
                            let s_crit_multi =
                                s_character_attributes.critical_damage as f32 / 100.0;
                            let h = hitted;
                            let h_character_attributes = h.character_attributes();
                            let h_evasion = h_character_attributes.evasion_stat as f32;
                            let h_defense_pow = h_character_attributes.defense_power as f32;
                            let damage = Self::hit_player(
                                s,
                                s_accuracy * accuracy_multi,
                                s_attack_pow * skill_multi,
                                s_crit_rate,
                                s_crit_multi,
                                h,
                                h_evasion,
                                h_defense_pow,
                            );
                            self.damage_logs.push(DamageLogData::new(target_id, damage));
                        }
                        BulletKind::MomoiOriginalSkill => {
                            let skill_multi = 0.78;
                            let s = &mut shooter;
                            let s_character_attributes = s.character_attributes();
                            let s_accuracy = s_character_attributes.accuracy_stat as f32;
                            let s_attack_pow = s_character_attributes.attack_power as f32;
                            let s_crit_rate = s_character_attributes.critical_rate as f32;
                            let s_crit_multi =
                                s_character_attributes.critical_damage as f32 / 100.0;
                            let h = hitted;
                            let h_character_attributes = h.character_attributes();
                            let h_evasion = h_character_attributes.evasion_stat as f32;
                            let h_defense_pow = h_character_attributes.defense_power as f32;
                            let damage = Self::hit_player(
                                s,
                                s_accuracy,
                                s_attack_pow * skill_multi,
                                s_crit_rate,
                                s_crit_multi,
                                h,
                                h_evasion,
                                h_defense_pow,
                            );
                            self.damage_logs.push(DamageLogData::new(target_id, damage));
                        }
                        _ => {
                            let s = &mut shooter;
                            let s_character_attributes = s.character_attributes();
                            let s_accuracy = s_character_attributes.accuracy_stat as f32;
                            let s_attack_pow = s_character_attributes.attack_power as f32;
                            let s_crit_rate = s_character_attributes.critical_rate as f32;
                            let s_crit_multi =
                                s_character_attributes.critical_damage as f32 / 100.0;
                            let h = hitted;
                            let h_character_attributes = h.character_attributes();
                            let h_evasion = h_character_attributes.evasion_stat as f32;
                            let h_defense_pow = h_character_attributes.defense_power as f32;
                            let damage = Self::hit_player(
                                s,
                                s_accuracy,
                                s_attack_pow,
                                s_crit_rate,
                                s_crit_multi,
                                h,
                                h_evasion,
                                h_defense_pow,
                            );

                            s.skill_cost_data.remaining = (s.skill_cost_data.remaining + 10)
                                .min(s.skill_cost_data.num_maximum_cost());
                            self.damage_logs.push(DamageLogData::new(target_id, damage));
                        }
                    }

                    // 발사자의 소유권을 돌려놓습니다.
                    world.players.insert(bullet.shooter_id, shooter);
                }
                None => {
                    bullet.translation += velocity;

                    // 갱신된 총알의 위치가 게임 월드 위치 제한을 넘은 경우 제거합니다.
                    let half_size_x = self.half_size_x.get() as f32;
                    let half_size_y = self.half_size_y.get() as f32;
                    let half_size_z = self.half_size_z.get() as f32;
                    if bullet.translation.x > half_size_x
                        || bullet.translation.x < -half_size_x
                        || bullet.translation.y > half_size_y
                        || bullet.translation.y < -half_size_y
                        || bullet.translation.z > half_size_z
                        || bullet.translation.z < -half_size_z
                    {
                        bullet.remaining_distance = 0.0;
                    } else {
                        bullet.remaining_distance -= velocity.length();
                    }
                }
            }

            // 수명이 다 된 총알을 제거합니다.
            if bullet.remaining_distance <= 0.0 {
                removed.insert(id);
            }
        }

        // 총알을 제거합니다.
        for id in removed {
            self.bullets.remove(&id);
            self.removed_bullets.insert(id);
        }
    }

    /// 플레이어 스킬을 사용합니다.
    fn use_player_skill(&mut self, uid: UserId, world: &mut GameWorld) {
        // 플레이어 데이터의 소유권을 가져옵니다.
        let mut data = match world.players.remove(&uid) {
            Some(data) => data,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                return;
            }
        };

        if data.skill_cost_data.count <= 1 {
            data.action_notify = ActionNotify::FirstSkill;
        } else {
            data.action_notify = ActionNotify::Skill;
        }

        match data.character_kind() {
            CharacterKind::ArisOriginal => {
                let character_attributes = data.character_attributes();
                // 카메라가 변환 행렬을 가져옵니다.
                let transform = get_camera_transform(
                    data.view_state,
                    data.view_state_timer,
                    character_attributes,
                    data.latlon,
                );
                let rotation = glam::Quat::from_mat4(&transform);
                let translation = data.translation + glam::vec3a(0.0, 0.4, 0.0);

                let id = self.generate_object_id();
                let shooter_id = uid;
                let shooter_team = data.team();
                let bullet_kind = BulletKind::ArisOriginalSkill;
                let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                let velocity = direction * bullet_kind.speed();
                let remaining_distance = character_attributes.attack_range as f32;
                let radius = 1.0;
                self.bullets.insert(
                    id,
                    Bullet::new(
                        shooter_id,
                        shooter_team,
                        bullet_kind,
                        translation,
                        rotation,
                        velocity,
                        remaining_distance,
                        radius,
                    ),
                );
            }
            CharacterKind::MomoiOriginal => {
                let character_attributes = data.character_attributes();
                // 카메라가 변환 행렬을 가져옵니다.
                let transform = get_camera_transform(
                    data.view_state,
                    data.view_state_timer,
                    character_attributes,
                    data.latlon,
                );
                let base_rotation = glam::Quat::from_mat4(&transform);
                let translation = data.translation + glam::vec3a(0.0, 0.4, 0.0);

                if data.skill_cost_data.count % 2 == 0 {
                    let id = self.generate_object_id();
                    let shooter_id = uid;
                    let shooter_team = data.team();
                    let bullet_kind = BulletKind::MomoiOriginalSkill;
                    let rotation = base_rotation * glam::Quat::from_rotation_y(-15f32.to_radians());
                    let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                    let velocity = direction * bullet_kind.speed();
                    let remaining_distance = character_attributes.attack_range as f32;
                    let radius = character_attributes.bullet_radius;
                    self.bullets.insert(
                        id,
                        Bullet::new(
                            shooter_id,
                            shooter_team,
                            bullet_kind,
                            translation,
                            rotation,
                            velocity,
                            remaining_distance,
                            radius,
                        ),
                    );

                    let id = self.generate_object_id();
                    let shooter_id = uid;
                    let shooter_team = data.team();
                    let bullet_kind = BulletKind::MomoiOriginalSkill;
                    let rotation = base_rotation * glam::Quat::from_rotation_y(-3f32.to_radians());
                    let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                    let velocity = direction * bullet_kind.speed();
                    let remaining_distance = character_attributes.attack_range as f32;
                    let radius = character_attributes.bullet_radius;
                    self.bullets.insert(
                        id,
                        Bullet::new(
                            shooter_id,
                            shooter_team,
                            bullet_kind,
                            translation,
                            rotation,
                            velocity,
                            remaining_distance,
                            radius,
                        ),
                    );

                    let id = self.generate_object_id();
                    let shooter_id = uid;
                    let shooter_team = data.team();
                    let bullet_kind = BulletKind::MomoiOriginalSkill;
                    let rotation = base_rotation * glam::Quat::from_rotation_y(9f32.to_radians());
                    let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                    let velocity = direction * bullet_kind.speed();
                    let remaining_distance = character_attributes.attack_range as f32;
                    let radius = character_attributes.bullet_radius;
                    self.bullets.insert(
                        id,
                        Bullet::new(
                            shooter_id,
                            shooter_team,
                            bullet_kind,
                            translation,
                            rotation,
                            velocity,
                            remaining_distance,
                            radius,
                        ),
                    );
                } else {
                    let id = self.generate_object_id();
                    let shooter_id = uid;
                    let shooter_team = data.team();
                    let bullet_kind = BulletKind::MomoiOriginalSkill;
                    let rotation =
                        base_rotation * glam::Quat::from_rotation_y(-9.5f32.to_radians());
                    let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                    let velocity = direction * bullet_kind.speed();
                    let remaining_distance = character_attributes.attack_range as f32;
                    let radius = character_attributes.bullet_radius;
                    self.bullets.insert(
                        id,
                        Bullet::new(
                            shooter_id,
                            shooter_team,
                            bullet_kind,
                            translation,
                            rotation,
                            velocity,
                            remaining_distance,
                            radius,
                        ),
                    );

                    let id = self.generate_object_id();
                    let shooter_id = uid;
                    let shooter_team = data.team();
                    let bullet_kind = BulletKind::MomoiOriginalSkill;
                    let rotation = base_rotation * glam::Quat::from_rotation_y(3f32.to_radians());
                    let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                    let velocity = direction * bullet_kind.speed();
                    let remaining_distance = character_attributes.attack_range as f32;
                    let radius = character_attributes.bullet_radius;
                    self.bullets.insert(
                        id,
                        Bullet::new(
                            shooter_id,
                            shooter_team,
                            bullet_kind,
                            translation,
                            rotation,
                            velocity,
                            remaining_distance,
                            radius,
                        ),
                    );

                    let id = self.generate_object_id();
                    let shooter_id = uid;
                    let shooter_team = data.team();
                    let bullet_kind = BulletKind::MomoiOriginalSkill;
                    let rotation = base_rotation * glam::Quat::from_rotation_y(15f32.to_radians());
                    let direction = rotation.mul_vec3a(glam::Vec3A::Z);
                    let velocity = direction * bullet_kind.speed();
                    let remaining_distance = character_attributes.attack_range as f32;
                    let radius = character_attributes.bullet_radius;
                    self.bullets.insert(
                        id,
                        Bullet::new(
                            shooter_id,
                            shooter_team,
                            bullet_kind,
                            translation,
                            rotation,
                            velocity,
                            remaining_distance,
                            radius,
                        ),
                    );
                }
            }
            CharacterKind::MidoriOriginal => {
                let character_attributes = data.character_attributes();
                let transform = get_camera_transform(
                    data.view_state,
                    data.view_state_timer,
                    character_attributes,
                    data.latlon,
                );
                let transform = glam::Mat4::from_translation(data.translation.into()) * transform;
                let proj = glam::Mat4::perspective_lh(
                    character_attributes.camera_def_fov_y,
                    1.0,
                    0.1,
                    35.0,
                );
                let view = glam::Mat4::look_to_lh(
                    transform.w_axis.truncate(),
                    transform.z_axis.truncate(),
                    glam::Vec3::Y,
                );
                let frustum = Frustum::from_mat4(proj * view);

                // 뷰 프러스텀이 충돌하는 다른 플레이어 중 가장 가까운 플레이어를 선정합니다.
                let find = world
                    .players
                    .iter_mut()
                    .filter(|(_, target)| {
                        !target.is_invincible()
                            && target.action_state != ActionState::Retreat
                            && target.team() != data.team()
                    })
                    .filter(|(_, target)| {
                        let target_attributes = target.character_attributes();
                        let mut capsule = target_attributes.collider.clone();
                        capsule.center = target.translation.into();
                        frustum.capsule_test(&capsule)
                    })
                    .min_by(|(_, lhs), (_, rhs)| {
                        let lhs_dist = data.translation.distance_squared(lhs.translation);
                        let rhs_dist = data.translation.distance_squared(rhs.translation);
                        lhs_dist.total_cmp(&rhs_dist)
                    });

                // 대상이 존재하는 경우 데미지를 즉시 적용합니다.
                if let Some((&target_id, target)) = find {
                    let skill_multi = 0.78;
                    let s = &mut data;
                    let s_character_attributes = s.character_attributes();
                    let s_accuracy = s_character_attributes.accuracy_stat as f32;
                    let s_attack_pow = s_character_attributes.attack_power as f32;
                    let s_crit_rate = s_character_attributes.critical_rate as f32;
                    let s_crit_multi = s_character_attributes.critical_damage as f32 / 100.0;
                    let h = target;
                    let h_character_attributes = h.character_attributes();
                    let h_evasion = h_character_attributes.evasion_stat as f32;
                    let h_defense_pow = h_character_attributes.defense_power as f32;
                    let damage = Self::hit_player(
                        s,
                        s_accuracy,
                        s_attack_pow * skill_multi,
                        s_crit_rate,
                        s_crit_multi,
                        h,
                        h_evasion,
                        h_defense_pow,
                    );
                    self.damage_logs.push(DamageLogData::new(target_id, damage));
                }
            }
            CharacterKind::YuukaOriginal => {
                // 자신 체력의 30% 방어막을 팀원 전체에 부여합니다.
                // 이미 방어막이 존재하는 플레이어는 더 높은 방어막으로 적용됩니다. (더해지지 않음)
                let shield = data.health_data.num_maximum_health() as f32 * 0.3;
                let shield = shield.round() as u16;

                // 자신의 체력에 적용합니다.
                data.health_data.shield = data.health_data.shield.max(shield);

                // 팀원의 체력에 적용합니다.
                for other in world.players.values_mut() {
                    if other.team() == data.team() {
                        other.health_data.shield = other.health_data.shield.max(shield);
                    }
                }
            }
        }

        // 플레이어 데이터의 소유권을 돌려놓습니다.
        world.players.insert(uid, data);
    }

    /// 총알과 충돌하는 플레이어를 확인합니다.  
    /// 건물, 바닥 등과 충돌시에는 총알의 남은 거리를 0.0으로 설정하고 None을 리턴합니다.  
    /// 주어지는 속도는 0이 아니어야 합니다.  
    fn check_bullet_collision(
        stage_attributes: &StageAttributes,
        player: &HashMap<UserId, Player>,
        bullet: &mut Bullet,
        mut velocity: glam::Vec3A,
    ) -> Option<UserId> {
        let length = velocity.length();
        let direction = velocity / length;
        let translation = bullet.translation;
        let radius = bullet.radius;
        let mut nearest_distance = None;

        // 1. 지형과 충돌 검사
        let dist =
            Self::check_bullet_ground_collision(stage_attributes, translation, direction, length);
        if let Some(dist) = dist {
            nearest_distance = Some(dist);
            velocity = direction * dist;
            bullet.remaining_distance = 0.0;
        }

        if nearest_distance.is_some_and(|dist| dist <= EPSILON) {
            return None;
        }

        // 2. 건물과 충돌 검사
        let bullet_collider = Sphere {
            center: translation.into(),
            radius,
        };
        let dist =
            Self::check_bullet_building_collision(stage_attributes, &bullet_collider, velocity);
        if let Some(dist) = dist {
            match nearest_distance.as_mut() {
                Some(distance) => {
                    if *distance > dist {
                        *distance = dist;
                        velocity = direction * dist;
                    }
                }
                None => {
                    nearest_distance = Some(dist);
                    velocity = direction * dist;
                    bullet.remaining_distance = 0.0;
                }
            }
        }

        if nearest_distance.is_some_and(|dist| dist <= EPSILON) {
            return None;
        }

        // 3. 플레이어와 충돌 검사
        let mut player_uid = None;
        for (&uid, player) in player.iter() {
            if uid == bullet.shooter_id
                || player.health_data.remaining == 0
                || player.team() == bullet.shooter_team
                || player.is_invincible()
            {
                continue;
            }

            let character_attributes = player.character_attributes();
            let mut player_collider = character_attributes.collider.clone();
            player_collider.center = player.translation.into();

            let details =
                bullet_collider.check_dynamic_collision_details(&velocity, &player_collider);
            if let Some(details) = details {
                if details.distance <= length {
                    match nearest_distance.as_mut() {
                        Some(distance) => {
                            if *distance > details.distance {
                                *distance = details.distance;
                                player_uid = Some(uid);
                            }
                        }
                        None => {
                            nearest_distance = Some(details.distance);
                            bullet.remaining_distance = 0.0;
                            player_uid = Some(uid);
                        }
                    }
                }
            }
        }

        player_uid
    }

    /// 0.1m 마다 바닥과 충돌을 검사합니다.
    fn check_bullet_ground_collision(
        stage_attributes: &StageAttributes,
        mut translation: glam::Vec3A,
        direction: glam::Vec3A,
        length: f32,
    ) -> Option<f32> {
        let mut distance = None;
        let mut current = 0.0;
        while current < length {
            let height = stage_attributes.get_area_height(translation.x, translation.z);
            if let Some(height) = height {
                if translation.y <= height {
                    distance = Some(current);
                    break;
                }
            }
            translation += direction * 0.1;
            current += 0.1;
        }

        distance
    }

    /// 총알과 충돌하는 건물의 거리를 반환합니다.
    fn check_bullet_building_collision(
        stage_attributes: &StageAttributes,
        bullet_collider: &Sphere,
        velocity: glam::Vec3A,
    ) -> Option<f32> {
        let mut distance = None;
        let colliders = &stage_attributes.collider;
        for collider in ColliderTreeIterator::new(colliders) {
            // 1. broad phase 검사 - 시작지점과 도착지검을 포함하는 AABB 생성
            let rad_box = bullet_collider.radius * velocity.signum();
            let center = glam::Vec3A::from(bullet_collider.center);
            let start = center - rad_box;
            let end = center + velocity + rad_box;
            let swept_aabb = BoundingBox::from_start_end(start.into(), end.into());

            if collider.check_aabb_collision(&swept_aabb) {
                // 2. narrow phase 검사 - 총알과 충돌체의 충돌 검사
                let details = match collider {
                    Collider::Aabb(collider) => {
                        bullet_collider.check_dynamic_collision_details(&velocity, collider)
                    }
                    Collider::Obb(collider) => {
                        bullet_collider.check_dynamic_collision_details(&velocity, collider)
                    }
                    Collider::Capsule(collider) => {
                        bullet_collider.check_dynamic_collision_details(&velocity, collider)
                    }
                    Collider::OrientedCapsule(collider) => {
                        bullet_collider.check_dynamic_collision_details(&velocity, collider)
                    }
                    Collider::Sphere(collider) => {
                        bullet_collider.check_dynamic_collision_details(&velocity, collider)
                    }
                };

                if let Some(details) = details {
                    match distance.as_mut() {
                        Some(distance) => {
                            if *distance > details.distance {
                                *distance = details.distance;
                            }
                        }
                        None => {
                            distance = Some(details.distance);
                        }
                    }
                }
            }
        }

        distance
    }

    /// 플레이어 데미지 처리를 수행합니다.
    fn hit_player(
        s: &mut Player,
        s_accuracy: f32,
        s_attack_pow: f32,
        s_crit_rate: f32,
        s_crit_multi: f32,
        h: &mut Player,
        h_evasion: f32,
        h_defense_pow: f32,
    ) -> Damage {
        let uniform_dstrib = Uniform::new(0.0, 1.0).unwrap();

        // 1. 회피률 계산
        let hit_chance = s_accuracy / (s_accuracy + h_evasion);
        let rand_val = uniform_dstrib.sample(&mut rand::rng()) * 0.5;
        if rand_val > hit_chance {
            return Damage::Miss;
        }

        // 2. 치명타 판정
        let crit_chance = s_crit_rate / (s_crit_rate + h_evasion * 1.5);
        let rand_val = uniform_dstrib.sample(&mut rand::rng());
        let is_critical = rand_val <= crit_chance;

        // 3. 피해량 계산
        let mut damage = (s_attack_pow - h_defense_pow) * rand::random_range(0.9..=1.1);
        let mut result = Damage::Common(damage.clamp(1.0, 9999.0) as u16);
        if is_critical {
            damage = (damage * s_crit_multi).round();
            result = Damage::Critial(damage as u16);
        };
        let mut final_damage = damage.clamp(1.0, 9999.0) as u16;

        // 데이터 갱신
        s.damage_dealt = s.damage_dealt.saturating_add(final_damage as u32);
        h.damage_taken = h.damage_taken.saturating_add(final_damage as u32);
        if h.health_data.shield < final_damage {
            final_damage -= h.health_data.shield;
            h.health_data.shield = 0;
            if h.health_data.remaining <= final_damage {
                // 플레이어 행동 불능 처리
                h.health_data.remaining = 0;
                h.action_state = ActionState::Retreat;
                h.action_state_timer.0 = 0;
                h.movement_state = MovementState::Idle;
                h.movement_state_timer.0 = 0;
                h.action_notify = ActionNotify::Retreat;

                // 플레이 데이터 갱신
                h.retreat_count += 1;
                s.kill_count += 1;
            } else {
                h.health_data.remaining -= final_damage;
            }
        } else {
            h.health_data.shield -= final_damage;
        };

        // 결과 반환
        result
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &mut GameWorld) {
        // 경과 시간이 5초 미만인 경우 전환을 시도하지 않습니다.
        if self.play_elapsed_time_ms < 5_000 {
            return;
        }

        // 블루 팀 플레이어가 비어있는 경우 레드 팀을 부전승으로 처리합니다.
        if self.num_blue_players == 0 {
            // 다음 상태로 전환합니다.
            let winner = Some(Team::Red);
            let play_time_ms = self.play_elapsed_time_ms;
            let leaved_players = self.leaved_players.drain().collect();
            let removed_bullets = self.removed_bullets.drain().collect();
            let bullets = self.bullets.drain().collect();
            let state = GameWorldInGameFinishState::new(
                winner,
                play_time_ms,
                self.stage_kind,
                self.custom_game,
                self.half_size_x,
                self.half_size_y,
                self.half_size_z,
                leaved_players,
                removed_bullets,
                bullets,
            );

            let flow = GameWorldStateFlow::Change(Box::new(state));
            world.flows.push(flow);
        }
        // 레드 팀 플레이어가 비어있는 경우 블루 팀을 부전승으로 처리합니다.
        else if self.num_red_players == 0 {
            // 다음 상태로 전환합니다.
            let winner = Some(Team::Blue);
            let play_time_ms = self.play_elapsed_time_ms;
            let leaved_players = self.leaved_players.drain().collect();
            let removed_bullets = self.removed_bullets.drain().collect();
            let bullets = self.bullets.drain().collect();
            let state = GameWorldInGameFinishState::new(
                winner,
                play_time_ms,
                self.stage_kind,
                self.custom_game,
                self.half_size_x,
                self.half_size_y,
                self.half_size_z,
                leaved_players,
                removed_bullets,
                bullets,
            );

            let flow = GameWorldStateFlow::Change(Box::new(state));
            world.flows.push(flow);
        }

        // 게임 진행 시간을 초과한 경우
        if self.play_elapsed_time_ms >= MAX_GAME_TIME {
            // 현재 점령 점수가 높은 팀을 가져옵니다.
            let capture_point = self.capture_point.as_ref();
            let max_score_team = capture_point.max_score_team();

            // 다음 상태로 전환합니다.
            let winner = max_score_team;
            let play_time_ms = self.play_elapsed_time_ms;
            let leaved_players = self.leaved_players.drain().collect();
            let removed_bullets = self.removed_bullets.drain().collect();
            let bullets = self.bullets.drain().collect();
            let state = GameWorldInGameFinishState::new(
                winner,
                play_time_ms,
                self.stage_kind,
                self.custom_game,
                self.half_size_x,
                self.half_size_y,
                self.half_size_z,
                leaved_players,
                removed_bullets,
                bullets,
            );

            let flow = GameWorldStateFlow::Change(Box::new(state));
            world.flows.push(flow);
        } else {
            // 현재 점령 점수가 높은 팀을 가져옵니다.
            let capture_point = self.capture_point.as_ref();
            let max_score_team = capture_point.max_score_team();
            if let Some(team) = max_score_team {
                match team {
                    Team::Blue => {
                        // 최대 점령 점수를 달성한 경우 다음 상태로 전환합니다.
                        let percent = capture_point.blue_score();
                        if percent >= 1.0 {
                            // 다음 상태로 전환합니다.
                            let winner = Some(Team::Blue);
                            let play_time_ms = self.play_elapsed_time_ms;
                            let leaved_players = self.leaved_players.drain().collect();
                            let removed_bullets = self.removed_bullets.drain().collect();
                            let bullets = self.bullets.drain().collect();
                            let state = GameWorldInGameFinishState::new(
                                winner,
                                play_time_ms,
                                self.stage_kind,
                                self.custom_game,
                                self.half_size_x,
                                self.half_size_y,
                                self.half_size_z,
                                leaved_players,
                                removed_bullets,
                                bullets,
                            );

                            let flow = GameWorldStateFlow::Change(Box::new(state));
                            world.flows.push(flow);
                        }
                    }
                    Team::Red => {
                        // 최대 점령 점수를 달성한 경우 다음 상태로 전환합니다.
                        let percent = capture_point.red_score();
                        if percent >= 1.0 {
                            // 다음 상태로 전환합니다.
                            let winner = Some(Team::Red);
                            let play_time_ms = self.play_elapsed_time_ms;
                            let leaved_players = self.leaved_players.drain().collect();
                            let removed_bullets = self.removed_bullets.drain().collect();
                            let bullets = self.bullets.drain().collect();
                            let state = GameWorldInGameFinishState::new(
                                winner,
                                play_time_ms,
                                self.stage_kind,
                                self.custom_game,
                                self.half_size_x,
                                self.half_size_y,
                                self.half_size_z,
                                leaved_players,
                                removed_bullets,
                                bullets,
                            );

                            let flow = GameWorldStateFlow::Change(Box::new(state));
                            world.flows.push(flow);
                        }
                    }
                }
            }
        }
    }
}

impl GameWorldState for GameWorldInGameRunState {
    fn on_enter(&mut self, world: &mut GameWorld) {
        self.broadcast_pull_packet(world);
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
            GameWorldEvent::InGameRunState {
                session,
                uid,
                event,
            } => match event {
                GameWorldInGameRunStateEvent::Input {
                    client_play_elapsed_time,
                    snapshots,
                } => self.handle_input_event(
                    world,
                    session,
                    uid,
                    client_play_elapsed_time,
                    snapshots,
                ),
                GameWorldInGameRunStateEvent::InputReset => {
                    self.handle_input_reset_event(world, session, uid)
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

    fn on_advanced(&mut self, world: &mut GameWorld, elapsed: Duration) {
        let elapsed_time_ms = elapsed.as_millis().min(MAX_GAME_TIME as u128) as u32;

        // 플레이 경과 시간을 갱신합니다.
        self.play_elapsed_time_ms = self
            .play_elapsed_time_ms
            .saturating_add(elapsed_time_ms)
            .min(MAX_GAME_TIME);
        // 패킷 전송 경과 시간을 갱신합니다.
        self.pull_send_elapsed_time_ms = self
            .pull_send_elapsed_time_ms
            .saturating_add(elapsed_time_ms);
        self.status_send_elapsed_time_ms = self
            .status_send_elapsed_time_ms
            .saturating_add(elapsed_time_ms);

        // 게임 월드를 갱신합니다.
        self.update(world, elapsed);

        // 일정 시각마다 패킷을 전송합니다.
        const PULL_TICK: u32 = 6;
        if self.pull_send_elapsed_time_ms >= PULL_TICK {
            self.pull_send_elapsed_time_ms = 0;
            self.broadcast_pull_packet(world);
        }

        // 일정 시각마다 패킷을 전송합니다.
        const STATUS_TICK: u32 = 16;
        if self.status_send_elapsed_time_ms >= STATUS_TICK {
            self.status_send_elapsed_time_ms = 0;
            self.broadcast_status_packet(world);
        }

        self.try_enter_next_state(world);
    }
}
