use std::collections::VecDeque;

use ahash::{HashMap, RandomState};
use hecs::Entity;
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterAttributes, CharacterKind, LatLon, MovementState,
    MovementStateTimer, Velocity, MAX_IN_GAME_PLAYERS, MAX_JUMP_DURATION, RESPAWN_DELAY,
};

use crate::component::CHARACTER_ATTRIBUTES;

/// 서버에서 받은 패킷 데이터의 스냅샷입니다.
#[derive(Debug, Clone)]
pub struct EntitySnapshot {
    pub time_stamp_ms: u32,
    pub velocity: Velocity,
    pub transform: glam::Mat4,
    pub action_state: ActionState,
    pub action_state_timer: ActionStateTimer,
    pub movement_state: MovementState,
    pub movement_state_timer: MovementStateTimer,
    pub latlon: LatLon,
}

impl EntitySnapshot {
    /// 새로운 엔터티 스냅샷을 생성합니다.
    pub fn new(
        time_stamp_ms: u32,
        velocity: [f32; 3],
        rotation: [f32; 4],
        translation: [f32; 3],
        action_state: ActionState,
        action_state_timer: ActionStateTimer,
        movement_state: MovementState,
        movement_state_timer: MovementStateTimer,
        latlon: LatLon,
    ) -> Self {
        Self {
            time_stamp_ms,
            velocity: Velocity(velocity.into()),
            transform: glam::Mat4::from_rotation_translation(
                glam::Quat::from_array(rotation),
                glam::Vec3::from_array(translation),
            ),
            action_state,
            action_state_timer,
            movement_state,
            movement_state_timer,
            latlon,
        }
    }
}

/// 스냅샷 저장 용량
pub const SNAPSHOT_CAPACITY: usize = 100;

/// 스냅샵을 모아 놓는 버퍼입니다.
#[derive(Debug, Clone)]
pub struct SnapshotBuffer<T> {
    snapshots: VecDeque<T>,
}

impl<T> SnapshotBuffer<T> {
    /// 새로운 스냅샷 버퍼를 생성합니다.
    pub fn new() -> Self {
        Self {
            snapshots: VecDeque::with_capacity(SNAPSHOT_CAPACITY + 1),
        }
    }

    /// 새로운 엔터티 스냅샷을 추가합니다.
    pub fn insert(&mut self, snapshot: T) {
        self.snapshots.push_back(snapshot);
        // 오래된 스냅샷을 제거합니다.
        while self.snapshots.len() > SNAPSHOT_CAPACITY {
            self.snapshots.pop_front();
        }
    }
}

/// 엔터티 스냅샷을 관리하고, 스냅샷의 선형 보간을 수행합니다.
#[derive(Debug, Clone)]
pub struct InterpolationManager {
    pub buffers: HashMap<Entity, (CharacterKind, SnapshotBuffer<EntitySnapshot>)>,
    interpolation_delay_ms: u32,
    max_extrapolation_ms: u32,
}

impl InterpolationManager {
    /// 새로운 스냅샷 관리자를 생성합니다.
    pub fn new() -> Self {
        Self {
            buffers: HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new()),
            interpolation_delay_ms: 100,
            max_extrapolation_ms: 250,
        }
    }

    pub fn get_interpolated(
        &self,
        entity: Entity,
        time_stamp_ms: u32,
    ) -> Option<(
        glam::Mat4,
        ActionState,
        ActionStateTimer,
        MovementState,
        MovementStateTimer,
        LatLon,
    )> {
        let (character_kind, buffer) = self.buffers.get(&entity)?;
        let time_stamp_ms = time_stamp_ms.saturating_sub(self.interpolation_delay_ms);

        let (prev, next) = find_snapshots(&buffer.snapshots, time_stamp_ms);

        let i = *character_kind as usize;
        let character_attributes = CHARACTER_ATTRIBUTES[i];
        match (prev, next) {
            (Some(prev), Some(next)) => {
                // Interpolation
                let elapsed_time_ms = time_stamp_ms - prev.time_stamp_ms;
                let t = elapsed_time_ms as f32 / (next.time_stamp_ms - prev.time_stamp_ms) as f32;
                let transform = prev.transform * (1.0 - t) + next.transform * t;
                let lat = prev.latlon.lat * (1.0 - t) * next.latlon.lat * t;
                let lon = prev.latlon.lon * (1.0 - t) * next.latlon.lon * t;
                let latlon = LatLon::new(lat, lon);

                let (action_state, action_state_timer) = action_state_interpolated(
                    prev.action_state,
                    prev.action_state_timer,
                    next.action_state,
                    character_attributes,
                    elapsed_time_ms as u16,
                );
                let (movement_state, movement_state_timer) = movement_state_interpolated(
                    prev.movement_state,
                    prev.movement_state_timer,
                    next.movement_state,
                    character_attributes,
                    elapsed_time_ms as u16,
                );

                Some((
                    transform,
                    action_state,
                    action_state_timer,
                    movement_state,
                    movement_state_timer,
                    latlon,
                ))
            }
            (Some(prev), None) => {
                // Extrapolation
                let elapsed_time_ms =
                    (time_stamp_ms - prev.time_stamp_ms).clamp(0, self.max_extrapolation_ms);
                let elapsed_time_sec = elapsed_time_ms as f32 / 1000.0;
                let distance = prev.velocity.0 * elapsed_time_sec;
                let mut transform = prev.transform;
                transform.w_axis += glam::Vec4::new(distance.x, distance.y, distance.z, 0.0);

                let (action_state, action_state_timer) = action_state_extrapolation(
                    prev.action_state,
                    prev.action_state_timer,
                    character_attributes,
                    elapsed_time_ms as u16,
                );
                let (movement_state, movement_state_timer) = movement_state_extrapolation(
                    prev.movement_state,
                    prev.movement_state_timer,
                    character_attributes,
                    elapsed_time_ms as u16,
                );

                Some((
                    transform,
                    action_state,
                    action_state_timer,
                    movement_state,
                    movement_state_timer,
                    prev.latlon,
                ))
            }
            _ => None,
        }
    }
}

/// 주어진 `time_point_ms`에 해당하는 엔터티 스냅샷 썅을 찾습니다.
fn find_snapshots<'a>(
    snapshots: &'a VecDeque<EntitySnapshot>,
    time_stamp_ms: u32,
) -> (Option<&'a EntitySnapshot>, Option<&'a EntitySnapshot>) {
    let mut prev = None;
    for snapshot in snapshots {
        if snapshot.time_stamp_ms > time_stamp_ms {
            return (prev, Some(snapshot));
        }
        prev = Some(snapshot);
    }
    (prev, None)
}

/// [`ActionState`]와 [`ActionStateTimer`]를 보간합니다.
fn action_state_interpolated(
    prev_state: ActionState,
    prev_state_timer: ActionStateTimer,
    next_state: ActionState,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> (ActionState, ActionStateTimer) {
    if prev_state == next_state {
        let action_state = next_state;
        let action_state_timer = ActionStateTimer(prev_state_timer.0 + elapsed_time_ms);
        (action_state, action_state_timer)
    } else {
        let mut action_state = prev_state;
        let mut action_state_timer = prev_state_timer;
        let duration = match prev_state {
            ActionState::Idle => character_attributes.normal_idle_duration,
            ActionState::Aiming => character_attributes.normal_idle_duration,
            ActionState::AimAt => character_attributes.normal_attack_start_duration,
            ActionState::AimOff => character_attributes.normal_attack_end_duration,
            ActionState::Attack => character_attributes.normal_attack_ing_duration,
            ActionState::Death => RESPAWN_DELAY,
            ActionState::Reload => character_attributes.normal_reload_duration,
            ActionState::Skill => character_attributes.skill_duration,
            _ => unreachable!(),
        };
        action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

        let diff_t = action_state_timer.0 as i32 - duration as i32;
        if diff_t >= 0 {
            action_state = next_state;
            action_state_timer.0 = diff_t as u16;
        }

        (action_state, action_state_timer)
    }
}

/// [`ActionState`]와 [`ActionStateTimer`]를 예측합니다.
fn action_state_extrapolation(
    mut action_state: ActionState,
    mut action_state_timer: ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> (ActionState, ActionStateTimer) {
    match action_state {
        ActionState::Idle => {
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms) % duration;
        }
        ActionState::Aiming => {
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms) % duration;
        }
        ActionState::AimAt => {
            let duration = character_attributes.normal_attack_start_duration;
            action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

            let diff_t = action_state_timer.0 as i32 - duration as i32;
            if diff_t >= 0 {
                let duration = character_attributes.normal_idle_duration;
                action_state = ActionState::Aiming;
                action_state_timer.0 = diff_t as u16 % duration;
            }
        }
        ActionState::AimOff => {
            let duration = character_attributes.normal_attack_end_duration;
            action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

            let diff_t = action_state_timer.0 as i32 - duration as i32;
            if diff_t >= 0 {
                let duration = character_attributes.normal_idle_duration;
                action_state = ActionState::Idle;
                action_state_timer.0 = diff_t as u16 % duration;
            }
        }
        ActionState::Attack => {
            let duration = character_attributes.normal_attack_ing_duration;
            action_state_timer.0 = action_state_timer
                .0
                .saturating_add(elapsed_time_ms)
                .min(duration);
        }
        ActionState::Death => {
            let duration = RESPAWN_DELAY;
            action_state_timer.0 = action_state_timer
                .0
                .saturating_add(elapsed_time_ms)
                .min(duration);
        }
        ActionState::Reload => {
            let duration = character_attributes.normal_reload_duration;
            action_state_timer.0 = action_state_timer
                .0
                .saturating_add(elapsed_time_ms)
                .min(duration);
        }
        ActionState::Skill => {
            let duration = character_attributes.skill_duration;
            action_state_timer.0 = action_state_timer
                .0
                .saturating_add(elapsed_time_ms)
                .min(duration);
        }
        _ => unreachable!(),
    };

    (action_state, action_state_timer)
}

/// [`MovementState`]와 [`MovementStateTimer`]를 보간합니다.
fn movement_state_interpolated(
    prev_state: MovementState,
    prev_state_timer: MovementStateTimer,
    next_state: MovementState,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> (MovementState, MovementStateTimer) {
    if prev_state == next_state {
        let movement_state = next_state;
        let movement_state_timer = MovementStateTimer(prev_state_timer.0 + elapsed_time_ms);
        (movement_state, movement_state_timer)
    } else {
        let mut movement_state = prev_state;
        let mut movement_state_timer = prev_state_timer;
        let duration = match prev_state {
            MovementState::Idle => character_attributes.normal_idle_duration,
            MovementState::Moving => character_attributes.move_ing_duration,
            MovementState::MoveToEnd => character_attributes.move_end_normal_duration,
            MovementState::Jumping | MovementState::Landing => {
                (movement_state_timer.0 + elapsed_time_ms).min(MAX_JUMP_DURATION)
            }
        };
        movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

        let diff_t = movement_state_timer.0 as i32 - duration as i32;
        if diff_t >= 0 {
            movement_state = next_state;
            movement_state_timer.0 = diff_t as u16;
        }

        (movement_state, movement_state_timer)
    }
}

/// [`MovementState`]와 [`MovementStateTimer`]를 예측합니다.
fn movement_state_extrapolation(
    mut movement_state: MovementState,
    mut movement_state_timer: MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> (MovementState, MovementStateTimer) {
    match movement_state {
        MovementState::Idle => {
            let duration = character_attributes.normal_idle_duration;
            movement_state_timer.0 =
                movement_state_timer.0.saturating_add(elapsed_time_ms) % duration;
        }
        MovementState::Moving => {
            let duration = character_attributes.move_ing_duration;
            movement_state_timer.0 =
                movement_state_timer.0.saturating_add(elapsed_time_ms) % duration;
        }
        MovementState::MoveToEnd => {
            let duration = character_attributes.move_end_normal_duration;
            movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

            let diff_t = movement_state_timer.0 as i32 - duration as i32;
            if diff_t >= 0 {
                let duration = character_attributes.normal_idle_duration;
                movement_state = MovementState::Idle;
                movement_state_timer.0 = diff_t as u16 % duration;
            }
        }
        MovementState::Jumping => {
            let duration = MAX_JUMP_DURATION;
            movement_state_timer.0 = movement_state_timer
                .0
                .saturating_add(elapsed_time_ms)
                .min(duration);
        }
        MovementState::Landing => {
            let duration = MAX_JUMP_DURATION;
            movement_state_timer.0 = movement_state_timer
                .0
                .saturating_add(elapsed_time_ms)
                .min(duration);
        }
    };

    (movement_state, movement_state_timer)
}
