mod data;
mod event;
mod player;

use std::{sync::{
    atomic::{AtomicU32, AtomicU64, Ordering as MemOrdering},
    Arc, OnceLock,
}, time::{SystemTime, UNIX_EPOCH}};

use ahash::RandomState;
use dashmap::DashMap;
use mod_network::{
    components::{
        ActionState, ActionStateTimer, Bullet, CharacterKind, ClientId, DamageLog, Epoch,
        HealthPoint, LatLon, MovementState, MovementStateTimer, ObjectId, Player, StageKind,
        ViewState, ViewStateTimer,
    },
    protocol::{InitStagePacket, Packet, PullStagePacket, UdpDamageLogPacket},
};
use mod_parallelism::collections::Queue;
use mod_physics::{Ray, YCapsule};

use crate::{
    data::{clamp_x, clamp_z, get_character_attributes, get_stage_height, is_valid_position}, session::Session
};

pub use self::{data::*, event::*, player::*};

use super::formula::movement_formulas as formulas;

/// 게임 개발을 위한 테스트 게임 월드 입니다.
///
/// # Note
/// 테스트 게임 월드는 인원 제한이 없습니다.
///
#[derive(Debug)]
pub struct World {
    /// 현재 게임 월드의 시대 정보입니다.
    epoch: AtomicU64,
    /// 오브젝트 식별자를 생성하기 위한 카운터입니다.
    counter: AtomicU32,

    /// 게임 지형의 종류입니다.
    stage_kind: StageKind,

    /// 게임 월드에 참가한 세션 데이터입니다.
    sessions: DashMap<Arc<Session>, ObjectId, RandomState>,
    /// 게임 월드에 포함된 플레이어 캐릭터 데이터입니다.
    players: DashMap<ClientId, ServerPlayer, RandomState>,
    /// 게임 월드에 포함된 총알 데이터입니다.
    bullets: DashMap<ObjectId, ServerBullet, RandomState>,

    /// 플레이어 데미지 로그입니다.
    damage_logs: Queue<DamageLog>,

    /// 게임 월드에 발생한 이벤트 대기열입니다.
    events: Queue<WorldEvents>,
}

impl World {
    /// 오브젝트 식별자를 생성합니다.
    pub fn generate_object_id(&self) -> ObjectId {
        let now = SystemTime::now();
        let duration = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let part_0 = (self.epoch.load(MemOrdering::Relaxed) & 0xFFF) as u32;
        let part_1 = self.counter.fetch_add(1, MemOrdering::AcqRel) & 0xFFF;
        let part_2 = duration.subsec_nanos() & 0xFF;

        ObjectId::new((part_0 << 24) | (part_1 << 12) | part_2)
    }

    /// 게임 월드의 인스턴스를 가져옵니다.
    pub fn get_instance() -> Arc<Self> {
        static INSTANCE: OnceLock<Arc<World>> = OnceLock::new();
        INSTANCE.get_or_init(|| Arc::new(World::default())).clone()
    }

    /// 현재 게임 월드 시대를 가져옵니다.
    pub fn get_current_epoch(&self) -> Epoch {
        Epoch::new(self.epoch.load(MemOrdering::Relaxed))
    }

    /// 게임 월드에 참가합니다.   
    /// NOTE: 플레이어의 캐릭터가 바로 생성되지 않습니다.
    pub fn join(&self, session: Arc<Session>, character_kind: CharacterKind) {
        // 오브젝트 식별자를 하나 할당하고, 세션 목록에 추가합니다.
        let client_id = session.client_id();
        let object_id = self.generate_object_id();
        self.sessions.insert(session, object_id);

        // 플레이어 생성 이벤트를 추가합니다.
        self.events
            .push(WorldEvents::AddPlayer(client_id, object_id, character_kind));
    }

    pub fn exit(&self, session: &Session) {
        self.sessions.remove(session);
        self.events
            .push(WorldEvents::RemovePlayer(session.client_id()));
    }

    /// 게임 월드 이벤트를 추가합니다.
    pub fn send_event(&self, event: WorldEvents) {
        self.events.push(event);
    }

    /// 게임 월드의 스냅샷을 생성합니다.
    fn create_snapshot(&self, epoch: Epoch, total_time_sec: f32) -> StageSnapshot {
        StageSnapshot {
            epoch,
            total_time_sec,
            stage_kind: self.stage_kind,
            players: self
                .players
                .iter()
                .map(|player| Player {
                    object_id: player.object_id,
                    character_kind: player.character_kind,
                    health_point: player.health_point,
                    translation: player.translation.to_array(),
                    rotation: player.rotation.to_array(),
                    velocity: player.velocity.to_array(),
                    action_state: player.action_state,
                    action_state_timer: player.action_state_timer,
                    movement_state: player.movement_state,
                    movement_state_timer: player.movement_state_timer,
                    view_state: player.view_state,
                    view_state_timer: player.view_state_timer,
                    view_rotation: player.view_rotation,
                })
                .collect(),
            bullets: self
                .bullets
                .iter()
                .map(|bullet| Bullet {
                    object_id: bullet.object_id,
                    shooter_id: bullet.shooter_id,
                    bullet_kind: bullet.bullet_kind,
                    translation: bullet.translation.to_array(),
                    rotation: bullet.rotation.to_array(),
                    velocity: bullet.velocity.to_array(),
                    remaining_distance: bullet.remaining_distance,
                })
                .collect(),
        }
    }

    /// 생성된 스냅샷 데이터를 각 클라이언트에 전송합니다.
    fn broadcast(&self, snapshot: StageSnapshot) {
        while !self.damage_logs.is_empty() {
            let mut count = UdpDamageLogPacket::capacity();
            let mut logs = Vec::with_capacity(count);
            while count > 0 {
                match self.damage_logs.pop() {
                    Some(log) => logs.push(log),
                    None => break,
                };
                count -= 1;
            }

            let packet = UdpDamageLogPacket::new(snapshot.epoch, logs);
            for session in self.sessions.iter() {
                match self.players.get_mut(&session.key().client_id()) {
                    Some(item) => {
                        if item.epoch != Epoch::new(0) {
                            session.key().tcp_write(packet.as_raw());
                        }
                    }
                    None => continue,
                }
            }
        }

        let mut init_stage_packet = InitStagePacket::new(
            self.stage_kind,
            snapshot.epoch,
            ObjectId::NULL,
            snapshot.players.clone(),
        );
        let pull_stage_packet =
            PullStagePacket::new(snapshot.epoch, snapshot.players, snapshot.bullets);
        for session in self.sessions.iter() {
            match self.players.get_mut(&session.key().client_id()) {
                Some(mut item) => {
                    if item.epoch == Epoch::new(0) {
                        item.epoch = snapshot.epoch;
                        init_stage_packet.object_id = *session.value();
                        session.key().tcp_write(init_stage_packet.as_raw());
                    } else {
                        session.key().tcp_write(pull_stage_packet.as_raw());
                    }
                }
                None => continue,
            }
        }
    }

    /// 플레이어 캐릭터를 추가합니다.
    fn add_player(&self, client_id: ClientId, object_id: ObjectId, character_kind: CharacterKind) {
        let attributes = get_character_attributes(character_kind);
        self.players.insert(
            client_id,
            ServerPlayer {
                epoch: Epoch::new(0),
                object_id,
                character_kind,
                health_point: HealthPoint(attributes.health_point),
                translation: glam::Vec3A::ZERO,
                rotation: glam::Quat::IDENTITY,
                velocity: glam::Vec3A::ZERO,
                direction: glam::Vec3A::Z,
                action_state: ActionState::default(),
                prev_action_state: ActionState::default(),
                action_state_timer: ActionStateTimer::default(),
                movement_state: MovementState::default(),
                movement_state_timer: MovementStateTimer::default(),
                view_state: ViewState::default(),
                view_state_timer: ViewStateTimer::default(),
                view_rotation: LatLon::default(),
                shot_count: 0,
            },
        );
    }

    /// 플레이어 캐릭터를 제거합니다.
    fn remove_player(&self, client_id: ClientId) {
        self.players.remove(&client_id);
    }

    /// 클라이언트에서 보내온 플레이어 정보로 업데이트
    fn update_player_status(
        &self,
        epoch: Epoch,
        client_id: ClientId,
        rotation: glam::Quat,
        direction: glam::Vec3A,
        action_state: ActionState,
        movement_state: MovementState,
        view_state: ViewState,
        view_rotation: LatLon,
    ) {
        if let Some(mut player) = self.players.get_mut(&client_id) {
            let attributes = get_character_attributes(player.character_kind);
            player.epoch = epoch;
            player.rotation = rotation;
            player.direction = direction;
            player.action_state = action_state;
            player.movement_state = movement_state;
            player.view_state = view_state;
            player.view_rotation = view_rotation;

            if player.action_state != player.prev_action_state {
                player.action_state_timer.0 = 0.0;
            }

            // 플레이어 총알 발사 확인
            match player.action_state {
                ActionState::Attack => {
                    if player.shot_count < attributes.normal_attack_count {
                        let index = player.shot_count as usize;
                        let timing = attributes.normal_attack_timing[index];
                        if timing <= player.action_state_timer.0 {
                            player.shot_count += 1;
                            self.events.push(WorldEvents::AddBullet(*player.key()));
                        }
                    }
                }
                _ => {}
            }

            player.prev_action_state = player.action_state;
        }
    }

    /// 총알 오브젝트를 추가합니다.
    fn add_bullet(&self, shooter_id: ClientId) {
        if let Some(player) = self.players.get(&shooter_id.into()) {
            let attributes = get_character_attributes(player.character_kind);

            // 각도의 파라미터를 계산합니다.
            let latitude = player.view_rotation.lat;
            let t = ((latitude + LatLon::LATITUDE_HALF_RANGE) / LatLon::LATITUDE_RANGE).clamp(0.0, 1.0);

            // 총구가 바라보는 방향을 계산합니다.
            let mut direction = glam::Vec3A::from(attributes.get_muzzle_direction(t));
            direction = direction.normalize_or(glam::Vec3A::Z);

            // let mut transform = glam::Mat4::from_translation(glam::Vec3::NEG_Z);
            let rotate = glam::Mat4::from_rotation_y(player.view_rotation.lon);
            direction = rotate.transform_vector3a(direction);
            let velocity = direction * 50.0;
            let rotation = glam::Quat::from_rotation_arc(glam::Vec3::Z, direction.into());

            // 총구의 위치를 계산합니다.
            let offset = glam::Vec3::from(attributes.get_muzzle_position(t));
            let mut offset = glam::Mat4::from_translation(offset);
            let rotate = glam::Mat4::from_rotation_y(player.view_rotation.lon);
            offset = rotate * offset;
            let offset = glam::Vec3A::from_vec4(offset.w_axis);
            let translation = player.translation + offset;

            let object_id = self.generate_object_id();
            self.bullets.insert(
                object_id,
                ServerBullet {
                    object_id,
                    shooter_id,
                    bullet_kind: player.character_kind.into(),
                    translation,
                    rotation,
                    velocity,
                    remaining_distance: attributes.attack_range as f32,
                },
            );
        }
    }

    /// 총알 오브젝트를 제거합니다.
    fn remove_bullet(&self, object_id: ObjectId) {
        self.bullets.remove(&object_id);
    }

    /// 이벤트를 처리합니다.
    fn handle_events(&self) {
        while let Some(event) = self.events.pop() {
            match event {
                WorldEvents::AddPlayer(client_id, object_id, character_kind) => {
                    self.add_player(client_id, object_id, character_kind)
                }
                WorldEvents::UpdatePlayerStatus(
                    epoch,
                    client_id,
                    rotation,
                    direction,
                    action_state,
                    movement_state,
                    view_state,
                    view_rotation,
                ) => self.update_player_status(
                    epoch,
                    client_id,
                    rotation,
                    direction,
                    action_state,
                    movement_state,
                    view_state,
                    view_rotation,
                ),
                WorldEvents::AddBullet(client_id) => self.add_bullet(client_id),
                WorldEvents::RemovePlayer(client_id) => self.remove_player(client_id),
                WorldEvents::RemoveBullet(object_id) => self.remove_bullet(object_id),
            }
        }
    }

    /// 주어진 시간 간격으로 게임 월드를 갱신합니다.
    fn update(&self, elapsed_time_sec: f32) {
        // NOTE: 이부분은 나중에 글로벌상수로 따로 정의하는게 좋아보이는데, 테스트를 위해 일단 여기에 작성
        const PLAYER_RADIUS: f32 = 0.25;
        const PLAYER_HEIGHT: f32 = 1.0;

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

                let attributes = get_character_attributes(player.character_kind);

                // 충돌 처리: 플레이어 - 총알
                // 플레이어의 충돌체: YCapsule(총알의 크기 만큼 확대)           나중에 세분화
                // 총알은 점으로 raycasting

                let mut center = player.translation;
                center[1] -= attributes.bullet_radius;

                // mod-network의 Player에 make_collider()를 추가해서 클라이언트에서도 표시할 수 있도록 해도 좋아보임.
                let player_capsule = YCapsule {
                    center: glam::Vec3::from(center),
                    radius: PLAYER_RADIUS + attributes.bullet_radius,
                    height: PLAYER_HEIGHT + attributes.bullet_radius * 2.0,
                };

                if let Some(dist) = ray.intersect(&player_capsule) {
                    if dist <= move_distance {
                        println!("Bullet find player (player id: {:?})", player.object_id);
                        if dist < nearest_distance {
                            nearest_distance = dist;
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
                    let char_info = get_character_attributes(player.character_kind);

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
                    let atk = char_info.attack_power as f32;
                    let def = char_info.defense_power as f32;
                    let dmg = formulas::default_damage(atk, def, 100.0);

                    // 치명타 계산
                    let crit = char_info.critical_rate as f32;
                    let crit_rate = formulas::cal_crt_rate(rand::random::<f32>(), crit, 250.0);
                    if crit_rate == 1.0 {
                        println!("  - critical!");
                    }

                    // 최종 데미지 계산
                    let crit_dam = char_info.critical_damage as f32;
                    let final_dmg =
                        formulas::final_damage(dmg, hit_rate, crit_rate, crit_dam).ceil() as u32;

                    player.health_point.0 = (player.health_point.0 - final_dmg).max(0);
                    self.damage_logs.push(DamageLog {
                        object_id: player.object_id,
                        damage: HealthPoint(final_dmg),
                    });
                    println!("  - hp: {:?}(-{})", player.health_point.0, final_dmg);
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
                self.events.push(WorldEvents::RemoveBullet(*bullet.key()));
            }
        }
    }

    /// 플레이어 상태 타이머를 갱신합니다.
    fn update_player_state_timer(&self, elapsed_time_sec: f32) {
        for mut player in self.players.iter_mut() {
            let attributes = get_character_attributes(player.character_kind);
            update_character_action_state_timer(&attributes, &mut player, elapsed_time_sec);
            update_character_movement_state_timer(&attributes, &mut player, elapsed_time_sec);
        }
    }

    /// 주어진 시간 간격으로 플레이어의 위치를 갱신합니다.
    fn update_player_position(&self, elapsed_time_sec: f32) {
        for mut player in self.players.iter_mut() {
            let attributes = get_character_attributes(player.character_kind);

            // 플레이어 이동 벡터 계산
            let velocity = match player.movement_state {
                MovementState::Moving => player.direction * attributes.speed,
                _ => glam::Vec3A::ZERO,
            };
            player.velocity.x = velocity.x;
            player.velocity.z = velocity.z;
            // 중력 누적
            player.velocity.y += -9.8 * elapsed_time_sec;

            // 이동 시도 (이동 전 위치 저장)
            let mut new_p = player.translation + player.velocity * elapsed_time_sec;

            // 기존 영역과 현재 영역을 인자로 넘겨서 x, z중 어느 값이 넘어갔는지 확인
            // 아니면 x만 이동했을때의 영역과 z만 이동했을때의 영역을 보고, 유효한 영역일때만 이동시키도록?
            // 유효한 영역이 아니라면 현재 영역의 가장 가장자리 부분으로 clamp하기
            if !is_valid_position(self.stage_kind, new_p.x, player.translation.z) {
                player.velocity.x = 0.0;
                new_p.x = clamp_x(self.stage_kind, player.translation.x, new_p.x);
            }
            if !is_valid_position(self.stage_kind, player.translation.x, new_p.z) {
                player.velocity.z = 0.0;
                new_p.z = clamp_z(self.stage_kind, player.translation.z, new_p.z);
            }

            new_p = player.translation + player.velocity * elapsed_time_sec;

            if let Some(height) = get_stage_height(self.stage_kind, new_p.x, new_p.z) {
                if height >= new_p.y {
                    new_p.y = height;
                    player.velocity.y = 0.0;
                }
                player.translation = new_p;
            }
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self {
            epoch: AtomicU64::new(0),
            counter: AtomicU32::new(0),
            stage_kind: StageKind::default(),
            sessions: DashMap::default(),
            players: DashMap::default(),
            bullets: DashMap::default(),
            damage_logs: Queue::default(),
            events: Queue::default(),
        }
    }
}

/// 고정 시간 갱신 간격입니다.
const INTERVAL: f32 = 1.0 / 120.0; // 120 FPS

/// 게임 월드를 갱신하는 루프 함수입니다.
pub async fn update_game_world(world: Arc<World>) {
    let mut total_time_sec = 0.0;
    let mut previous_time_point = tokio::time::Instant::now();
    loop {
        // 시대를 증가시킵니다.
        let epoch = Epoch::new(world.epoch.fetch_add(1, MemOrdering::Release));

        // 경과 시간을 계산합니다.
        let current_time_point = tokio::time::Instant::now();
        let mut elapsed_time_sec = current_time_point
            .saturating_duration_since(previous_time_point)
            .as_secs_f32();
        previous_time_point = current_time_point;
        total_time_sec += elapsed_time_sec;

        // 게임 월드를 갱신합니다.
        world.handle_events();
        while elapsed_time_sec > INTERVAL {
            world.update(INTERVAL);
            total_time_sec += INTERVAL;
            elapsed_time_sec -= INTERVAL;
        }
        world.update(elapsed_time_sec);

        // 게임 월드의 스냅샷을 생성합니다.
        let snapshot = world.create_snapshot(epoch, total_time_sec);

        // 모든 세션에 게임 월드 데이터를 전송합니다.
        world.broadcast(snapshot);

        // 다른 태스크들이 실행될 기회를 주기 위해 양보
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
}
