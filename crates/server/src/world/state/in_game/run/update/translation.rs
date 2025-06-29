//! 게임 월드 위치 갱신과 관련된 코드를 관리합니다.
//!

use core::f32;

use mod_network::components::{
    ActionState, BulletKind, CharacterAttributes, Damage, DamageLogData, MAX_DAMAGE,
    MAX_INPUT_STATE_TIME, MovementState, ObjectId, StageKind, UserId,
};
use mod_physics::{
    collision::{Collider, ColliderTreeIterator, DynamicCollision},
    object3d::{BoundingBox, Sphere},
};
use rand::{random, random_range};
use tokio::time::Duration;

use crate::{
    data::{
        get_nearest_valid_position, get_stage_colliders, get_stage_height, is_safe_area,
        is_valid_position,
    },
    entities::{Bullet, Player},
    world::{GameWorld, GameWorldEvent, GameWorldInGameRunStateEvent},
};

const GROUNDED_ANGLE: f32 = 45f32.to_radians();

/// 플레이어 위치를 갱신합니다.
pub fn update_player_translation(stage_kind: StageKind, data: &mut Player, elapsed: Duration) {
    let elapsed_time_sec = elapsed.as_secs_f32();

    // 플레이어의 현재 위치를 가져옵니다.
    let translation = data.translation;

    // 플레이어 속도를 갱신후 가져옵니다.
    update_player_velocity(data);
    let mut velocity = data.velocity;

    // 중력 가속도를 적용합니다.
    if !data.is_grounded() {
        const GRAVITY: f32 = -9.8;
        velocity.y = velocity.y * GRAVITY * elapsed_time_sec;
    }

    // 새로운 위치를 계산합니다.
    let mut new_translation = translation + velocity * elapsed_time_sec;

    // ------ 충돌 처리를 수행합니다. --------
    data.set_grounded(false);

    let mut player_capsule = data.character_attributes().collider.clone();
    player_capsule.center = new_translation.into();
    let player_aabb = BoundingBox::from(&player_capsule);
    let player_collider = Collider::Capsule(player_capsule);

    let colliders = get_stage_colliders(stage_kind);
    for collider in ColliderTreeIterator::new(colliders) {
        if !collider.check_aabb_collision(&player_aabb) {
            continue;
        }
        if let Some(collision_info) = player_collider.check_collision_details(collider) {
            new_translation += collision_info.normal * collision_info.penetration;
            // 충돌벡터가 지면(xz평면)과 일정 이상의 각을 이루면 서있을 수 있음
            if collision_info.normal.y >= GROUNDED_ANGLE.cos() {
                velocity.y = 0.0;
                data.set_grounded(true);
            }
            // 아니라면 미끄러지도록 처리
            else {
                let slide = velocity - collision_info.normal * velocity.dot(collision_info.normal);
                // +y방향으로 튀어오르지 않게 한다.
                let vy = if slide.y < velocity.y {
                    slide.y
                } else {
                    velocity.y
                };
                velocity = glam::Vec3A::new(slide.x, vy, slide.z);
            }
        }
    }

    let team = data.team();

    if !is_valid_position(stage_kind, team, new_translation.x, new_translation.z) {
        let (x, z) =
            get_nearest_valid_position(stage_kind, team, new_translation.x, new_translation.z);
        if x != new_translation.x {
            velocity.x = 0.0;
            new_translation.x = x;
        }
        if z != new_translation.z {
            velocity.z = 0.0;
            new_translation.z = z;
        }
    }

    if let Some(height) = get_stage_height(stage_kind, new_translation.x, new_translation.z) {
        if height >= new_translation.y {
            new_translation.y = height;
            velocity.y = 0.0;
            data.set_grounded(true);
        }
    }

    let in_safe_area = is_safe_area(stage_kind, team, new_translation.x, new_translation.z);
    data.set_invincible(in_safe_area);
    if in_safe_area {
        data.current_health = data.maximum_health();
    }

    if data.is_grounded() {
        let movement_state = data.player_states.movement_state();
        match movement_state {
            MovementState::Jumping => {
                velocity.y = 5.0;
            }
            MovementState::Landing => {
                data.player_states
                    .set_movement_state(data.prev_movement_state);
                data.prev_movement_state = MovementState::Landing;
                data.movement_state_timer.0 = 0;
            }
            _ => {}
        }
    }

    data.velocity = velocity;
    data.translation = new_translation;
}

/// 플레이어 속도를 갱신합니다.
fn update_player_velocity(data: &mut Player) {
    // 플레이어의 속도를 갱신합니다.
    let action_state = data.player_states.action_state();
    let movement_state = data.player_states.movement_state();
    match action_state {
        ActionState::Idle | ActionState::Death => match movement_state {
            MovementState::Idle => {
                update_player_velocity_when_idle(data);
            }
            MovementState::Moving => {
                update_player_velocity_when_moving(data);
            }
            MovementState::MoveToEnd => {
                update_player_velocity_when_move_to_end(data);
            }
            _ => {}
        },
        ActionState::Aiming | ActionState::Attack | ActionState::Skill | ActionState::Reload => {
            match movement_state {
                MovementState::Idle => {
                    update_player_velocity_when_idle(data);
                }
                MovementState::Moving => {
                    update_player_velocity_when_walking(data);
                }
                MovementState::MoveToEnd => {
                    update_player_velocity_when_move_to_end(data);
                }
                _ => {}
            }
        }
        ActionState::AimAt => match movement_state {
            MovementState::Idle => {
                update_player_velocity_when_idle(data);
            }
            MovementState::Moving => {
                update_player_velocity_when_move_to_aim_move(data);
            }
            MovementState::MoveToEnd => {
                update_player_velocity_when_move_to_end(data);
            }
            _ => {}
        },
        ActionState::AimOff => match movement_state {
            MovementState::Idle => {
                update_player_velocity_when_idle(data);
            }
            MovementState::Moving => {
                update_player_velocity_when_aim_move_to_move(data);
            }
            MovementState::MoveToEnd => {
                update_player_velocity_when_move_to_end(data);
            }
            _ => {}
        },
        _ => {}
    }
}

/// [`ActionState::Idle`]일 때 플레이어의 속도를 갱신합니다.
fn update_player_velocity_when_idle(data: &mut Player) {
    // ActionState::Idle일 때 InputStateTimer가 0이 아닌 경우
    // => Walking 상태에서 정지한 상태
    //
    let s = data.input_state_timer.0 as f32 / MAX_INPUT_STATE_TIME as f32;
    let delta = s * s * (3.0 - 2.0 * s);
    let speed = 0.5 * data.character_attributes().speed * delta;
    let direction = data
        .velocity
        .normalize_or(data.rotation.mul_vec3a(glam::Vec3A::Z));
    data.velocity.x = direction.x * speed;
    data.velocity.z = direction.z * speed;
}

/// [`ActionState::Idle`] 이 아니고, [`MovementState::Moving`]일 때 플레이어의 속도를 갱신합니다.
fn update_player_velocity_when_walking(data: &mut Player) {
    let s = data.input_state_timer.0 as f32 / MAX_INPUT_STATE_TIME as f32;
    let delta = s * s * (3.0 - 2.0 * s);
    let speed = 0.5 * data.character_attributes().speed * delta;
    let direction = data
        .velocity
        .normalize_or(data.rotation.mul_vec3a(glam::Vec3A::Z));
    data.velocity.x = direction.x * speed;
    data.velocity.z = direction.z * speed;
}

/// [`ActionState::Idle`] 이고, [`MovementState::Moving`]일 때 플레이어의 속도를 갱신합니다.
fn update_player_velocity_when_moving(data: &mut Player) {
    let s = data.input_state_timer.0 as f32 / MAX_INPUT_STATE_TIME as f32;
    let delta = s * s * (3.0 - 2.0 * s);
    let speed = data.character_attributes().speed * delta;
    let direction = data
        .velocity
        .normalize_or(data.rotation.mul_vec3a(glam::Vec3A::Z));
    data.velocity.x = direction.x * speed;
    data.velocity.z = direction.z * speed;
}

/// [`ActionState::Idle`] 이고, [`MovementState::MoveToEnd`]일 때 플레이어의 속도를 갱신합니다.
fn update_player_velocity_when_move_to_end(data: &mut Player) {
    let s = data.input_state_timer.0 as f32 / MAX_INPUT_STATE_TIME as f32;
    let delta = s * s * (3.0 - 2.0 * s);
    let speed = data.character_attributes().speed * delta;
    let direction = data
        .velocity
        .normalize_or(data.rotation.mul_vec3a(glam::Vec3A::Z));
    data.velocity.x = direction.x * speed;
    data.velocity.z = direction.z * speed;
}

/// [`ActionState::AimAt`] 이고, [`MovementState::Moving`]일 때 플레이어 속도를 갱신합니다.
fn update_player_velocity_when_move_to_aim_move(data: &mut Player) {
    let duration = data.character_attributes().normal_attack_start_duration;
    let s = 1.0 - data.movement_state_timer.0 as f32 / duration as f32;
    let delta = 0.5 + 0.5 * s;
    let speed = data.character_attributes().speed * delta;
    let direction = data
        .velocity
        .normalize_or(data.rotation.mul_vec3a(glam::Vec3A::Z));
    data.velocity.x = direction.x * speed;
    data.velocity.z = direction.z * speed;
}

/// [`ActionState::AimOff`] 이고, [`MovementState::Moving`]일 때 플레이어 속도를 갱신합니다.
fn update_player_velocity_when_aim_move_to_move(data: &mut Player) {
    let duration = data.character_attributes().normal_attack_end_duration;
    let s = data.movement_state_timer.0 as f32 / duration as f32;
    let delta = 0.5 + 0.5 * s;
    let speed = data.character_attributes().speed * delta;
    let direction = data
        .velocity
        .normalize_or(data.rotation.mul_vec3a(glam::Vec3A::Z));
    data.velocity.x = direction.x * speed;
    data.velocity.z = direction.z * speed;
}

/// 총알의 위치를 갱신합니다.
pub fn update_bullet_translation(
    stage_kind: StageKind,
    world: &mut GameWorld,
    id: ObjectId,
    data: &mut Bullet,
    elapsed: Duration,
) -> Option<DamageLogData> {
    // 플레이어 데이터가 없는 경우 처리 (비정상)
    if !world.players.contains_key(&data.shooter_id) {
        log::error!("Player({}) data not found in {}!", &data.shooter_id, &world);
        println!("Player({}) data not found in {}!", &data.shooter_id, &world);
        return None;
    }

    let mut damage_log_data = None;
    let elapsed_time_sec = elapsed.as_secs_f32();
    let translate = data.velocity * elapsed_time_sec;
    let result = check_bullet_collision(stage_kind, id, data, world.players.iter(), translate);
    match result {
        Some(target_id) => {
            // Safety: GameWorld에 플레이어 데이터가 존재하는지 미리 확인함. AND GameWorld는 mutable borrow
            let shooter_attr = unsafe { world.players.get(&data.shooter_id).unwrap_unchecked() };
            let shooter_attr = shooter_attr.character_attributes();
            let target_attr = unsafe { world.players.get(&target_id).unwrap_unchecked() };
            let target_attr = target_attr.character_attributes();
            let damage = bullet_hit_player(data, shooter_attr, target_attr, target_id);

            // 총알을 쏜 플레이어의 스킬 코스트를 증가시킵니다.
            let shooter = unsafe { world.players.get_mut(&data.shooter_id).unwrap_unchecked() };
            shooter.current_skill_cost = shooter
                .current_skill_cost
                .saturating_add(10)
                .min(shooter.maximum_skill_cost());

            // 대상의 체력을 감소시킵니다.
            let mut diff_damage = match damage {
                Damage::Miss => 0,
                Damage::Common(damage) => damage,
                Damage::Critial(damage) => damage,
            };
            let target = unsafe { world.players.get_mut(&target_id).unwrap_unchecked() };
            if diff_damage > 0 && target.guard_health > 0 {
                let guard_health = target.guard_health as i32 - diff_damage as i32;
                if guard_health < 0 {
                    diff_damage -= target.guard_health;
                } else {
                    diff_damage = 0;
                }
            }
            if diff_damage > 0 && target.current_health > 0 {
                let health = target.current_health as i32 - diff_damage as i32;
                if health <= 0 {
                    // 사망처리
                    target.current_health = 0;
                    target.prev_action_state = ActionState::Idle;
                    target.player_states.set_action_state(ActionState::Death);
                } else {
                    target.current_health -= diff_damage;
                }
            }

            damage_log_data = Some(DamageLogData::new(target_id, damage));
        }
        _ => {
            if data.remaining_distance > 0.0 {
                data.translation += translate;
                data.remaining_distance = data.remaining_distance - translate.length();
            }
        }
    };

    damage_log_data
}

/// 총알과 충돌하는 플레이어를 확인합니다.  
/// 건물, 바닥 등과 충돌시에는 총알의 남은 거리를 0.0으로 설정하고 None을 리턴합니다.  
/// 움직일 벡터(move_v)는 0이 아니어야 합니다.  
fn check_bullet_collision<'a, I>(
    stage_kind: StageKind,
    id: ObjectId,
    data: &mut Bullet,
    players: I,
    mut translate: glam::Vec3A,
) -> Option<UserId>
where
    I: Iterator<Item = (&'a UserId, &'a Player)>,
{
    let distance = translate.length();
    let direction = translate / distance.max(f32::EPSILON);

    // 지형과 총알이 충돌하는지 검사합니다.
    let result = check_bullet_ground_collision(
        stage_kind,
        data.translation,
        data.radius,
        direction,
        distance,
    );

    let mut nearest_distance = f32::MAX;
    if let Some(collision_distance) = result {
        nearest_distance = collision_distance;
        translate = direction * nearest_distance;
        data.remaining_distance = 0.0;
        log::debug!("Bullet({}) hit fround (distance:{})", &id, nearest_distance,);
    }

    if nearest_distance <= 0.0 {
        return None;
    }

    // 건물과 총알이 충돌하는지 검사합니다.
    let bullet_collider = Sphere {
        center: data.translation.into(),
        radius: data.radius,
    };
    let result = check_bullet_building_collision(stage_kind, &bullet_collider, translate);
    if let Some(collision_distance) = result {
        nearest_distance = collision_distance;
        translate = direction * nearest_distance;
        data.remaining_distance = 0.0;
        log::debug!(
            "Bullet({}) hit building (distance:{})",
            &id,
            nearest_distance
        );
    }

    if nearest_distance <= 0.0 {
        return None;
    }

    // 플레이어와 총알이 충돌하는지 검사합니다.
    let mut nearest_player_id = None;
    for (&uid, player_data) in players {
        if uid == data.shooter_id
            || player_data.current_health == 0
            || player_data.team() == data.shooter_team
            || player_data.is_invincible()
        {
            continue;
        }

        let mut player_collider = player_data.character_attributes().collider.clone();
        player_collider.center = player_data.translation.into();

        let result = bullet_collider.check_dynamic_collision_details(&translate, &player_collider);
        if let Some(details) = result {
            if details.distance < nearest_distance && details.distance <= translate.length() {
                nearest_distance = details.distance;
                nearest_player_id = Some(uid);
            }
        }
    }

    nearest_player_id
}

/// 0.1m 마다 바닥과의 충돌을 검사하여 바닥과의 충돌을 확인합니다.
fn check_bullet_ground_collision(
    stage_kind: StageKind,
    mut translation: glam::Vec3A,
    radius: f32,
    direction: glam::Vec3A,
    distance: f32,
) -> Option<f32> {
    let mut nearest_distance = None;
    let mut moved = 0.0;
    let v = direction * 0.1;
    while moved < distance {
        if let Some(height) = get_stage_height(stage_kind, translation.x, translation.z) {
            if translation.y <= height + radius {
                nearest_distance = Some(moved);
                break;
            }
        }
        translation += v;
        moved += 0.1;
    }

    nearest_distance
}

/// 총알과 건물의 충돌 거리를 반환합니다.
fn check_bullet_building_collision(
    stage_kind: StageKind,
    bullet_collider: &Sphere,
    translate: glam::Vec3A,
) -> Option<f32> {
    let mut nearest_distance = None;
    let colliders = get_stage_colliders(stage_kind);
    for collider in ColliderTreeIterator::new(colliders) {
        // broadphase 검사 - 시작지점과 도착지점을 포함하는 AABB를 생성
        let rad_box = glam::Vec3A::new(
            bullet_collider.radius * translate.x.signum(),
            bullet_collider.radius * translate.y.signum(),
            bullet_collider.radius * translate.z.signum(),
        );
        let center = glam::Vec3A::from(bullet_collider.center);
        let start = center - rad_box;
        let end = center + translate + rad_box;
        let swept_aabb = BoundingBox::from_start_end(start.into(), end.into());

        if collider.check_aabb_collision(&swept_aabb) {
            // narrowphase 검사 - 총알과 충돌체의 충돌 검사
            let details = match collider {
                Collider::Aabb(aabb) => {
                    bullet_collider.check_dynamic_collision_details(&translate, aabb)
                }
                Collider::Obb(obb) => {
                    bullet_collider.check_dynamic_collision_details(&translate, obb)
                }
                Collider::Capsule(capsule) => {
                    bullet_collider.check_dynamic_collision_details(&translate, capsule)
                }
                Collider::OrientedCapsule(obb) => {
                    bullet_collider.check_dynamic_collision_details(&translate, obb)
                }
                Collider::Sphere(sphere) => {
                    bullet_collider.check_dynamic_collision_details(&translate, sphere)
                }
            };
            if let Some(details) = details {
                // 충돌체와의 충돌이 발생한 경우, 총알의 남은 거리와 충돌체의 거리 비교
                let distance = details.distance;
                if let Some(nearest) = nearest_distance {
                    if distance < nearest {
                        nearest_distance = Some(distance);
                    }
                } else {
                    nearest_distance = Some(distance);
                }
            }
        }
    }

    nearest_distance
}

/// 플레이어 총알 맞음 처리를 진행합니다.
fn bullet_hit_player(
    data: &mut Bullet,
    shooter_attr: &CharacterAttributes,
    target_attr: &CharacterAttributes,
    target_id: UserId,
) -> Damage {
    // 관통되지 않도록 처리
    data.remaining_distance = 0.0;

    // 회피 계산
    let accuracy_stat = shooter_attr.accuracy_stat as f32; // 명중 수치 (쏜 플레이어)
    let evasion_stat = target_attr.evasion_stat as f32; // 회피 수치 (맞은 플레이어)
    let hit_rate = accuracy_stat / (accuracy_stat + evasion_stat) + 0.5;
    if random_range(0.0..=1.0) > hit_rate {
        return Damage::miss();
    }

    // 총알 무기 상수
    let weapon_multi = match data.bullet_kind {
        BulletKind::Common => 1.0,
        BulletKind::EnergyBoll => 1.2,
    };
    let attk_stat = shooter_attr.attack_power as f32; // 공격력 수치 (쏜 플레이어)
    let defen_stat = target_attr.defense_power as f32; // 방어력 수치 (맞은 플레이어)
    let rand_factor = random_range(0.9..=1.1);
    let base_damage = (attk_stat - defen_stat).max(1.0) * weapon_multi * rand_factor;

    // 치명타 계산
    let crit_stat = shooter_attr.critical_rate as f32; // 치명률 (쏜 플레이어)
    let crit_rate = crit_stat / (crit_stat + evasion_stat);
    if random_range(0.0..=1.0) < crit_rate {
        // 크리티컬 판정
        let crit_damage = shooter_attr.critical_damage as f32 / 100.0; // 치명 데미지 (쏜 플레이어)
        let final_damage = (base_damage * crit_damage).min(MAX_DAMAGE as f32).floor() as u16;

        log::info!(
            "Player({}) hit by Bullet from Player({}) (critical damage:{})",
            &data.shooter_id,
            &target_id,
            &final_damage
        );
        Damage::critical(final_damage)
    } else {
        let final_damage = base_damage.min(MAX_DAMAGE as f32).floor() as u16;

        log::info!(
            "Player({}) hit by Bullet from Player({}) (damage:{})",
            &data.shooter_id,
            &target_id,
            &final_damage
        );
        Damage::common(final_damage)
    }
}
