use std::{
    fmt,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ahash::HashMap;
use mod_network::{
    components::{
        ActionState, ActionStateTimer, DamageLog, ExSkillCost, FinishPhasePlayer, GamePlayData,
        HealthPoint, LatLon, MAX_IN_GAME_PLAYERS, MovementState, MovementStateTimer, ObjectId,
        PlayPhasePlayer, RemainingBullet, StageKind, Team, UserId, VictoryType, ViewState,
        ViewStateTimer,
    },
    protocol::{FinishStagePacket, Packet, PullStagePacket, UdpDamageLogPacket},
};
use mod_parallelism::collections::Queue;
use mod_physics::{
    collision::{Collider, ColliderTreeIterator, DynamicCollision},
    object3d::{BoundingBox, Sphere},
};
use tokio::time::Instant;

use crate::{
    data::{get_nearest_valid_position, get_stage_colliders, get_stage_height, is_safe_area, is_valid_position},
    entities::{BulletObject, CapturePointObject, PlayData, PlayerObject},
    formula::movement_formulas as formulas,
    session::SessionEvents,
    world::{GameWorld, GameWorldEvent},
};

use super::{GameWorldState, GameWorldStateFlow};

/// 중력 가속도입니다.
const GRAVITY: glam::Vec3A = glam::vec3a(0.0, -9.8, 0.0);
const GROUNDED_ANGLE: f32 = 45.0;
lazy_static::lazy_static! {
    static ref GROUNDED_ANGLE_COS: f32 = f32::cos(f32::to_radians(GROUNDED_ANGLE));
}

pub struct GameWorldInGameState {
    /// 게임 월드 상태 실행 여부
    is_running: bool,
    /// 이전 측정 시각
    previous_time_pt: Instant,

    /// 총 게임 진행 시간(초)
    total_play_sec: f32,
    /// 남은 게임 시간 (초)
    remaining_time_sec: f32,
    /// 게임 월드 상태의 경과 시간
    elapsed_time_sec: f32,

    /// 오브젝트 식별자를 생성하기 위한 카운터입니다.
    counter: u32,

    /// 게임 스테이지 종류
    stage_kind: StageKind,

    /// 플레이어 데미지 로그입니다.
    damage_logs: Queue<DamageLog>,

    /// 플레이어 스폰 위치 저장
    spawn_positions: HashMap<UserId, (glam::Vec3A, glam::Quat, LatLon)>,
    /// 플레이어 게임 플레이 데이터 저장
    play_data: Option<HashMap<UserId, PlayData>>,

    /// 점령지 오브젝트
    capture_point: CapturePointObject,
}

impl GameWorldInGameState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(
        stage_kind: StageKind,
        spawn_positions: HashMap<UserId, (glam::Vec3A, glam::Quat, LatLon)>,
        play_data: HashMap<UserId, PlayData>,
        game_duration_sec: f32,
    ) -> Self {
        Self {
            is_running: true,
            previous_time_pt: Instant::now(),
            total_play_sec: game_duration_sec,
            remaining_time_sec: game_duration_sec,
            elapsed_time_sec: 0.0,
            counter: 0,
            stage_kind,
            damage_logs: Queue::new(),
            spawn_positions,
            play_data: Some(play_data),
            capture_point: CapturePointObject::new(Collider::Sphere(Sphere {
                center: glam::Vec3::ZERO,
                radius: 7.5,
            })),
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
            "the Player({}) fires a Bullet({}) into the GameWorld({})",
            &shooter_id,
            &object_id,
            &world.id()
        );
    }

    /// 게임 세상에서 총알 오브젝트를 제거합니다.
    fn remove_bullet(&self, world: &GameWorld, object_id: ObjectId) {
        match world.bullets.remove(&object_id) {
            Some(_) => log::info!(
                "Bullet({}) is removed from the GameWorld({})",
                &object_id,
                &world.id()
            ),
            None => log::warn!(
                "the Bullet({}) could not be found in GameWorld({})!",
                &object_id,
                &world.id()
            ),
        };
    }
}

//--------------------------------------------------------------------------------------------
// 처리와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldInGameState {
    /// 플레이어 떠남 이벤트를 처리합니다.
    fn handle_player_leave_event(&mut self, uid: UserId) {
        let play_data = self
            .play_data
            .as_mut()
            .expect("the game play data must exist!");

        // 연결 상태 부울 플래그를 false로 설정합니다.
        if let Some(data) = play_data.get_mut(&uid) {
            data.connected = false;
        } else {
            log::warn!("unknown game player (UID:{})", uid);
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
        let mut nearest_distance = None;
        let mut position = bullet_position.clone();
        let mut moved = 0.0;
        let v = move_v_normalized * 0.1;
        while moved < move_distance {
            if let Some(height) = get_stage_height(self.stage_kind, position.x, position.z) {
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
        let colliders = get_stage_colliders(self.stage_kind);
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
        bullet: &mut BulletObject,
        move_v: &glam::Vec3A,
    ) -> Option<UserId> {
        let mut nearest_distance = f32::MAX;
        let mut move_v = move_v.clone();
        let move_v_normalized = move_v.normalize();

        // 지형 충돌 검사
        if let Some(collision_distance) = self.check_bullet_ground_collision(
            &bullet.translation,
            bullet.radius,
            &move_v_normalized,
            move_v.length(),
        ) {
            nearest_distance = collision_distance;
            move_v = move_v_normalized * nearest_distance;
            bullet.remaining_distance = 0.0;
            log::debug!(
                "Bullet({}) hit ground (distance: {})",
                bullet.object_id,
                nearest_distance
            );
        }

        if nearest_distance == 0.0 {
            return None;
        }

        // 건물 충돌 검사
        let bullet_collider = Sphere {
            center: bullet.translation.into(),
            radius: bullet.radius,
        };
        if let Some(collision_distance) =
            self.check_bullet_building_collision(&bullet_collider, &move_v)
        {
            nearest_distance = collision_distance;
            move_v = move_v_normalized * nearest_distance;
            bullet.remaining_distance = 0.0;
            log::debug!(
                "Bullet({}) hit building (distance: {})",
                bullet.object_id,
                nearest_distance
            );
        }

        if nearest_distance == 0.0 {
            return None;
        }

        // 플레이어 충돌 검사
        let mut nearest_player_id = None;
        for player in world.players.iter() {
            if *player.key() == bullet.shooter_id
                || player.health_point().current == 0
                || player.team() == bullet.shooter_team
                || player.is_invincible()
            {
                continue;
            }

            let player_collider = player.collider();

            // 충돌 처리: 플레이어 - 총알
            if let Some(info) =
                bullet_collider.check_dynamic_collision_details(&move_v, &player_collider)
            {
                if info.distance <= move_v.length() {
                    // println!("Bullet find player (player id: {})", player.account().uid);
                    // println!("  - distance: {}", info.distance);
                    // println!("  - surface normal: {}", info.normal);
                    if info.distance < nearest_distance {
                        nearest_distance = info.distance;
                        nearest_player_id = Some(*player.key());
                    }
                }
            }
        }

        nearest_player_id
    }

    /// 총알과 플레이어의 충돌처리를 수행합니다.
    fn bullet_hit_player(
        &self,
        bullet: &mut BulletObject,
        shooter: &mut PlayerObject,
        player: &mut PlayerObject,
    ) {
        // println!("Player({}) hit by bullet", player.account().uid);

        // 관통되지 않도록 처리
        bullet.remaining_distance = 0.0;

        let char_info = player.character_attributes();

        //발포자 정보
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
            // println!("  - critical!");
        }

        // 최종 데미지 계산
        //기존: let crit_dam = char_info.critical_damage as f32;
        let crit_dam = shooter_info.critical_damage as f32; //발포자의 치명 수치여야 하는거아닌가?
        let final_dmg = formulas::final_damage(dmg, hit_rate, crit_rate, crit_dam).ceil() as u16;

        let uid = player.account().uid;
        let health_point = player.health_point_mut();
        health_point.current = health_point.current.saturating_sub(final_dmg);
        // println!("  - hp: {}(-{})", health_point.0, final_dmg);
        log::info!(
            "Player({}) hit by Bullet from Player({}) (damage: {})",
            uid,
            shooter.account().uid,
            final_dmg
        );

        // 데미지 비례 코스트 회복
        shooter.add_ex_skill_cost(final_dmg as f32 / 20.0);

        if health_point.current == 0 {
            // println!("Player({}) is dead", player.account().uid);
            log::info!(
                "Player({}) is dead (shooter: {})",
                player.account().uid,
                shooter.account().uid,
            );
            player.death();
        }

        self.damage_logs.push(DamageLog {
            user_id: player.account().uid,
            damage: final_dmg,
        });
    }

    /// 점령지 안에 존재하는 팀과 인원수를 리턴합니다.  
    /// 점령지 안에 존재하는 팀이 없거나, 두 팀 모두 존재하는 경우 팀은 None입니다.  
    /// 점령지 안에 두 팀이 모두 존재하는 경우 인원수는 0이 아닌 양의 정수입니다.  
    fn get_new_capture_team(&self, world: &GameWorld) -> (Option<Team>, usize) {
        let mut new_capture_team = None;
        let mut capturing_count = 0;

        // 점령지 안에 있는 플레이어의 팀 확인
        let in_capture_point = world
            .players
            .iter()
            .filter(|player| player.health_point().current > 0)
            .filter(|player| {
                self.capture_point
                    .collider()
                    .check_point_collision(&player.translation())
            })
            .map(|player| player.team());
        for team in in_capture_point {
            match new_capture_team {
                Some(capturing_team) => {
                    if team == capturing_team {
                        capturing_count += 1;
                    } else {
                        new_capture_team = None;
                        break;
                    }
                }
                None => {
                    new_capture_team = Some(team);
                    capturing_count += 1;
                }
            }
        }

        (new_capture_team, capturing_count)
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&self, world: &GameWorld) {
        // 게임 월드 상태가 비활성화 되어있거나,
        // 경과 시간이 5초 미만인 경우 전환을 시도하지 않습니다.
        if !self.is_running || self.elapsed_time_sec < 5.0 {
            return;
        }

        // 힌쪽 팀 플레이어가 비어있는 경우 부전승으로 처리합니다.
        let mut blue_teams = 0;
        let mut red_teams = 0;
        let play_data = self.play_data.as_ref().unwrap();
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (user_id, data) in play_data.iter() {
            if world.players.contains_key(user_id) {
                match data.team {
                    Team::Blue => blue_teams += 1,
                    Team::Red => red_teams += 1,
                };
            }

            players.push(FinishPhasePlayer::new(
                data.account,
                data.character_kind,
                data.kill_count,
                data.dead_count,
                data.damage_dealt,
                data.damage_taken,
                data.healing_given,
                data.team,
                data.team_index,
            ));
        }

        // 플레이어가 비어있는 경우 함수 실행을 중단합니다.
        if players.is_empty() {
            return;
        }

        if blue_teams == 0 {
            // 패킷을 생성하고 전송합니다.
            let play_time = self.total_play_sec - self.remaining_time_sec;
            let packet = FinishStagePacket::new(
                Team::Red,
                VictoryType::DefaultWin,
                self.stage_kind,
                play_time,
                players,
            );

            for item in world.sessions.iter() {
                item.key().push_event(SessionEvents::GameFinished);
                item.key().tcp_write(packet.as_raw());
            }

            // 게임 월드 상태를 변경합니다.
            let control_flow = GameWorldStateFlow::Pop;
            let event = GameWorldEvent::SetControlFlow(control_flow);
            world.push_event(event);
            return;
        } else if red_teams == 0 {
            // 패킷을 생성하고 전송합니다.
            let play_time = self.total_play_sec - self.remaining_time_sec;
            let packet = FinishStagePacket::new(
                Team::Blue,
                VictoryType::DefaultWin,
                self.stage_kind,
                play_time,
                players,
            );

            for item in world.sessions.iter() {
                item.key().push_event(SessionEvents::GameFinished);
                item.key().tcp_write(packet.as_raw());
            }

            // 게임 월드 상태를 변경합니다.
            let control_flow = GameWorldStateFlow::Pop;
            let event = GameWorldEvent::SetControlFlow(control_flow);
            world.push_event(event);
            return;
        }
    }
}

//--------------------------------------------------------------------------------------------
// 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldInGameState {
    /// 플레이어 상태 타이머를 갱신합니다.
    fn update_player_state_timer(&self, world: &GameWorld, elapsed_time_sec: f32) {
        for mut player in world.players.iter_mut() {
            player.update_state_timer(world, elapsed_time_sec);
        }
    }

    /// 주어진 시간 간격으로 플레이어의 위치를 갱신합니다.
    fn update_player_position(&self, world: &GameWorld, elapsed_time_sec: f32) {
        let colliders = get_stage_colliders(self.stage_kind);

        for mut player in world.players.iter_mut() {
            // 플레이어 위치를 가져옵니다.
            let translation = player.translation();

            // 플레이어 속도를 갱신합니다.
            player.update_velocity();

            // 플레이어의 이동 속도를 가져옵니다.
            let mut velocity = player.velocity();

            // 속도에 가속도를 적용합니다.
            if !player.is_grounded() {
                velocity += GRAVITY * elapsed_time_sec;
            }

            // 이동 시도 (이동 전 위치 저장)
            let mut new_p = translation + velocity * elapsed_time_sec;

            // 충돌처리 시작
            player.set_grounded(false);

            let mut player_capsule = player.collider();
            player_capsule.center = new_p.into();
            let player_aabb = BoundingBox::from(&player_capsule);
            let player_collider = Collider::Capsule(player_capsule);

            for collider in ColliderTreeIterator::new(colliders) {
                if !collider.check_aabb_collision(&player_aabb) {
                    continue;
                }
                if let Some(collision_info) = player_collider.check_collision_details(collider) {
                    new_p += collision_info.normal * collision_info.penetration;
                    // 충돌벡터가 지면(xz평면)과 일정 이상의 각을 이루면 서있을 수 있음
                    if collision_info.normal.y >= *GROUNDED_ANGLE_COS {
                        velocity.y = 0.0;
                        player.set_grounded(true);
                    }
                    // 아니라면 미끄러지도록 처리
                    else {
                        let slide =
                            velocity - collision_info.normal * velocity.dot(collision_info.normal);
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

            let team = player.team();

            if !is_valid_position(self.stage_kind, team, new_p.x, new_p.z) {
                let (x, z) = get_nearest_valid_position(self.stage_kind, 
                    team, new_p.x, new_p.z);
                if x != new_p.x {
                    velocity.x = 0.0;
                    new_p.x = x;
                }
                if z != new_p.z {
                    velocity.z = 0.0;
                    new_p.z = z;
                }
            }

            if let Some(height) = get_stage_height(self.stage_kind, new_p.x, new_p.z) {
                if height >= new_p.y {
                    new_p.y = height;
                    velocity.y = 0.0;
                    player.set_grounded(true);
                }
            }

            let in_safe_area = is_safe_area(self.stage_kind, team, new_p.x, new_p.z);
            player.set_invincible(in_safe_area);
            if in_safe_area {
                player.health_point_mut().current = player.health_point().maximum;
            }

            if player.is_grounded() {
                match player.movement_state() {
                    MovementState::InPlaceLanding => {
                        player.change_movement_state(MovementState::Idle);
                    }
                    MovementState::MovingLanding => {
                        player.change_movement_state(MovementState::Moving);
                    }
                    MovementState::InPlaceJumping | MovementState::MovingJumping => {
                        velocity.y = 5.0;
                    }
                    _ => {}
                }
            }

            *player.velocity_mut() = velocity;
            *player.translation_mut() = new_p;
            player.update_collider();
        }
    }

    /// 총알 이동 및 충돌처리를 수행합니다.
    fn update_bullet(&self, world: &GameWorld, elapsed_time_sec: f32) {
        // 총알 이동
        for mut bullet in world.bullets.iter_mut() {
            let velocity = bullet.velocity * elapsed_time_sec;

            match self.check_bullet_collision(world, &mut bullet, &velocity) {
                Some(id) => {
                    let mut shooter = world.players.get_mut(&bullet.shooter_id).unwrap();
                    let mut player = world.players.get_mut(&id).unwrap();
                    self.bullet_hit_player(&mut bullet, &mut shooter, &mut player);
                }
                None => {
                    bullet.move_velocity(velocity);
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

    /// 점령지의 상태를 갱신합니다.
    fn update_capture_point(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        let (new_capture_team, capturing_count) = self.get_new_capture_team(world);

        // 점령 수행
        let winner =
            self.capture_point
                .capture(new_capture_team, elapsed_time_sec, capturing_count);
        if let Some(winner) = winner {
            log::info!("capture complete");
            self.game_over(world, winner, VictoryType::JudgmentWin);
        }

        // println!("capture team: {:?}({:.1}%)\t score: RED[{:.1}%] : BLUE[{:.1}%]",
        //     self.capture_point.capture_team(), self.capture_point.capture_progress(),
        //     self.capture_point.capture_score()[Team::Red as usize] / CapturePointObject::MAX_CAPTURE_SCORE * 100.0,
        //     self.capture_point.capture_score()[Team::Blue as usize] / CapturePointObject::MAX_CAPTURE_SCORE * 100.0
        // );
    }

    /// 게임을 종료합니다.
    fn game_over(
        &mut self,
        world: &GameWorld,
        winner: Team,
        victory_type: VictoryType,
    ) {
        log::info!("game over (winner: {:?})", winner);

        self.is_running = false;

        let play_data = self.play_data.as_ref().unwrap();
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (_, data) in play_data.iter() {
            players.push(FinishPhasePlayer::new(
                data.account,
                data.character_kind,
                data.kill_count,
                data.dead_count,
                data.damage_dealt,
                data.damage_taken,
                data.healing_given,
                data.team,
                data.team_index,
            ));
        }

        // 플레이어가 비어있는 경우 함수 실행을 중단합니다.
        if players.is_empty() {
            return;
        }

        // 패킷을 생성하고 전송합니다.
        let play_time = self.total_play_sec - self.remaining_time_sec;
        let packet = FinishStagePacket::new(
            winner,
            victory_type,
            self.stage_kind,
            play_time,
            players,
        );

        for item in world.sessions.iter() {
            item.key().push_event(SessionEvents::GameFinished);
            item.key().tcp_write(packet.as_raw());
        }

        // 게임 월드 상태를 변경합니다.
        let control_flow = GameWorldStateFlow::Pop;
        let event = GameWorldEvent::SetControlFlow(control_flow);
        world.push_event(event);
    }

    /// 시간 초과로 게임을 종료합니다.
    fn time_out(&mut self, world: &GameWorld) {
        log::info!("time out");
        
        let capture_scores = self.capture_point.capture_score();
        let red_score = capture_scores[Team::Red as usize];
        let blue_score = capture_scores[Team::Blue as usize];

        let winner = if red_score > blue_score {
            Team::Red
        } else if red_score < blue_score {
            Team::Blue
        } else {
            // 동점인 경우 아직 게임을 끝내지 않음
            return;
        };

        self.game_over(world, winner, VictoryType::JudgmentWin);
    }

    /// 게임 월드를 갱신합니다.
    fn update(&mut self, world: &GameWorld) {
        let current_time_pt = Instant::now();
        let elapsed_time_sec = current_time_pt
            .saturating_duration_since(self.previous_time_pt)
            .as_secs_f32();
        self.previous_time_pt = current_time_pt;

        // 경과 시간과 남은 시간 업데이트
        self.remaining_time_sec = (self.remaining_time_sec - elapsed_time_sec).max(0.0);
        self.elapsed_time_sec += elapsed_time_sec;
        if self.remaining_time_sec <= 0.0 {
            self.time_out(world);
            // return;
        }

        self.update_player_state_timer(world, elapsed_time_sec);
        self.update_player_position(world, elapsed_time_sec);

        // 총알 이동 및 충돌처리
        self.update_bullet(world, elapsed_time_sec);

        // 점령상태 갱신
        self.update_capture_point(world, elapsed_time_sec);
    }
}

//--------------------------------------------------------------------------------------------
// 패킷 전송과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl GameWorldInGameState {
    /// 모든 세션 데이터에 패킷을 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        // Safe: 플레이 데이터가 없는 경우 이벤트를 처리하지 않습니다.
        let play_data = unsafe { self.play_data.as_ref().unwrap_unchecked() };

        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for (&user_id, data) in play_data.iter() {
            // 게임 월드에서 플레이어를 가져옵니다.
            let player = world.players.get(&user_id);
            players.push(match player {
                Some(player) => PlayPhasePlayer::new(
                    true,
                    player.account().clone(),
                    GamePlayData {
                        kill_count: data.kill_count,
                        dead_count: data.dead_count,
                    },
                    player.character_kind(),
                    player.remaining_bullet(),
                    player.health_point(),
                    player.translation().to_array(),
                    player.rotation().to_array(),
                    player.team(),
                    player.team_index(),
                    player.get_ex_skill_cost(),
                    player.action_state(),
                    player.action_state_timer(),
                    player.movement_state(),
                    player.movement_state_timer(),
                    player.view_state(),
                    player.view_state_timer(),
                    player.view_rotation(),
                ),
                None => PlayPhasePlayer::new(
                    false,
                    data.account,
                    GamePlayData {
                        kill_count: data.kill_count,
                        dead_count: data.dead_count,
                    },
                    data.character_kind,
                    RemainingBullet::default(),
                    HealthPoint::default(),
                    [0.0; 3],
                    [0.0; 4],
                    data.team,
                    data.team_index,
                    ExSkillCost::default(),
                    ActionState::default(),
                    ActionStateTimer::default(),
                    MovementState::default(),
                    MovementStateTimer::default(),
                    ViewState::default(),
                    ViewStateTimer::default(),
                    LatLon::default(),
                ),
            });
        }

        let bullets: Vec<_> = world
            .bullets
            .iter()
            .map(|bullet| bullet.as_bullet())
            .collect();

        let capture_point = self.capture_point.capture_point().clone();
        let remaining_time_sec = self.remaining_time_sec; // Copy the value

        // 게임 월드에 플레이어가 없는 경우 함수 실행을 중단합니다.
        if world.players.is_empty() {
            return;
        }

        // 패킷을 생성하고 전송합니다.
        let packet = PullStagePacket::new(players, bullets, capture_point, remaining_time_sec);
        for session in world.sessions.iter() {
            session.key().tcp_write(packet.as_raw());
        }

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
    }
}

//--------------------------------------------------------------------------------------------

impl GameWorldState for GameWorldInGameState {
    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
        // 게임 월드 상태가 실행 중이 아닌 경우 함수를 빠져나옵니다.
        if !self.is_running {
            return;
        }

        match event {
            GameWorldEvent::PlayerLeave(user_id) => {
                self.handle_player_leave_event(user_id);
            }
            GameWorldEvent::AddBullet { shooter_id, delay } => {
                self.add_bullet(world, shooter_id, delay);
            }
            GameWorldEvent::RemoveBullet(object_id) => {
                self.remove_bullet(world, object_id);
            }
            GameWorldEvent::RespawnPlayer { uid } => {
                if let Some(mut player) = world.players.get_mut(&uid) {
                    let user_id = player.account().uid;
                    if let Some(&(position, direction, view_rotation)) =
                        self.spawn_positions.get(&user_id)
                    {
                        player.reset_state();
                        player
                            .with_translation(position)
                            .with_rotation(direction)
                            .with_view_rotation(view_rotation);
                    } else {
                        log::warn!("could not find player spawn position! (UID:{user_id})");
                    }
                } else {
                    log::warn!("failed to respawn player (uid: {})", uid);
                }
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
        self.try_enter_next_state(world);
    }
}

impl fmt::Debug for GameWorldInGameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameState))
    }
}
