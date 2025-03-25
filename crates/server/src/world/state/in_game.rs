use std::{
    fmt,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use mod_network::{
    components::{
        DamageLog, HealthPoint, MovementState, ObjectId, PlayPhasePlayer, StageKind, UserId,
    },
    protocol::{Packet, PullStagePacket, UdpDamageLogPacket},
};
use mod_parallelism::collections::Queue;
use mod_physics::{
    collision::{Collider, ColliderTreeIterator, DynamicCollision, StaticCollision},
    object3d::{BoundingBox, Sphere},
};
use tokio::time::{Duration, Instant};

use crate::{
    data::{clamp_x, clamp_z, get_stage_colliders, get_stage_height, is_valid_position},
    formula::movement_formulas as formulas,
    world::{GameWorld, GameWorldEvent},
};

use super::GameWorldState;

/// 중력 가속도입니다.
const GRAVITY: glam::Vec3A = glam::vec3a(0.0, -9.8, 0.0);

pub struct GameWorldInGameState {
    /// 게임 월드 상태 실행 여부
    is_running: bool,
    /// 이전 측정 시각
    previous_time_pt: Instant,
    /// 오브젝트 식별자를 생성하기 위한 카운터입니다.
    counter: u32,

    /// 게임 스테이지 종류
    stage_kind: StageKind,

    /// 플레이어 데미지 로그입니다.
    damage_logs: Queue<DamageLog>,
}

impl GameWorldInGameState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(stage_kind: StageKind) -> Self {
        Self {
            is_running: true,
            previous_time_pt: Instant::now(),
            counter: 0,
            stage_kind,
            damage_logs: Queue::new(),
        }
    }

    /// 오브젝트 식별자를 생성합니다.
    pub fn generate_object_id(&mut self) -> ObjectId {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

        self.counter += 1;
        let counter_bit = self.counter & 0xFFFF;
        let time_bit = duration.subsec_nanos() & 0xFFFF;

        ObjectId::new((time_bit << 16) | counter_bit)
    }

    /// 플레이어 상태 타이머를 갱신합니다.
    fn update_player_state_timer(&self, world: &GameWorld, elapsed_time_sec: f32) {
        for mut player in world.players.iter_mut() {
            player.update_state_timer(world, elapsed_time_sec);
        }
    }

    /// 주어진 시간 간격으로 플레이어의 위치를 갱신합니다.
    fn update_player_position(&self, world: &GameWorld, elapsed_time_sec: f32) {
        for mut player in world.players.iter_mut() {
            // 플레이어 위치를 가져옵니다.
            let translation = player.translation();

            // 총 가속도를 계산합니다.
            let mut acceleration = GRAVITY;
            acceleration += player.acceleration();

            // 플레이어 속도를 갱신합니다.
            player.update_velocity();

            // 속도에 가속도를 적용합니다.
            let velocity = player.velocity_mut();
            *velocity += acceleration * elapsed_time_sec;

            // 이동 시도 (이동 전 위치 저장)
            let mut new_p = translation + (*velocity) * elapsed_time_sec;

            // 기존 영역과 현재 영역을 인자로 넘겨서 x, z중 어느 값이 넘어갔는지 확인
            // 아니면 x만 이동했을때의 영역과 z만 이동했을때의 영역을 보고, 유효한 영역일때만 이동시키도록?
            // 유효한 영역이 아니라면 현재 영역의 가장 가장자리 부분으로 clamp하기
            if !is_valid_position(self.stage_kind, new_p.x, translation.z) {
                velocity.x = 0.0;
                new_p.x = clamp_x(self.stage_kind, translation.x, new_p.x);
            }
            if !is_valid_position(self.stage_kind, translation.x, new_p.z) {
                velocity.z = 0.0;
                new_p.z = clamp_z(self.stage_kind, translation.z, new_p.z);
            }

            new_p = translation + (*velocity) * elapsed_time_sec;

            if let Some(height) = get_stage_height(self.stage_kind, new_p.x, new_p.z) {
                if height >= new_p.y {
                    new_p.y = height;
                    velocity.y = 0.0;

                    let current = player.movement_state();
                    if current == MovementState::InPlaceLanding {
                        player.change_movement_state(MovementState::Idle);
                    } else if current == MovementState::MovingLanding {
                        player.change_movement_state(MovementState::Moving);
                    }
                }
                *player.translation_mut() = new_p;
                player.update_collider();
            }
        }
    }

    /// 게임 세상에 총알 오브젝트를 추가합니다.
    fn add_bullet(&mut self, world: &GameWorld, shooter_id: UserId, delay: f32) {
        // 총알을 발사한 플레이어 정보를 가져옵니다.
        let player = match world.players.get(&shooter_id) {
            Some(player) => player,
            None => {
                log::warn!(
                    "failed to create bullet. (REASON:the Player({}) could not be found in {:?})",
                    &shooter_id,
                    &world
                );
                return;
            }
        };

        let object_id = self.generate_object_id();
        let bullet = player.generate_bullet(object_id, delay);
        world.bullets.insert(object_id, bullet);
        log::info!(
            "the Player({}) fires a Bullet({}) into the {:?}",
            &shooter_id,
            &object_id,
            &world
        );
    }

    /// 게임 세상에서 총알 오브젝트를 제거합니다.
    fn remove_bullet(&self, world: &GameWorld, object_id: ObjectId) {
        match world.bullets.remove(&object_id) {
            Some(_) => log::info!("Bullet({}) is removed from the {:?}", &object_id, &world),
            None => log::warn!(
                "the Bullet({}) could not be found in {:?}!",
                &object_id,
                &world
            ),
        };
    }

    /// 게임 월드를 갱신합니다.
    fn update(&mut self, world: &GameWorld) {
        let current_time_pt = Instant::now();
        let elapsed_time_sec = current_time_pt
            .saturating_duration_since(self.previous_time_pt)
            .as_secs_f32();
        self.previous_time_pt = current_time_pt;

        self.update_player_state_timer(world, elapsed_time_sec);
        self.update_player_position(world, elapsed_time_sec);

        let colliders = get_stage_colliders(self.stage_kind);
        let colliders_iter = ColliderTreeIterator::new(colliders);
        for collider in colliders_iter {
            let aabb = match collider {
                Collider::Aabb(aabb) => aabb,
                Collider::Obb(obb) => &BoundingBox::from(obb),
                Collider::Capsule(capsule) => &BoundingBox::from(capsule),
                Collider::OrientedCapsule(ocapsule) => &BoundingBox::from(ocapsule),
                Collider::Sphere(sphere) => &BoundingBox::from(sphere),
            };
            for mut player in world.players.iter_mut() {
                let player_capsule = player.collider();
                let player_aabb = BoundingBox::from(player_capsule);
                if !aabb.check_static_collision(&player_aabb) {
                    continue;
                }

                let player_collider = Collider::Capsule(player_capsule.clone());
                if let Some(collision_info) = player_collider.check_collision_details(collider) {
                    *player.translation_mut() += collision_info.normal * collision_info.penetration;
                }
            }
        }

        // 총알 이동
        for mut bullet in world.bullets.iter_mut() {
            let translation = bullet.translation;
            let direction = bullet.velocity * elapsed_time_sec;
            let move_distance = direction.length();

            // bullet.velocity가 영벡터가 아니라고 가정
            // TODO: 총알에 반지름값 추가해야함 - 플레이어가 총을 쏜 후 나가면 shooter_id로 플레이어를 가져올 수 없음 -> panic
            let bullet_radius = world
                .players
                .get(&bullet.shooter_id)
                .unwrap()
                .character_attributes()
                .bullet_radius;
            let bullet_collider = Sphere {
                center: translation.into(),
                radius: bullet_radius,
            };

            let mut nearest_distance = f32::MAX;
            let mut nearest_player_id = None;

            for player in world.players.iter() {
                if *player.key() == bullet.shooter_id {
                    continue;
                }

                let player_collider = player.collider();

                // 충돌 처리: 플레이어 - 총알
                if let Some(info) = bullet_collider
                    .check_dynamic_collision_details(&bullet.velocity, &player_collider)
                {
                    if info.distance <= move_distance {
                        println!("Bullet find player (player id: {:?})", player.account().uid);
                        println!("  - distance: {}", info.distance);
                        println!("  - surface normal: {:?}", info.normal);
                        if info.distance < nearest_distance {
                            nearest_distance = info.distance;
                            nearest_player_id = Some(*player.key());
                        }
                    }
                }
            }

            match nearest_player_id {
                // 충돌했다면
                Some(id) => {
                    // 피격 처리(회피하더라도 일단 총알은 제거)
                    bullet.remaining_distance = 0.0;

                    println!("Player {:?} hit by bullet", id);
                    let mut player = world.players.get_mut(&id).unwrap();
                    let char_info = player.character_attributes();

                    //발포자 정보
                    let shooter = world.players.get_mut(&bullet.shooter_id).unwrap();
                    let shooter_info = shooter.character_attributes();

                    // 각 식에서의 상수값은 제안서에 있는 값으로 설정

                    // 1. 회피 계산
                    // 2. 기본 데미지 계산
                    // 3. 치명타 계산
                    // 4. 최종 데미지 계산

                    // 회피 계산
                    let accuracy = char_info.accuracy_stat as f32;
                    let evasion = char_info.evasion_stat as f32;
                    let hit_rate = formulas::cal_hit_rate(accuracy, evasion, 100.0);
                    // if rand::random::<f64>() > hit_rate {
                    //     println!("  - miss");
                    //     continue;
                    // }

                    // 데미지 계산
                    let def = char_info.defense_power as f32;

                    //기존: let atk = char_info.attack_power as f32;
                    let atk = shooter_info.attack_power as f32; //발포자의 공격력 수치여야 하는거아닌가?
                    let dur = shooter_info.normal_attack_ing_duration as f32;
                    let cnt = shooter_info.normal_attack_count as f32;
                    let dmg = formulas::default_damage(atk, def, 100.0, dur, cnt);

                    // 치명타 계산
                    //기존: let crit = char_info.critical_rate as f32;
                    let crit = shooter_info.critical_rate as f32; //발포자의 치명 수치여야 하는거아닌가?
                    let crit_rate = formulas::cal_crt_rate(rand::random::<f32>(), crit, 250.0);
                    if crit_rate == 1.0 {
                        println!("  - critical!");
                    }

                    // 최종 데미지 계산
                    //기존: let crit_dam = char_info.critical_damage as f32;
                    let crit_dam = shooter_info.critical_damage as f32; //발포자의 치명 수치여야 하는거아닌가?
                    let final_dmg =
                        formulas::final_damage(dmg, hit_rate, crit_rate, crit_dam).ceil() as u16;

                    let health_point = player.health_point_mut();
                    health_point.0 = (health_point.0 - final_dmg).max(0);
                    println!("  - hp: {:?}(-{})", health_point.0, final_dmg);

                    self.damage_logs.push(DamageLog {
                        user_id: player.account().uid,
                        damage: HealthPoint(final_dmg),
                    });
                }

                // 충돌하지 않았다면
                None => {
                    // 누적 이동거리 증가
                    bullet.remaining_distance -= move_distance;

                    // println!("range: {}, moved: {}", bullet.blob.range, bullet.moved_distance);

                    // 총알 사거리를 넘어가면 총알 제거
                    if bullet.remaining_distance <= 0.0 {
                        println!("Bullet range over");
                    } else {
                        // 총알 위치 이동
                        bullet.translation = translation + direction;
                    }
                }
            }
        }

        // 살아남은 총알만 남김
        for bullet in world.bullets.iter() {
            if bullet.remaining_distance <= 0.0 {
                world.push_event(GameWorldEvent::RemoveBullet(*bullet.key()));
            }
        }
    }

    /// 모든 세션 데이터에 패킷을 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let players: Vec<_> = world
            .players
            .iter()
            .map(|player| {
                PlayPhasePlayer::new(
                    player.account().clone(),
                    player.character_kind(),
                    player.health_point(),
                    player.translation().to_array(),
                    player.rotation().to_array(),
                    player.team(),
                    player.action_state(),
                    player.action_state_timer(),
                    player.movement_state(),
                    player.movement_state_timer(),
                    player.view_state(),
                    player.view_state_timer(),
                    player.view_rotation(),
                )
            })
            .collect();
        let bullets: Vec<_> = world
            .bullets
            .iter()
            .map(|bullet| bullet.as_bullet())
            .collect();

        // 패킷을 생성하고 전송합니다.
        let capacity = UdpDamageLogPacket::capacity();
        loop {
            let mut logs = Vec::with_capacity(capacity);
            for _ in 0..capacity {
                if let Some(log) = self.damage_logs.pop() {
                    logs.push(log);
                } else {
                    break;
                }
            }

            if !logs.is_empty() {
                let packet = UdpDamageLogPacket::new(logs);
                for session in world.sessions.iter() {
                    session.key().tcp_write(packet.as_raw());
                }
            } else {
                break;
            }
        }

        // 패킷을 생성하고 전송합니다.
        let packet = PullStagePacket::new(players, bullets);
        for session in world.sessions.iter() {
            session.key().tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldState for GameWorldInGameState {
    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
        match event {
            GameWorldEvent::AddBullet { shooter_id, delay } => {
                self.add_bullet(world, shooter_id, delay);
            }
            GameWorldEvent::RemoveBullet(object_id) => {
                self.remove_bullet(world, object_id);
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

        self.update(world);
        self.broadcast(world);
    }

    fn yield_now(&self) {
        std::thread::sleep(Duration::from_millis(1));
    }
}

impl fmt::Debug for GameWorldInGameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameState))
    }
}
