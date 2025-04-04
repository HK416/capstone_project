use std::{
    fmt,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ahash::HashMap;
use mod_network::{
    components::{
        DamageLog, HealthPoint, LatLon, MovementState, ObjectId, PlayPhasePlayer, StageKind, Team, UserId
    },
    protocol::{Packet, PullStagePacket, UdpDamageLogPacket},
};
use mod_parallelism::collections::Queue;
use mod_physics::{
    collision::{Collider, ColliderTreeIterator, DynamicCollision},
    object3d::{BoundingBox, Sphere},
};
use tokio::time::{Duration, Instant};

use crate::{
    data::{get_nearest_valid_position, get_stage_colliders, get_stage_height, is_valid_position},
    entities::{BulletObject, PlayerObject}, 
    formula::movement_formulas as formulas, 
    world::{GameWorld, GameWorldEvent}
};

use super::GameWorldState;

/// 중력 가속도입니다.
const GRAVITY: glam::Vec3A = glam::vec3a(0.0, -9.8, 0.0);
const GROUNDED_ANGLE: f32 = 60.0;
lazy_static::lazy_static! {
    static ref GROUNDED_ANGLE_COS: f32 = f32::cos(f32::to_radians(GROUNDED_ANGLE));
}

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

    /// 플레이어 스폰 위치 저장
    spawn_positions: HashMap<UserId, (glam::Vec3A, glam::Quat, LatLon)>,

    /// 점령도. 100이 되어야 점령 점수를 얻을 수 있습니다.
    capture_progress: f32,
    /// 팀별 점령 점수. 점령도가 100일때 초당 1점씩 증가합니다.
    capture_score: [f32; 2],
    /// 점령중인 팀
    capture_team: Option<Team>,
    /// 점령지 충돌체
    capture_point_collider: Sphere, 
}

impl GameWorldInGameState {
    /// 최대 점령점수. capture_score가 이 값에 도달하면 게임이 종료됩니다.
    const MAX_CAPTURE_SCORE: f32 = 60.0;

    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(
        stage_kind: StageKind,
        spawn_positions: HashMap<UserId, (glam::Vec3A, glam::Quat, LatLon)>,
    ) -> Self {
        Self {
            is_running: true,
            previous_time_pt: Instant::now(),
            counter: 0,
            stage_kind,
            damage_logs: Queue::new(),
            spawn_positions,
            capture_progress: 0.0,
            capture_score: [0.0; 2],
            capture_team: None,
            capture_point_collider: Sphere {
                center: glam::Vec3::ZERO,
                radius: 7.5,
            },
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
        let colliders = get_stage_colliders(self.stage_kind);

        for mut player in world.players.iter_mut() {
            // 플레이어 위치를 가져옵니다.
            let translation = player.translation();

            // 플레이어 속도를 갱신합니다.
            player.update_velocity();

            // 플레이어의 이동 속도를 가져옵니다.
            let mut velocity = player.velocity();

            // 속도에 가속도를 적용합니다.
            if !player.is_grounded {
                velocity += GRAVITY * elapsed_time_sec;
            }

            // 이동 시도 (이동 전 위치 저장)
            let mut new_p = translation + velocity * elapsed_time_sec;

            // 충돌처리 시작
            player.is_grounded = false;

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
                        player.is_grounded = true;
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

            if !is_valid_position(self.stage_kind, new_p.x, new_p.z) {
                let (x, z) = get_nearest_valid_position(self.stage_kind, new_p.x, new_p.z);
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
                    player.is_grounded = true;
                }
            }

            if player.is_grounded {
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

    /// 총알과 충돌하는 플레이어를 확인합니다.
    fn check_bullet_collision(
        &self, 
        world: &GameWorld, 
        bullet: &mut BulletObject, 
        velocity: &glam::Vec3A,
    ) -> Option<UserId> {
        // bullet.velocity가 영벡터가 아니라고 가정
        let bullet_collider = Sphere {
            center: bullet.translation.into(),
            radius: bullet.radius,
        };

        let mut nearest_distance = f32::MAX;
        let mut nearest_player_id = None;

        for player in world.players.iter() {
            if *player.key() == bullet.shooter_id
                || player.health_point().0 == 0
                || player.team() == bullet.shooter_team
            {
                continue;
            }

            let player_collider = player.collider();

            // 충돌 처리: 플레이어 - 총알
            if let Some(info) = bullet_collider
                .check_dynamic_collision_details(velocity, &player_collider)
            {
                if info.distance <= velocity.length() {
                    println!("Bullet find player (player id: {})", player.account().uid);
                    println!("  - distance: {}", info.distance);
                    println!("  - surface normal: {}", info.normal);
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
        shooter: &PlayerObject,
        player: &mut PlayerObject,
    ) {
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
            println!("  - critical!");
        }

        // 최종 데미지 계산
        //기존: let crit_dam = char_info.critical_damage as f32;
        let crit_dam = shooter_info.critical_damage as f32; //발포자의 치명 수치여야 하는거아닌가?
        let final_dmg =
            formulas::final_damage(dmg, hit_rate, crit_rate, crit_dam).ceil() as u16;

        let health_point = player.health_point_mut();
        health_point.0 = health_point.0.saturating_sub(final_dmg);
        println!("  - hp: {}(-{})", health_point.0, final_dmg);

        if health_point.0 == 0 {
            println!("Player({}) is dead", player.account().uid);
            player.death();
        }

        self.damage_logs.push(DamageLog {
            user_id: player.account().uid,
            damage: HealthPoint(final_dmg),
        });
    }

    /// 총알 이동 및 충돌처리를 수행합니다.
    fn update_bullet(&self, world: &GameWorld, elapsed_time_sec: f32) {
        // 총알 이동
        for mut bullet in world.bullets.iter_mut() {
            let velocity = bullet.velocity * elapsed_time_sec;

            match self.check_bullet_collision(world, &mut bullet, &velocity) {
                Some(id) => {
                    println!("Player({}) hit by bullet", id);
                    let shooter = world.players.get_mut(&bullet.shooter_id).unwrap();
                    let mut player = world.players.get_mut(&id).unwrap();
                    self.bullet_hit_player(&mut bullet, &shooter, &mut player);
                }
                None => {
                    bullet.move_velocity(velocity);
                }
            }
        }

        // 살아남은 총알만 남김
        for bullet in world.bullets.iter() {
            if bullet.remaining_distance <= 0.0 {
                println!("Bullet range over");
                world.push_event(GameWorldEvent::RemoveBullet(*bullet.key()));
            }
        }
    }

    /// 점령지 안에 존재하는 팀과 인원수를 리턴합니다.  
    /// 점령지 안에 존재하는 팀이 없거나, 두 팀 모두 존재하는 경우 팀은 None입니다.  
    /// 점령지 안에 두 팀이 모두 존재하는 경우 인원수는 0이 아닌 양의 정수입니다.  
    fn get_new_capture_team(&self, world: &GameWorld) -> (Option<Team>, usize) {
        let mut new_capture_team = None;
        let mut capturing_count = 0;

        // 점령지 안에 있는 플레이어의 팀 확인
        let in_capture_point = world.players.iter()
            .filter(|player| player.health_point().0 > 0)
            .filter(|player| {
                self.capture_point_collider.check_point_collision(&player.translation())
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

    /// 점령지의 상태를 갱신합니다.
    fn update_capture_point(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        let (new_capture_team, capturing_count) = self.get_new_capture_team(world);

        // 아무도 없으면
        if capturing_count == 0 {
            // 현재 점령완료한 팀의 점령시간 증가
            if let Some(team) = self.capture_team {
                if self.capture_progress == 100.0 {
                    self.capture_score[team as usize] += elapsed_time_sec;
                    if self.capture_score[team as usize] >= Self::MAX_CAPTURE_SCORE {
                        self.capture_score[team as usize] = Self::MAX_CAPTURE_SCORE;
                        world.push_event(GameWorldEvent::GameOver {
                            winner: self.capture_team,
                        });
                    }
                }
            }
            return;
        }

        // 두 팀 모두 있는 경우
        if new_capture_team.is_none() {
            return;
        }

        // 한 팀만 있는 경우

        // 점령팀 및 점령도 갱신
        if new_capture_team != self.capture_team {
            // 인원수에 비례해서 점령도 증가
            self.capture_progress -= 10.0 * capturing_count as f32 * elapsed_time_sec;
            if self.capture_progress <= 0.0 {
                self.capture_team = new_capture_team;
                self.capture_progress = self.capture_progress.abs();
            }
        } else {
            if self.capture_progress == 100.0 {
                let team = new_capture_team.unwrap();
                self.capture_score[team as usize] += elapsed_time_sec;
                if self.capture_score[team as usize] >= Self::MAX_CAPTURE_SCORE {
                    self.capture_score[team as usize] = Self::MAX_CAPTURE_SCORE;
                    world.push_event(GameWorldEvent::GameOver {
                        winner: self.capture_team,
                    });
                }
            } else {
                self.capture_progress += 10.0 * capturing_count as f32 * elapsed_time_sec;
                self.capture_progress = self.capture_progress.min(100.0);
            }
        }

        println!("capture team: {:?}({:.1}%)", self.capture_team, self.capture_progress);
        println!("capture score: RED[{:.1}%] : BLUE[{:.1}%]", 
            self.capture_score[Team::Red as usize] / Self::MAX_CAPTURE_SCORE * 100.0, 
            self.capture_score[Team::Blue as usize] / Self::MAX_CAPTURE_SCORE * 100.0);
    }

    /// 게임 월드를 갱신합니다.
    fn update(&mut self, world: &GameWorld) {
        let current_time_pt = Instant::now();
        let elapsed_time_sec = current_time_pt
            .saturating_duration_since(self.previous_time_pt)
            .as_secs_f32();
        self.previous_time_pt = current_time_pt;

        // println!("fps: {:.2} (elapsed time: {})", 1.0 / elapsed_time_sec, elapsed_time_sec);

        self.update_player_state_timer(world, elapsed_time_sec);
        self.update_player_position(world, elapsed_time_sec);

        // 총알 이동 및 충돌처리
        self.update_bullet(world, elapsed_time_sec);

        // 점령상태 갱신
        self.update_capture_point(world, elapsed_time_sec);
    }

    /// 모든 세션 데이터에 패킷을 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let players: Vec<_> = world
            .players
            .iter_mut()
            .map(|player| {
                PlayPhasePlayer::new(
                    player.account().clone(),
                    player.character_kind(),
                    player.max_health_point(),
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

        // 플레이어가 비어있는 경우 패킷 전송을 생략합니다.
        if players.is_empty() {
            return;
        }

        let bullets: Vec<_> = world
            .bullets
            .iter()
            .map(|bullet| bullet.as_bullet())
            .collect();

        // 패킷을 생성하고 전송합니다.
        let packet = PullStagePacket::new(players, bullets);
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

impl GameWorldState for GameWorldInGameState {
    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
        match event {
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
            GameWorldEvent::GameOver { winner } => {
                println!("{:?} win!", self.capture_team.unwrap());
                log::info!("game over - winner: {:?}", winner);
                // self.is_running = false;
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

    fn yield_now(&self) -> Duration {
        Duration::from_millis(1)
    }
}

impl fmt::Debug for GameWorldInGameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameWorldInGameState))
    }
}
