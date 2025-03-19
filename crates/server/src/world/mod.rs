mod bullet;
mod event;
mod player;

use std::{
    fmt,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU32, AtomicU64, Ordering as MemOrdering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use ahash::RandomState;
use dashmap::{DashMap, DashSet, mapref::one::RefMut};
use mod_network::{
    components::{
        CharacterKind, DamageLog, Epoch, HealthPoint, InGameStatus, MovementState, ObjectId,
        StageKind, Team, UserId, WorldId,
    },
    protocol::{InitStagePacket, Packet, PullStagePacket, UdpDamageLogPacket},
};
use mod_parallelism::collections::Queue;
use mod_physics::collision::Ray;
use tokio::time::Instant;

use crate::{
    data::{clamp_x, clamp_z, get_stage_height, is_valid_position},
    session::Session,
};

pub use self::{bullet::*, event::*, player::*};

use super::formula::movement_formulas as formulas;

/// 중력 가속도입니다.
const GRAVITY: glam::Vec3A = glam::vec3a(0.0, -9.8, 0.0);

/// 게임 개발을 위한 테스트 게임 월드 입니다.
///
/// # Note
/// 테스트 게임 월드는 인원 제한이 없습니다.
///
#[derive(Debug)]
pub struct GameWorld {
    /// 게임 월드 식별자입니다.
    world_id: WorldId,
    /// 게임 월드의 현재 시대입니다.  
    epoch: AtomicU64,
    /// 오브젝트 식별자를 생성하기 위한 카운터입니다.
    counter: AtomicU32,

    /// 게임 지형의 종류입니다.
    stage_kind: StageKind,

    /// 게임 월드에 새로 들어온 세션 데이터입니다.
    new_sessions: Queue<Arc<Session>>,
    /// 게임 월드에 참가한 세션 데이터입니다.
    sessions: DashSet<Arc<Session>, RandomState>,
    /// 게임 월드에 포함된 플레이어 캐릭터 데이터입니다.
    players: DashMap<UserId, PlayerObject, RandomState>,
    /// 게임 월드에 포함된 총알 데이터입니다.
    bullets: DashMap<ObjectId, BulletObject, RandomState>,

    /// 플레이어 데미지 로그입니다.
    damage_logs: Queue<DamageLog>,

    /// 게임 월드에 발생한 이벤트 대기열입니다.
    events: Queue<GameWorldEvent>,
}

impl GameWorld {
    /// 게임 월드의 인스턴스를 가져옵니다.
    pub fn get_instance() -> Arc<Self> {
        static INSTANCE: OnceLock<Arc<GameWorld>> = OnceLock::new();
        INSTANCE
            .get_or_init(|| Arc::new(GameWorld::default()))
            .clone()
    }

    /// 오브젝트 식별자를 생성합니다.
    pub fn generate_object_id(&self) -> ObjectId {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

        let counter_bit = self.counter.fetch_add(1, MemOrdering::AcqRel) & 0xFFFF;
        let time_bit = duration.subsec_nanos() & 0xFFFF;

        ObjectId::new((time_bit << 16) | counter_bit)
    }

    /// 게임 월드에 참가합니다.   
    /// NOTE: 플레이어의 캐릭터가 바로 생성되지 않습니다.
    pub fn join(&self, session: Arc<Session>, character_kind: CharacterKind) {
        // 플레이어 생성 이벤트를 추가합니다.
        self.push_event(GameWorldEvent::AddPlayer(session, character_kind));
    }

    /// 게임 월드에서 나갑니다.
    pub fn exit(&self, session: &Session) {
        self.sessions.remove(session);
        self.push_event(GameWorldEvent::RemovePlayer(session.user().uid));
    }

    /// 플레이어 캐릭터를 추가합니다.
    fn add_player(&self, session: Arc<Session>, character_kind: CharacterKind) {
        let info = session.user().clone();
        let user_id = info.uid;
        if !self.players.contains_key(&user_id) {
            // 새로 참가한 세션 목록에 세션을 추가합니다.
            self.new_sessions.push(session);
            // 플레이어 오브젝트를 생성합니다.
            let player = PlayerObject::new(
                info,
                Team::default(),         // Temp
                InGameStatus::default(), // Temp
                character_kind,
            );
            // 플레이어 오브젝트를 추가합니다.
            self.players.insert(user_id, player);
            log::info!("the Player({}) has been added to the {}", &user_id, &self);
        } else {
            log::warn!(
                "failed to create player. (REASON:the Player({}) already exists in the {})",
                &user_id,
                &self
            );
        }
    }

    /// 게임 세상에서 플레이어 오브젝트를 제거합니다.
    fn remove_player(&self, user_id: UserId) {
        match self.players.remove(&user_id) {
            Some(_) => log::info!("Player({}) is removed from the {}", &user_id, &self),
            None => log::warn!("the Player({}) could not be found in {}!", &user_id, &self),
        };
    }

    /// 게임 세상에 존재하는 해당 플레이어 오브젝트를 가져옵니다.  
    /// 해당 플레이어 오브젝트가 존재하지 않는 경우 `None`을 전달합니다.
    pub fn get_mut_player<F>(&self, user_id: UserId, func: F)
    where
        F: FnOnce(&GameWorld, Option<RefMut<'_, UserId, PlayerObject>>),
    {
        func(self, self.players.get_mut(&user_id))
    }

    /// 플레이어 상태 타이머를 갱신합니다.
    fn update_player_state_timer(&self, elapsed_time_sec: f32) {
        for mut player in self.players.iter_mut() {
            player.update_state_timer(self, elapsed_time_sec);
        }
    }

    /// 주어진 시간 간격으로 플레이어의 위치를 갱신합니다.
    fn update_player_position(&self, elapsed_time_sec: f32) {
        for mut player in self.players.iter_mut() {
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
    fn add_bullet(&self, shooter_id: UserId, delay: f32) {
        // 총알을 발사한 플레이어 정보를 가져옵니다.
        let player = match self.players.get(&shooter_id) {
            Some(player) => player,
            None => {
                log::warn!(
                    "failed to create bullet. (REASON:the Player({}) could not be found in {})",
                    &shooter_id,
                    &self
                );
                return;
            }
        };

        let object_id = self.generate_object_id();
        let bullet = player.generate_bullet(object_id, delay);
        self.bullets.insert(object_id, bullet);
        log::info!(
            "the Player({}) fires a Bullet({}) into the {}",
            &shooter_id,
            &object_id,
            &self
        );
    }

    /// 게임 세상에서 총알 오브젝트를 제거합니다.
    fn remove_bullet(&self, object_id: ObjectId) {
        match self.bullets.remove(&object_id) {
            Some(_) => log::info!("Bullet({}) is removed from the {}", &object_id, &self),
            None => log::warn!(
                "the Bullet({}) could not be found in {}!",
                &object_id,
                &self
            ),
        };
    }

    /// 게임 월드 이벤트를 추가합니다.
    pub fn push_event(&self, event: GameWorldEvent) {
        self.events.push(event);
    }

    /// 이벤트를 처리합니다.
    pub fn flush_events(&self) {
        while let Some(event) = self.events.pop() {
            match event {
                GameWorldEvent::AddPlayer(session, kind) => {
                    self.add_player(session, kind);
                }
                GameWorldEvent::RemovePlayer(user_id) => {
                    self.remove_player(user_id);
                }
                GameWorldEvent::AddBullet { shooter_id, delay } => {
                    self.add_bullet(shooter_id, delay);
                }
                GameWorldEvent::RemoveBullet(object_id) => {
                    self.remove_bullet(object_id);
                }
            };
        }
    }

    /// 주어진 시간 간격으로 게임 월드를 갱신합니다.
    fn advanced(&self, elapsed_time_sec: f32) {
        self.update_player_state_timer(elapsed_time_sec);
        self.update_player_position(elapsed_time_sec);

        // 총알 이동
        for mut bullet in self.bullets.iter_mut() {
            let translation = bullet.translation;
            let direction = bullet.velocity * elapsed_time_sec;
            let move_distance = direction.length();

            // bullet.velocity가 영벡터가 아니라고 가정
            let ray = Ray::build(bullet.translation, direction).unwrap();

            let mut nearest_distance = f32::MAX;
            let mut nearest_player_id = None;

            for player in self.players.iter() {
                if *player.key() == bullet.shooter_id {
                    continue;
                }

                let attributes = player.character_attributes();
                let player_collider = player.collider().inflated(attributes.bullet_radius);

                // 충돌 처리: 플레이어 - 총알
                if let Some(info) = ray.intersect(&player_collider) {
                    if info.distance <= move_distance {
                        println!("Bullet find player (player id: {:?})", player.info().uid);
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
                    let mut player = self.players.get_mut(&id).unwrap();
                    let char_info = player.character_attributes();

                    //발포자 정보
                    let mut shooter = self.players.get_mut(&bullet.shooter_id).unwrap();
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
                        formulas::final_damage(dmg, hit_rate, crit_rate, crit_dam).ceil() as u32;

                    let health_point = player.health_mut();
                    health_point.0 = (health_point.0 - final_dmg).max(0);
                    println!("  - hp: {:?}(-{})", health_point.0, final_dmg);

                    self.damage_logs.push(DamageLog {
                        user_id: player.info().uid,
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
        for bullet in self.bullets.iter() {
            if bullet.remaining_distance <= 0.0 {
                self.events
                    .push(GameWorldEvent::RemoveBullet(*bullet.key()));
            }
        }
    }

    /// 모든 세션 데이터에 패킷을 전송합니다.
    fn broadcast(&self) {
        let epoch = self.epoch.fetch_add(1, MemOrdering::AcqRel);
        let players: Vec<_> = self
            .players
            .iter()
            .map(|player| player.as_player())
            .collect();
        let bullets: Vec<_> = self
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
                let packet = UdpDamageLogPacket::new(Epoch::new(epoch), logs);
                for session in self.sessions.iter() {
                    session.tcp_write(packet.as_raw());
                }
            } else {
                break;
            }
        }

        // 패킷을 생성하고 전송합니다.
        let packet = PullStagePacket {
            epoch: Epoch::new(epoch),
            num_players: players.len() as u16,
            players: players.clone(),
            num_bullets: bullets.len() as u16,
            bullets,
        };
        for session in self.sessions.iter() {
            session.tcp_write(packet.as_raw());
        }

        // 패킷을 생성하고 전송합니다.
        let packet = InitStagePacket {
            epoch: Epoch::new(epoch),
            stage_kind: self.stage_kind,
            num_players: players.len() as u16,
            players,
        };
        while let Some(session) = self.new_sessions.pop() {
            session.tcp_write(packet.as_raw());
            self.sessions.insert(session);
        }
    }
}

impl Default for GameWorld {
    fn default() -> Self {
        Self {
            world_id: WorldId::NULL,
            epoch: AtomicU64::new(0),
            counter: AtomicU32::new(0),
            stage_kind: StageKind::default(),
            new_sessions: Queue::default(),
            sessions: DashSet::default(),
            players: DashMap::default(),
            bullets: DashMap::default(),
            damage_logs: Queue::default(),
            events: Queue::default(),
        }
    }
}

impl fmt::Display for GameWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameWorld({})", &self.world_id)
    }
}

/// 고정 시간 갱신 간격입니다.
const INTERVAL: f32 = 1.0 / 120.0; // 120 FPS

/// 게임 월드를 갱신하는 루프 함수입니다.
pub async fn update_game_world(world: Arc<GameWorld>) {
    let mut previous_time_point = Instant::now();
    loop {
        // 경과 시간을 계산합니다.
        let current_time_point = Instant::now();
        let mut elapsed_time_sec = current_time_point
            .saturating_duration_since(previous_time_point)
            .as_secs_f32();
        previous_time_point = current_time_point;

        // 게임 월드를 갱신합니다.
        world.flush_events();
        while elapsed_time_sec > INTERVAL {
            world.advanced(INTERVAL);
            elapsed_time_sec -= INTERVAL;
        }
        world.advanced(elapsed_time_sec);

        // 모든 세션에 게임 월드 데이터를 전송합니다.
        world.broadcast();

        // 다른 태스크들이 실행될 기회를 주기 위해 양보
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
}
