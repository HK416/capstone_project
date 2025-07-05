use std::{collections::VecDeque, sync::Arc};

use ahash::{HashMap, HashSet, RandomState};
use mod_network::{
    components::{
        ActionState, BulletKind, DamageLogData, HeldInput, InGamePlayerPullData, InputKind,
        MAX_IN_GAME_PLAYERS, NetworkState, ObjectId, Permission, StageKind, StateChangeEvent, Team,
        UserId, update_action_state, update_action_state_timer, update_movement_state,
        update_movement_state_timer, update_player_rotation, update_player_translation,
    },
    protocol::{InGamePullPacket, InputEvent, JoinFailedReason, JoinRoomFailedPacket, Packet},
};
use rand::seq::SliceRandom;
use tokio::time::Duration;

use crate::{
    data::get_stage_attributes,
    entities::{Bullet, MAX_SNAPSHOTS, Player, PlayerSnapshot},
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
    /// 총알 오브젝트
    bullets: HashMap<ObjectId, Bullet>,
    /// 상태 이벤트 목록
    events: Option<HashMap<UserId, Vec<StateChangeEvent>>>,

    /// 플레이어 스냅샷 데이터
    player_snapshots: HashMap<UserId, VecDeque<PlayerSnapshot>>,
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
            bullets: HashMap::with_capacity_and_hasher(1024, RandomState::new()),
            events: None,
            player_snapshots: HashMap::with_capacity_and_hasher(
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

    /// [`GameWorldInGameRunStateEvent::InputEvent`] 이벤트를 처리합니다.
    fn handle_input_event(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        events: Vec<InputEvent>,
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

        let character_attributes = data.character_attributes();
        let stage_attributes = get_stage_attributes(self.stage_kind);
        let team = data.team();

        // 플레이어 데이터 스냅샷의 소유권을 가져옵니다.
        let mut buffer = match self.player_snapshots.remove(&uid) {
            Some(buffer) => buffer,
            None => {
                log::error!("Player({}) snapshot data not found in {}!", &uid, &world);
                eprintln!("Player({}) snapshot data not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 맨 처음 입력 이벤트를 가져옵니다.
        let mut iter = events.iter();
        let mut event = match iter.next() {
            Some(event) => event,
            None => {
                log::error!("Player({}) input events is empty!", &uid);
                eprintln!("Player({}) input events is empty!", &uid);
                session.close();
                return;
            }
        };

        // 이벤트 시작 스냅샷을 찾습니다.
        let mut select = None;
        for snapshot in buffer.iter() {
            if snapshot.play_elapsed_time_ms > event.play_elapsed_time_ms {
                break;
            }
            let interval = event.play_elapsed_time_ms - snapshot.play_elapsed_time_ms;
            select = Some((snapshot, interval));
        }

        if let Some((snapshot, interval)) = select
            && event.play_elapsed_time_ms < self.play_elapsed_time_ms
            && interval < 250
        {
            // 재 시뮬레이션을 진행합니다.
            data.action_state = snapshot.action_state;
            data.movement_state = snapshot.movement_state;
            data.action_state_timer = snapshot.action_state_timer;
            data.movement_state_timer = snapshot.movement_state_timer;
            data.latlon = snapshot.latlon;
            data.translation = snapshot.translation;
            data.rotation = snapshot.rotation;
            data.velocity = snapshot.velocity;
            data.direction = snapshot.direction;
            data.input_timer = snapshot.input_timer;
            data.held_input = snapshot.held_input;
            data.set_invincible(snapshot.is_invincible);
            data.set_grounded(snapshot.is_grounded);

            let mut play_elapsed_time_ms = snapshot.play_elapsed_time_ms;
            let mut elapsed_time_ms = interval;
            loop {
                if event.pressed {
                    data.held_input |= event.input.into_bits();
                } else {
                    data.held_input &= !event.input.into_bits();
                }

                let mut events = Vec::new();
                update_action_state(
                    data.held_input,
                    &mut data.action_state,
                    &mut data.action_state_timer,
                    character_attributes,
                    &mut data.bullet_data,
                    &mut data.skill_cost_data,
                    &mut events,
                );
                update_movement_state(
                    data.held_input,
                    data.action_state,
                    &mut data.movement_state,
                    &mut data.movement_state_timer,
                    &mut events,
                );

                data.input_timer
                    .update(data.held_input, elapsed_time_ms as u16);
                update_action_state_timer(
                    data.held_input,
                    &mut data.bullet_data,
                    &mut data.skill_cost_data,
                    &mut data.action_state,
                    &mut data.action_state_timer,
                    character_attributes,
                    elapsed_time_ms as u16,
                    &mut events,
                );
                update_movement_state_timer(
                    data.action_state,
                    &mut data.movement_state,
                    &mut data.movement_state_timer,
                    character_attributes,
                    elapsed_time_ms as u16,
                    &mut events,
                );

                data.direction.update(data.held_input, data.latlon);
                let mut look = data.rotation.mul_vec3a(glam::Vec3A::Z);
                look = update_player_rotation(
                    look,
                    data.action_state,
                    data.movement_state,
                    data.direction,
                    data.latlon,
                );

                let mut is_grounded = data.is_grounded();
                let mut is_invincible = data.is_invincible();
                let elapsed_time_sec = elapsed_time_ms as f32 / 1000.0;
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
                    &mut data.health_data,
                    data.input_timer,
                    elapsed_time_sec,
                );
                data.set_grounded(is_grounded);
                data.set_invincible(is_invincible);

                // 게임 플레이 경과 시간을 증가시킵니다.
                play_elapsed_time_ms += elapsed_time_ms;

                // 다음 이벤트를 가져옵니다.
                event = match iter.next() {
                    Some(event)
                        if event.play_elapsed_time_ms < self.play_elapsed_time_ms
                            && event.play_elapsed_time_ms < play_elapsed_time_ms =>
                    {
                        event
                    }
                    _ => break,
                };

                elapsed_time_ms = event.play_elapsed_time_ms - play_elapsed_time_ms;
            }
        } else {
            log::info!("no suitable Player({}) snapshot found!", &uid);
        }

        // 플레이어 데이터 스냅샷의 소유권을 돌려놓습니다.
        self.player_snapshots.insert(uid, buffer);
    }

    /// [`GameWorldInGameRunStateEvent::InputState`] 이벤트를 처리합니다.
    fn handle_input_state(
        &mut self,
        world: &mut GameWorld,
        session: Arc<Session>,
        uid: UserId,
        delta_x: f32,
        delta_y: f32,
        delta_z: f32,
        delta_lat: f32,
        delta_lon: f32,
        held_input: HeldInput,
        play_elapsed_time_ms: u32,
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

        data.held_input = held_input;
    }

    /// [`GameWorldInGameRunStateEvent::PlayerRespawn`] 이벤트를 처리합니다.
    fn handle_player_respawn_event(&mut self, uid: UserId) {}

    fn handle_bullet_spawn_event(
        &mut self,
        shooter_id: UserId,
        delay_time_ms: u16,
        bullet_kind: BulletKind,
        translation: glam::Vec3A,
        rotation: glam::Quat,
    ) {
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
                data.bullet_data.remaining,
                data.skill_cost_data.remaining,
                data.translation.to_array(),
                data.rotation.to_array(),
                data.velocity.0.to_array(),
                data.permission(),
                connected,
                data.is_grounded(),
                data.is_invincible(),
                data.network_state(),
                data.player_states(),
                data.action_state_timer,
                data.movement_state_timer,
                data.latlon,
            ));
        }

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        // 상태 변경 이벤트를 가져옵니다.
        let events = match self.events.take() {
            Some(events) => events,
            None => HashMap::default(),
        };

        let packet = InGamePullPacket::new(self.play_elapsed_time_ms, players, events);
        for session in world.sessions.keys() {
            session.tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldInGameRunState {
    /// 게임 월드를 갱신합니다.
    fn update(&mut self, world: &mut GameWorld, elapsed: Duration) {
        // 플레이어를 갱신합니다.
        for (&uid, data) in world.players.iter_mut() {
            // 서버와 연결이 끊어진 경우 건너뜁니다.
            if self.leaved_players.contains(&uid) {
                continue;
            }

            // 플레이어 상태 타이머를 갱신합니다.
            let elapsed_time_ms = elapsed.as_millis().min(MAX_GAME_TIME as u128) as u16;
            let character_attributes = data.character_attributes();
            let mut events = Vec::new();
            data.input_timer.update(data.held_input, elapsed_time_ms);
            update_action_state_timer(
                data.held_input,
                &mut data.bullet_data,
                &mut data.skill_cost_data,
                &mut data.action_state,
                &mut data.action_state_timer,
                character_attributes,
                elapsed_time_ms,
                &mut events,
            );
            update_movement_state_timer(
                data.action_state,
                &mut data.movement_state,
                &mut data.movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                &mut events,
            );
            data.input_timer.update(data.held_input, elapsed_time_ms);

            // 플레이어 스킬 코스트를 증가시킵니다.
            if data.action_state != ActionState::Death {
                data.skill_cost_timer = data.skill_cost_timer.saturating_add(elapsed_time_ms);
                if data.skill_cost_timer >= SKILL_COST_TICK {
                    let maximum = data.maximum_skill_cost();
                    let add = data.skill_cost_timer % SKILL_COST_TICK;
                    data.skill_cost_data.remaining =
                        (data.skill_cost_data.remaining.saturating_add(add)).min(maximum);
                }
            }

            // 플레이어 위치를 갱신합니다.
            let elapsed_time_sec = elapsed.as_secs_f32();
            let stage_attributes = get_stage_attributes(self.stage_kind);
            let team = data.team();
            let mut is_grounded = data.is_grounded();
            let mut is_invincible = data.is_invincible();
            let mut look = data.rotation.mul_vec3a(glam::Vec3A::Z);
            data.direction.update(data.held_input, data.latlon);
            look = update_player_rotation(
                look,
                data.action_state,
                data.movement_state,
                data.direction,
                data.latlon,
            );
            let z = look.normalize();
            let x = glam::Vec3A::Y.cross(z).normalize();
            let y = z.cross(x);
            let rot = glam::Mat3A::from_cols(x, y, z);
            data.rotation = glam::Quat::from_mat3(&rot.into());

            update_player_translation(
                stage_attributes,
                data.character_attributes(),
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
                &mut data.health_data,
                data.input_timer,
                elapsed_time_sec,
            );
            data.set_grounded(is_grounded);
            data.set_invincible(is_invincible);

            update_action_state(
                data.held_input,
                &mut data.action_state,
                &mut data.action_state_timer,
                character_attributes,
                &mut data.bullet_data,
                &mut data.skill_cost_data,
                &mut events,
            );
            update_movement_state(
                data.held_input,
                data.action_state,
                &mut data.movement_state,
                &mut data.movement_state_timer,
                &mut events,
            );
        }

        // 총알 오브젝트를 갱신합니다.
        // let mut removed_bullets = Vec::with_capacity(self.bullets.len());
        // for (&id, data) in self.bullets.iter_mut() {
        //     let result = update_bullet_translation(self.stage_kind, world, id, data, elapsed);
        //     if let Some(log) = result {
        //         self.damage_log_data.push(log);
        //     }

        //     if data.remaining_distance <= 0.0 {
        //         removed_bullets.push(id);
        //     }
        // }

        // 총알 오브젝트를 제거합니다.
        // while let Some(id) = removed_bullets.pop() {
        //     self.bullets.remove(&id);
        // }
    }

    /// 플레이어 스냅샷 데이터를 추가합니다.
    fn insert_snapshot(&mut self, uid: UserId, data: &Player) {
        // 스냅샷 버퍼의 소유권을 가져옵니다.
        let mut buffer = match self.player_snapshots.remove(&uid) {
            Some(buffer) => buffer,
            None => VecDeque::with_capacity(MAX_SNAPSHOTS + 1),
        };

        // 스냅샷 데이터를 추가합니다.
        buffer.push_back(PlayerSnapshot {
            play_elapsed_time_ms: self.play_elapsed_time_ms,
            action_state: data.action_state,
            movement_state: data.movement_state,
            action_state_timer: data.action_state_timer,
            movement_state_timer: data.movement_state_timer,
            latlon: data.latlon,
            translation: data.translation,
            rotation: data.rotation,
            velocity: data.velocity,
            direction: data.direction,
            input_timer: data.input_timer,
            held_input: data.held_input,
            is_invincible: data.is_invincible(),
            is_grounded: data.is_grounded(),
        });

        // 오래된 스냅샷을 제거합니다.
        while buffer.len() > MAX_SNAPSHOTS {
            buffer.pop_front();
        }

        // 스냅샷 버퍼의 소유권을 되돌려놓습니다.
        self.player_snapshots.insert(uid, buffer);
    }
}

impl GameWorldState for GameWorldInGameRunState {
    fn on_enter(&mut self, world: &mut GameWorld) {
        // 각 플레이어의 스냅샷을 생성합니다.
        for (&uid, data) in world.players.iter() {
            if self.leaved_players.contains(&uid) {
                continue;
            }
            self.insert_snapshot(uid, data);
        }

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
                GameWorldInGameRunStateEvent::InputEvent {
                    session,
                    uid,
                    events,
                } => self.handle_input_event(world, session, uid, events),
                GameWorldInGameRunStateEvent::InputState {
                    session,
                    uid,
                    delta_x,
                    delta_y,
                    delta_z,
                    delta_lat,
                    delta_lon,
                    held_input,
                    play_elapsed_time_ms,
                } => self.handle_input_state(
                    world,
                    session,
                    uid,
                    delta_x,
                    delta_y,
                    delta_z,
                    delta_lat,
                    delta_lon,
                    held_input,
                    play_elapsed_time_ms,
                ),
                GameWorldInGameRunStateEvent::PlayerRespawn {
                    uid,
                    play_elapsed_time_ms,
                } => todo!(),
                GameWorldInGameRunStateEvent::BulletSpawn {
                    shooter_id,
                    play_elapsed_time_ms,
                    bullet_kind,
                    translation,
                    rotation,
                } => todo!(),
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
        // 게임 플레이 경과 시간을 갱신합니다.
        let elapsed_time_ms = elapsed.as_millis().min(MAX_GAME_TIME as u128) as u32;
        self.play_elapsed_time_ms = self
            .play_elapsed_time_ms
            .saturating_add(elapsed_time_ms)
            .min(MAX_GAME_TIME);
        self.packet_send_elapsed_time_ms = self
            .packet_send_elapsed_time_ms
            .saturating_add(elapsed_time_ms);

        // 게임 월드를 갱신합니다.
        self.update(world, elapsed);

        // 일전 시각마다 패킷을 전송합니다.
        const TICK: u32 = 16;
        if self.packet_send_elapsed_time_ms >= TICK {
            self.packet_send_elapsed_time_ms = 0;
            self.broadcast(world);
        }

        // self.try_enter_next_state(world);
    }
}
