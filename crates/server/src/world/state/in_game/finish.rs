use std::{f32::EPSILON, num::NonZeroU32, sync::Arc};

use ahash::{HashMap, HashSet};
use mod_network::{
    components::{
        HeldInput, InGameBulletPullData, InGamePlayerPullData, InGamePlayerResultData,
        MAX_IN_GAME_BULLETS, MAX_IN_GAME_PLAYERS, NetworkState, ObjectId, Permission,
        StageAttributes, StageKind, Team, UserId, update_action_state, update_action_state_timer,
        update_movement_state, update_movement_state_timer, update_player_rotation,
        update_player_translation,
    },
    protocol::{
        InGameFinishPacket, InGamePullPacket, JoinFailedReason, JoinRoomFailedPacket, Packet,
    },
};
use mod_physics::{
    collision::{Collider, ColliderTreeIterator, DynamicCollision},
    object3d::{BoundingBox, Sphere},
};
use rand::seq::SliceRandom;
use tokio::time::Duration;

use crate::{
    data::get_stage_attributes,
    entities::{Bullet, Player},
    session::{Session, SessionStateFlow},
    world::{GameWorld, GameWorldEvent, GameWorldState, GameWorldStateFlow, GameWorldSystemEvent},
};

/// 최대 게임 대기 시간 (단위: ms)
const MAX_WAIT_TIME: u32 = 17_500;

pub struct GameWorldInGameFinishState {
    /// 우승 팀
    winner: Option<Team>,
    /// 게임 플레이 시간
    play_time_ms: u32,

    /// 게임 스테이지 종류
    stage_kind: StageKind,
    /// 커스텀 게임 여부
    custom_game: bool,
    /// 게임 플레이 경과 시간
    play_elapsed_time_ms: u32,
    /// 마지막 Pull 패킷 전송 경과 시간
    pull_send_elapsed_time_ms: u32,

    /// x축 방향의 게임 월드 절반 크기
    half_size_x: NonZeroU32,
    /// y축 방향의 게임 월드 절반 크기
    half_size_y: NonZeroU32,
    /// z축 방향의 게임 월드 절반 크기
    half_size_z: NonZeroU32,

    /// 떠난 플레이어 식별자입니다.
    leaved_players: HashSet<UserId>,
    /// 제거된 총알 오브젝트 목록
    removed_bullets: HashSet<ObjectId>,

    /// 총알 오브젝트
    bullets: HashMap<ObjectId, Bullet>,
}

impl GameWorldInGameFinishState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(
        winner: Option<Team>,
        play_time_ms: u32,
        stage_kind: StageKind,
        custom_game: bool,
        half_size_x: NonZeroU32,
        half_size_y: NonZeroU32,
        half_size_z: NonZeroU32,
        leaved_players: HashSet<UserId>,
        removed_bullets: HashSet<ObjectId>,
        bullets: HashMap<ObjectId, Bullet>,
    ) -> Self {
        Self {
            winner,
            play_time_ms,
            stage_kind,
            custom_game,
            play_elapsed_time_ms: 0,
            pull_send_elapsed_time_ms: 0,
            half_size_x,
            half_size_y,
            half_size_z,
            leaved_players,
            removed_bullets,
            bullets,
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

    /// 모든 세션에 Pull 패킷 데이터를 전송합니다.
    fn broadcast_pull_packet(&mut self, world: &GameWorld) {
        // 플레이어 데이터를 수집합니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (&uid, data) in world.players.iter() {
            players.push(InGamePlayerPullData::new(
                uid,
                self.half_size_x,
                self.half_size_y,
                self.half_size_z,
                data.translation,
                data.rotation,
                data.action_state,
                data.action_state_timer,
                data.movement_state,
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
}

impl GameWorldInGameFinishState {
    /// 게임 월드를 갱신합니다.
    fn update(&mut self, world: &mut GameWorld, elapsed: Duration) {
        let elapsed_time_ms = elapsed.as_millis().min(u16::MAX as u128) as u16;

        // 모든 플레이어와 오브젝트 데이터를 현재 경과 시간 만큼 갱신합니다.
        self.update_player(world, elapsed_time_ms);
        self.update_bullet(world, elapsed_time_ms);
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

            // 입력 상태 타이머를 갱신합니다.
            player
                .input_state_timer
                .update(HeldInput::empty(), elapsed_time_ms);

            // 행동 상태 타이머를 갱신합니다.
            let character_attributes = player.character_attributes();
            update_action_state_timer(
                HeldInput::empty(),
                &mut player.bullet_data,
                &mut player.skill_cost_data,
                &mut player.action_state,
                &mut player.action_state_timer,
                character_attributes,
                elapsed_time_ms,
                &mut vec![],
            );

            // 움직임 상태 타이머를 갱신합니다.
            update_movement_state_timer(
                player.action_state,
                &mut player.movement_state,
                &mut player.movement_state_timer,
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
                HeldInput::empty(),
                team,
                &mut is_grounded,
                &mut is_invincible,
                Some(&mut player.health_data),
                player.input_state_timer,
                elapsed_time_sec,
            );
            player.set_grounded(is_grounded);
            player.set_invincible(is_invincible);
        }
    }

    fn update_bullet(&mut self, world: &mut GameWorld, elapsed_time_ms: u16) {
        let elapsed_time_sec = elapsed_time_ms as f32 / 1000.0;
        let stage_attributes = get_stage_attributes(self.stage_kind);

        // 총알 오브젝트를 갱신합니다.
        let mut removed = HashSet::default();
        for (&id, bullet) in self.bullets.iter_mut() {
            let velocity = bullet.velocity * elapsed_time_sec;
            let player_uid =
                Self::check_bullet_collision(stage_attributes, &world.players, bullet, velocity);
            if player_uid.is_none() {
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
        let dist = Self::check_bullet_ground_collision(
            stage_attributes,
            translation,
            direction,
            length,
            radius,
        );
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

        player_uid
    }

    /// 0.1m 마다 바닥과 충돌을 검사합니다.
    fn check_bullet_ground_collision(
        stage_attributes: &StageAttributes,
        mut translation: glam::Vec3A,
        direction: glam::Vec3A,
        length: f32,
        radius: f32,
    ) -> Option<f32> {
        let mut distance = None;
        let mut current = 0.0;
        while current < length {
            let height = stage_attributes.get_area_height(translation.x, translation.z);
            if let Some(height) = height {
                if translation.y <= height + radius {
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

    fn try_enter_next_state(&mut self, world: &mut GameWorld) {
        // 경과 시간이 대기 시간을 초과하지 않은 경우 전환을 시도하지 않습니다.
        if self.play_elapsed_time_ms < MAX_WAIT_TIME {
            return;
        }

        // 모든 세션 상태를 이전 상태로 되돌립니다.
        for session in world.sessions.keys() {
            session.add_flow(SessionStateFlow::Pop);
        }

        // 이전 게임 월드 상태로 돌아갑니다.
        let flow = GameWorldStateFlow::Pop;
        world.flows.push(flow);
    }
}

impl GameWorldState for GameWorldInGameFinishState {
    fn on_enter(&mut self, world: &mut GameWorld) {
        let mut players = Vec::with_capacity(world.players.len());
        for (&uid, player) in world.players.iter_mut() {
            // 플레이어의 입력을 초기화합니다.
            player.held_input = HeldInput::empty();

            // 플레이어 상태를 갱신합니다.
            let character_attributes = player.character_attributes();
            update_action_state(
                player.held_input,
                &mut player.action_state,
                &mut player.action_state_timer,
                character_attributes,
                &mut player.bullet_data,
                &mut player.skill_cost_data,
            );
            update_movement_state(
                player.held_input,
                player.action_state,
                &mut player.movement_state,
                &mut player.movement_state_timer,
            );

            // 플레이어 데이터를 수집합니다.
            let is_connected = !self.leaved_players.contains(&uid);
            players.push(InGamePlayerResultData::new(
                uid,
                player.name,
                player.profile_icon,
                player.character_kind(),
                player.kill_count,
                player.retreat_count,
                player.damage_dealt,
                player.damage_taken,
                player.healing_given,
                is_connected,
                0,
                player.team(),
                player.team_index(),
                player.tier(),
            ));
        }

        // 게임 종료 패킷을 전송합니다.
        let packet = InGameFinishPacket::new(self.play_time_ms, self.winner, players);
        for session in world.sessions.keys() {
            session.tcp_write(packet.as_raw());
        }
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
        // 남은 시간을 갱신합니다.
        let elapsed_time_ms = elapsed.as_millis().min(MAX_WAIT_TIME as u128) as u32;

        // 플레이 경과 시간을 갱신합니다.
        self.play_elapsed_time_ms = self
            .play_elapsed_time_ms
            .saturating_add(elapsed_time_ms)
            .min(MAX_WAIT_TIME);
        // 패킷 전송 경과 시간을 갱신합니다.
        self.pull_send_elapsed_time_ms = self
            .pull_send_elapsed_time_ms
            .saturating_add(elapsed_time_ms);

        // 게임 월드를 갱신합니다.
        self.update(world, elapsed);

        // 일정 시각마다 패킷을 전송합니다.
        const PULL_TICK: u32 = 5;
        if self.pull_send_elapsed_time_ms >= PULL_TICK {
            self.pull_send_elapsed_time_ms = 0;
            self.broadcast_pull_packet(world);
        }

        self.try_enter_next_state(world);
    }
}
