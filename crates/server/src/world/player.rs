use mod_network::components::{
    ActionState, ActionStateTimer, CharacterAttributes, CharacterKind, CompressedState,
    HealthPoint, LatLon, MovementState, MovementStateTimer, ObjectId, Player, UserId, ViewState,
    ViewStateTimer, MAX_JUMP_DURATION, NUM_ACTION_STATES, NUM_MOVEMENT_STATES,
};

use crate::data::get_character_attributes;

use super::{BulletObject, GameWorld, GameWorldEvent};

/// 서버에서 관리하는 플레이어 오브젝트 데이터
#[derive(Debug, Clone)]
pub struct PlayerObject {
    /// 플레이어의 사용자 식별자
    user_id: UserId,

    /// 플레이어 캐릭터 종류
    character_kind: CharacterKind,
    /// 플레이어 캐릭터의 속성 데이터
    attributes: &'static CharacterAttributes,
    /// 플레이어 캐릭터 체력
    health_point: HealthPoint,
    /// 한 공격 당 총알 발사 횟수
    fired_per_attack: u32,
    /// 남은 총알의 개수
    remaining_bullets: u32,

    /// 플레이어 캐릭터의 월드 공간 위치
    translation: glam::Vec3A,
    /// 플레이어 캐릭터의 월드 공간 방향 (캐릭터가 움직이는 방향과 다를 수 있음)
    rotation: glam::Quat,
    /// 플레이어 속도
    velocity: glam::Vec3A,
    /// 플레이어 움직임 방향
    direction: glam::Vec3A,

    /// 플레이어 캐릭터 행동 상태
    action_state: ActionState,
    /// 플레이어 캐릭터 이전 행동 상태
    prev_action_state: ActionState,
    /// 플레이어 캐릭터 행동 상태 타이머
    action_state_timer: ActionStateTimer,
    /// 플레이어 캐릭터 움직임 상태
    movement_state: MovementState,
    /// 플레이어 캐릭터 움직임 상태 타이머
    movement_state_timer: MovementStateTimer,
    /// 플레이어 카메라 상태
    view_state: ViewState,
    /// 플레이어 카메라 상태 타이머
    view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터 중심으로 바라보는 방향
    view_rotation: LatLon,
}

impl PlayerObject {
    pub fn new(user_id: UserId, character_kind: CharacterKind) -> Self {
        let attributes = get_character_attributes(character_kind);
        Self {
            user_id,
            character_kind,
            attributes,
            health_point: HealthPoint(attributes.health_point),
            fired_per_attack: 0,
            remaining_bullets: 1,
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
        }
    }

    /// 플레이어 오브젝트의 사용자 식별자를 가져옵니다.
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// 플레이어 오브젝트의 캐릭터 속성을 가져옵니다.
    pub fn character_attributes(&self) -> &'static CharacterAttributes {
        self.attributes
    }

    /// 플레이어 오브젝트의 체력을 가져옵니다.
    pub fn health_mut(&mut self) -> &mut HealthPoint {
        &mut self.health_point
    }

    /// 플레이어 오브젝트의 위치를 가져옵니다.
    pub fn translation(&self) -> glam::Vec3A {
        self.translation
    }

    /// 플레이어 오브젝트의 위치를 가져옵니다.
    pub fn translation_mut(&mut self) -> &mut glam::Vec3A {
        &mut self.translation
    }

    /// 플레이어 오브젝트의 캐릭터 방향을 가져옵니다.
    pub fn rotation_mut(&mut self) -> &mut glam::Quat {
        &mut self.rotation
    }

    /// 플레이어 오브젝트의 속도를 가져옵니다.
    pub fn velocity_mut(&mut self) -> &mut glam::Vec3A {
        &mut self.velocity
    }

    /// 플레이어 오브젝트의 이동 방향을 가져옵니다.
    pub fn direction_mut(&mut self) -> &mut glam::Vec3A {
        &mut self.direction
    }

    /// 플레이어 오브젝트의 `ViewState`, `ViewStateTimer`, `Latlon`을 설정합니다.
    pub fn set_view(&mut self, state: ViewState, timer: ViewStateTimer, rotation: LatLon) {
        self.view_state = state;
        self.view_state_timer = timer;
        self.view_rotation = rotation;
    }

    /// 플레이어 오브젝트의 속도를 계산합니다.
    pub fn compute_velocity(&self) -> glam::Vec3A {
        type Func = fn(&PlayerObject) -> glam::Vec3A;
        const FUNC_TABLE: [Func; NUM_MOVEMENT_STATES] = [
            PlayerObject::velocity_when_idle,
            PlayerObject::velocity_when_moving,
            PlayerObject::velocity_when_move_to_end,
            PlayerObject::velocity_when_in_place_jumping,
            PlayerObject::velocity_when_in_place_landing,
            PlayerObject::velocity_when_moving_jumping,
            PlayerObject::velocity_when_moving_landing,
        ];

        let i = self.movement_state as usize;
        FUNC_TABLE[i](self)
    }

    /// `MovementState::Idle`일 때 속도를 가져옵니다.
    fn velocity_when_idle(&self) -> glam::Vec3A {
        glam::Vec3A::ZERO
    }

    /// `MovementState::Moving`일 떄 속도를 가져옵니다.
    fn velocity_when_moving(&self) -> glam::Vec3A {
        self.direction * self.attributes.speed
    }

    /// `MovementState::MoveToEnd`일 때 속도를 가져옵니다.
    fn velocity_when_move_to_end(&self) -> glam::Vec3A {
        glam::Vec3A::ZERO
    }

    /// `MovementState::InPlaceJumping`일 때 속도를 가져옵니다.
    fn velocity_when_in_place_jumping(&self) -> glam::Vec3A {
        let s = self.movement_state_timer.0 / MAX_JUMP_DURATION;
        glam::Vec3A::Y * 9.8 * 2.0 * (s * s * (3.0 - 2.0 * s))
    }

    /// `MovementState::InPlaceLanding`일 때 속도를 가져옵니다.
    fn velocity_when_in_place_landing(&self) -> glam::Vec3A {
        glam::Vec3A::ZERO
    }

    /// `MovementState::MovingJumping`일 때 속도를 가져옵니다.
    fn velocity_when_moving_jumping(&self) -> glam::Vec3A {
        let s = self.movement_state_timer.0 / MAX_JUMP_DURATION;
        self.direction * self.attributes.speed
            + glam::Vec3A::Y * 9.8 * 2.0 * (s * s * (3.0 - 2.0 * s))
    }

    /// `MovementState::MovingLanding`일 때 속도를 가져옵니다.
    fn velocity_when_moving_landing(&self) -> glam::Vec3A {
        self.direction * self.attributes.speed
    }

    /// 현재 플레이어 오브젝트의 데이터로 총알 오브젝트를 생성합니다.
    pub fn generate_bullet(&self, object_id: ObjectId, delay: f32) -> BulletObject {
        let t = (self.view_rotation.lat + LatLon::LATITUDE_HALF_RANGE) / LatLon::LATITUDE_RANGE;
        let rotate = glam::Mat4::from_rotation_y(self.view_rotation.lon);

        // 총구가 향하는 방향을 계산합니다.
        let mut direction =
            glam::Vec3A::from(self.attributes.get_muzzle_direction(t)).normalize_or(glam::Vec3A::Z);
        direction = rotate.transform_vector3a(direction);

        // 총알의 위치를 계산합니다.
        let translation = self.translation
            + rotate.transform_point3a(self.attributes.get_muzzle_position(t).into())
            + direction * delay;

        // 총알의 방향을 계산합니다.
        let rotation = glam::Quat::from_rotation_arc(glam::Vec3::Z, direction.into());

        // 총알의 발사 속도를 계산합니다.
        let velocity = direction * self.attributes.speed * 10.0;

        BulletObject {
            object_id,
            shooter_id: self.user_id,
            bullet_kind: self.character_kind.into(),
            translation,
            rotation,
            velocity,
            remaining_distance: self.attributes.attack_range as f32,
        }
    }

    /// 플레이어 오브젝트의 상태 타이머를 갱신합니다.
    pub fn update_state_timer(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        self.update_action_state_timer(world, elapsed_time_sec);
        self.update_movement_state_timer(world, elapsed_time_sec);
    }

    /// 플레이어 오브젝트의 `ActionState`를 가져옵니다.
    pub fn action_state(&self) -> ActionState {
        self.action_state
    }

    /// 클라이언트가 `ActionState` 변경을 시도합니다.   
    /// 해당 `ActionState`로 변경이 불가능할 경우 무시됩니다.
    pub fn change_action_state(&mut self, new: ActionState) {
        type Func = fn(&mut PlayerObject, ActionState);
        const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
            PlayerObject::change_action_state_when_idle,
            PlayerObject::change_action_state_when_aiming,
            PlayerObject::change_action_state_when_aim_at,
            PlayerObject::change_action_state_when_aim_off,
            PlayerObject::change_action_state_when_attack,
        ];

        let i = self.action_state as usize;
        FUNC_TABLE[i](self, new);
    }

    /// `ActionState::Idle`일 때 `ActionState` 변경을 시도합니다.  
    /// 해당 `ActionState`로 변경이 불가능할 경우 무시됩니다.
    fn change_action_state_when_idle(&mut self, new: ActionState) {
        // 변경 가능한 다음 상태
        // - ActionState::AimAt
        // - ActionState::Attack
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_this: &mut PlayerObject) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(this: &mut PlayerObject) {
            this.prev_action_state = this.action_state;
            this.action_state_timer.reset();
        }

        type Func = fn(&mut PlayerObject);
        const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
            (ActionState::Idle, maintain_timer), // `ActionState::Idle`
            (ActionState::Idle, maintain_timer), // `ActionState::Aiming`
            (ActionState::AimAt, reset_timer),   // `ActionState::AimAt`
            (ActionState::Idle, maintain_timer), // `ActionState::AimOff`
            (ActionState::Attack, reset_timer),  // `ActionState::Attack`
        ];

        let i = new as usize;
        let (next_state, timer_func) = TABLE[i];

        timer_func(self);
        self.action_state = next_state;
    }

    /// `ActionState::Aiming`일 때 `ActionState` 변경을 시도합니다.
    /// 해당 `ActionState`로 변경이 불가능할 경우 무시됩니다.
    fn change_action_state_when_aiming(&mut self, new: ActionState) {
        // 변경 가능한 다음 상태
        // - ActionState::AimOff
        // - ActionState::Attack
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_this: &mut PlayerObject) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(this: &mut PlayerObject) {
            this.prev_action_state = this.action_state;
            this.action_state_timer.reset();
        }

        type Func = fn(&mut PlayerObject);
        const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
            (ActionState::Aiming, maintain_timer), // `ActionState::Idle`
            (ActionState::Aiming, maintain_timer), // `ActionState::Aiming`
            (ActionState::Aiming, maintain_timer), // `ActionState::AimAt`
            (ActionState::AimOff, reset_timer),    // `ActionState::AimOff`
            (ActionState::Attack, reset_timer),    // `ActionState::Attack`
        ];

        let i = new as usize;
        let (next_state, timer_func) = TABLE[i];

        timer_func(self);
        self.action_state = next_state;
    }

    /// `ActionState::AimAt`일 때 `ActionState` 변경을 시도합니다.
    /// 해당 `ActionState`로 변경이 불가능할 경우 무시됩니다.
    fn change_action_state_when_aim_at(&mut self, new: ActionState) {
        // 변경 가능한 다음 상태
        // - ActionState::AimOff
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_this: &mut PlayerObject) {
            /* empty */
        }

        /// 타이머를 변환합니다.
        fn convert_timer(this: &mut PlayerObject) {
            this.prev_action_state = this.action_state;

            let length = this.attributes.normal_attack_start_duration;
            let s = this.action_state_timer.0 / length;

            let length = this.attributes.normal_attack_end_duration;
            this.action_state_timer.0 = s * length;
        }

        type Func = fn(&mut PlayerObject);
        const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
            (ActionState::AimAt, maintain_timer), // `ActionState::Idle`
            (ActionState::AimAt, maintain_timer), // `ActionState::Aiming`
            (ActionState::AimAt, maintain_timer), // `ActionState::AimAt`
            (ActionState::AimOff, convert_timer), // `ActionState::AimOff`
            (ActionState::AimAt, maintain_timer), // `ActionState::Attack`
        ];

        let i = new as usize;
        let (next_state, timer_func) = TABLE[i];

        timer_func(self);
        self.action_state = next_state;
    }

    /// `ActionState::AimOff`일 때 `ActionState` 변경을 시도합니다.
    /// 해당 `ActionState`로 변경이 불가능할 경우 무시됩니다.
    fn change_action_state_when_aim_off(&mut self, new: ActionState) {
        // 변경 가능한 다음 상태
        // - ActionState::AimAt
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_this: &mut PlayerObject) {
            /* empty */
        }

        /// 타이머를 변환합니다.
        fn convert_timer(this: &mut PlayerObject) {
            this.prev_action_state = this.action_state;

            let length = this.attributes.normal_attack_end_duration;
            let s = this.action_state_timer.0 / length;

            let length = this.attributes.normal_attack_start_duration;
            this.action_state_timer.0 = s * length;
        }

        type Func = fn(&mut PlayerObject);
        const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
            (ActionState::AimOff, maintain_timer), // `ActionState::Idle`
            (ActionState::AimOff, maintain_timer), // `ActionState::Aiming`
            (ActionState::AimOff, convert_timer),  // `ActionState::AimAt`
            (ActionState::AimOff, maintain_timer), // `ActionState::AimOff`
            (ActionState::AimOff, maintain_timer), // `ActionState::Attack`
        ];

        let i = new as usize;
        let (next_state, timer_func) = TABLE[i];

        timer_func(self);
        self.action_state = next_state;
    }

    /// `ActionState::Attack`일 때 `ActionState` 변경을 시도합니다.
    /// 해당 `ActionState`로 변경이 불가능할 경우 무시됩니다.
    fn change_action_state_when_attack(&mut self, _new: ActionState) {
        // 변경 가능한 다음 상태: 없음
        //
    }

    /// `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        type Func = fn(&mut PlayerObject, &GameWorld, f32);
        const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
            PlayerObject::update_action_state_timer_when_idle,
            PlayerObject::update_action_state_timer_when_aiming,
            PlayerObject::update_action_state_timer_when_aim_at,
            PlayerObject::update_action_state_timer_when_aim_off,
            PlayerObject::update_action_state_timer_when_attack,
        ];

        let i = self.action_state as usize;
        FUNC_TABLE[i](self, world, elapsed_time_sec);
    }

    /// `ActionState::Idle`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_idle(&mut self, _world: &GameWorld, elapsed_time_sec: f32) {
        // 타이머를 갱신합니다.
        let duration = self.attributes.normal_idle_duration;
        self.action_state_timer.0 = (self.action_state_timer.0 + elapsed_time_sec) % duration;
    }

    /// `ActionState::Aiming`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_aiming(&mut self, _world: &GameWorld, elapsed_time_sec: f32) {
        // 타이머를 갱신합니다.
        let duration = self.attributes.normal_idle_duration;
        self.action_state_timer.0 = (self.action_state_timer.0 + elapsed_time_sec) % duration;
    }

    /// `ActionState::AimAt`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_aim_at(&mut self, _world: &GameWorld, elapsed_time_sec: f32) {
        // 타이머를 갱신합니다.
        self.action_state_timer.0 += elapsed_time_sec;

        // 캐릭터의 `*_Normal_Attack_Start` 애니메이션 길이보다 클 경우 `ActionState`를 변경합니다.
        let duration = self.attributes.normal_attack_start_duration;
        let diff_t = self.action_state_timer.0 - duration;
        if diff_t >= 0.0 {
            self.prev_action_state = self.action_state;
            self.action_state = ActionState::Aiming;
            self.action_state_timer.0 = diff_t;
        }
    }

    /// `ActionState::AimOff`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_aim_off(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        self.action_state_timer.0 += elapsed_time_sec;

        // 캐릭터의 `*_Normal_Attack_End` 애니메이션 길이보다 클 경우 `ActionState`를 변경합니다.
        let duration = self.attributes.normal_attack_end_duration;
        let diff_t = self.action_state_timer.0 - duration;
        if diff_t >= 0.0 {
            self.prev_action_state = self.action_state;
            self.action_state = ActionState::Idle;
            self.action_state_timer.0 = diff_t;
        }
    }

    /// `ActionState::AimOff`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_attack(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        // 타이머를 갱신합니다.
        self.action_state_timer.0 += elapsed_time_sec;

        // 총알의 발사 시점을 가져옵니다.
        let duration = self.attributes.normal_attack_ing_duration;
        let attack_timing = &self.attributes.normal_attack_timing;
        let time_point = attack_timing
            .get(self.fired_per_attack as usize)
            .unwrap_or(&duration);

        // `ActionStateTimer`가 총알 발사 시점을 지났을 경우 게임 월드에 총알을 생성합니다.
        if self.action_state_timer.0 < duration
            && *time_point <= self.action_state_timer.0
            && self.remaining_bullets > 0
        {
            self.fired_per_attack += 1;
            // self.remaining_bullets -= 1;

            let shooter_id = self.user_id;
            let delay = self.action_state_timer.0 - *time_point;
            world.push_event(GameWorldEvent::AddBullet { shooter_id, delay });
        }

        // 캐릭터의 `*_Normal_Attack_End` 애니메이션 길이보다 클 경우 `ActionState`를 변경합니다.
        let diff_t = self.action_state_timer.0 - duration;
        if diff_t >= 0.0 {
            self.action_state = self.prev_action_state;
            self.action_state_timer.0 = diff_t;
            self.prev_action_state = ActionState::Attack;
            self.fired_per_attack = 0;
        }
    }

    /// 플레이어 오브젝트의 `MovementState`를 가져옵니다.
    pub fn movement_state(&self) -> MovementState {
        self.movement_state
    }

    /// 클라이언트가 `MovementState` 변경을 시도합니다.   
    /// 해당 `MovementState`로 변경이 불가능할 경우 무시됩니다.
    pub fn change_movement_state(&mut self, new: MovementState) {
        type Func = fn(&mut PlayerObject, MovementState);
        const FUNC_TABLE: [Func; NUM_MOVEMENT_STATES] = [
            PlayerObject::change_movement_state_when_idle,
            PlayerObject::change_movement_state_when_moving,
            PlayerObject::change_movement_state_when_move_to_end,
            PlayerObject::change_movement_state_when_in_place_jumping,
            PlayerObject::change_movement_state_when_in_place_landing,
            PlayerObject::change_movement_state_when_moving_jumping,
            PlayerObject::change_movement_state_when_moving_landing,
        ];

        let i = self.movement_state as usize;
        FUNC_TABLE[i](self, new);
    }

    /// `MovementState::Idle`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_idle(&mut self, new: MovementState) {
        // 변경 가능한 다음 상태
        // - MovementState::Moving
        // - MovementState::InPlaceJumping
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_: &CharacterAttributes, _: &mut ActionStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut ActionStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut ActionStateTimer);
        const TABLE: [[(MovementState, Func); NUM_MOVEMENT_STATES]; NUM_ACTION_STATES] = [
            // (`MovementState::Idle`, ActionState::Idle`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::Aiming`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::AimAt`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::AimOff`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::Attack`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
        ];

        let i = self.action_state as usize;
        let j = new as usize;
        let (next_state, timer_func) = TABLE[i][j];

        self.movement_state = next_state;
        timer_func(&self.attributes, &mut self.action_state_timer);
    }

    /// `MovementState::Moving`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_moving(&mut self, new: MovementState) {
        // 변경 가능한 다음 상태
        // - MovementState::Idle
        // - MovementState::MoveToEnd
        // - MovementState::MovingJumping
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_: &CharacterAttributes, _: &mut ActionStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut ActionStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut ActionStateTimer);
        const TABLE: [[(MovementState, Func); NUM_MOVEMENT_STATES]; NUM_ACTION_STATES] = [
            // (`MovementState::Moving`, `ActionState::Idle`)
            [
                (MovementState::MoveToEnd, reset_timer), // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::MoveToEnd, reset_timer), // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::Aiming`)
            [
                (MovementState::Idle, reset_timer),      // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),      // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::AimAt`)
            [
                (MovementState::Idle, reset_timer),      // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),      // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::AimOff`)
            [
                (MovementState::Idle, reset_timer),      // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),      // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::Attack`)
            [
                (MovementState::Idle, reset_timer),      // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),      // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
        ];

        let i = self.action_state as usize;
        let j = new as usize;
        let (next_state, timer_func) = TABLE[i][j];

        self.movement_state = next_state;
        timer_func(&self.attributes, &mut self.action_state_timer);
    }

    /// `MovementState::MoveToEnd`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_move_to_end(&mut self, new: MovementState) {
        // 변경 가능한 다음 상태
        // - MovementState::Moving
        // - MovementState::InPlaceJumping
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_: &CharacterAttributes, _: &mut ActionStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut ActionStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut ActionStateTimer);
        const TABLE: [[(MovementState, Func); NUM_MOVEMENT_STATES]; NUM_ACTION_STATES] = [
            // (`MovementState::MoveToEnd`, `ActionState::Idle`)
            [
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),       // `MovementState::Moving`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::Aiming`)
            [
                (MovementState::Idle, reset_timer),   // `MovementState::Idle`
                (MovementState::Moving, reset_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),   // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::InPlaceLanding`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::AimAt`)
            [
                (MovementState::Idle, reset_timer),   // `MovementState::Idle`
                (MovementState::Moving, reset_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),   // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::InPlaceLanding`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::AimOff`)
            [
                (MovementState::Idle, reset_timer),   // `MovementState::Idle`
                (MovementState::Moving, reset_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),   // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::InPlaceLanding`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::Attack`)
            [
                (MovementState::Idle, reset_timer),   // `MovementState::Idle`
                (MovementState::Moving, reset_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),   // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::InPlaceLanding`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingLanding`
            ],
        ];

        let i = self.action_state as usize;
        let j = new as usize;
        let (next_state, timer_func) = TABLE[i][j];

        self.movement_state = next_state;
        timer_func(&self.attributes, &mut self.action_state_timer);
    }

    /// `MovementState::InPlaceJumping`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_in_place_jumping(&mut self, _new: MovementState) {
        // 변경 가능한 다음 상태: 없음
        //
    }

    /// `MovementState::InPlaceLanding`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_in_place_landing(&mut self, _new: MovementState) {
        // 변경 가능한 다음 상태: 없음
        //
    }

    /// `MovementState::MovingJumping`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_moving_jumping(&mut self, _new: MovementState) {
        // 변경 가능한 다음 상태: 없음
        //
    }

    /// `MovementState::MovingLanding`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_moving_landing(&mut self, _new: MovementState) {
        // 변경 가능한 다음 상태: 없음
        //
    }

    /// `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_timer(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        type Func = fn(&mut PlayerObject, &GameWorld, f32);
        const FUNC_TABLE: [[Func; NUM_MOVEMENT_STATES]; NUM_ACTION_STATES] = [
            // `ActionState::Idle`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_moving,
                PlayerObject::update_movement_state_timer_when_move_to_end,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::Aiming`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_walking,
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::AimAt`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_walking,
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::AimOff`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_walking,
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::Attack`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_walking,
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
        ];

        let i = self.action_state as usize;
        let j = self.movement_state as usize;
        FUNC_TABLE[i][j](self, world, elapsed_time_sec);
    }

    /// `MovementState::Idle`일 때 `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_timer_when_idle(&mut self, _world: &GameWorld, elapsed_time_sec: f32) {
        // 타이머를 갱신합니다.
        let duration = self.attributes.normal_idle_duration;
        self.movement_state_timer.0 = (self.movement_state_timer.0 + elapsed_time_sec) % duration;
    }

    /// `ActionState::Idle`이고, `MovementState::Moving`일 때 `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_timer_when_moving(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        let duration = self.attributes.move_ing_duration;
        self.movement_state_timer.0 = (self.movement_state_timer.0 + elapsed_time_sec) % duration;
    }

    /// `MovementState::MoveToEnd`일 때 `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_timer_when_move_to_end(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        self.movement_state_timer.0 += elapsed_time_sec;

        // 캐릭터의 `*_Move_End_Normal` 애니메이션 길이보다 클 경우 `MovementState`를 갱신합니다.
        let duration = self.attributes.move_end_normal_duration;
        let diff_t = self.movement_state_timer.0 - duration;
        if diff_t >= 0.0 {
            self.movement_state = MovementState::Idle;
            self.movement_state_timer.0 = diff_t;
        }
    }

    /// `ActionState::Idle`이 아니고, `MovementState::Moving`일 때 `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_timer_when_walking(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        let duration = self.attributes.walk_duration;
        self.movement_state_timer.0 = (self.movement_state_timer.0 + elapsed_time_sec) % duration;
    }

    /// `MovemenetState::InPlaceJumping`일 때 `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_timer_when_in_place_jumping(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        self.movement_state_timer.0 += elapsed_time_sec;

        // 캐릭터의 점프 지속 시간보다 클 경우 `MovementState`를 갱신합니다.
        let diff_t = self.movement_state_timer.0 - MAX_JUMP_DURATION;
        if diff_t >= 0.0 {
            self.movement_state = MovementState::InPlaceLanding;
            self.movement_state_timer.reset();
        }
    }

    /// `MovemenetState::MovingJumping`일 때 `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_timer_when_moving_jumping(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        self.movement_state_timer.0 += elapsed_time_sec;

        // 캐릭터의 점프 지속 시간보다 클 경우 `MovementState`를 갱신합니다.
        let diff_t = self.movement_state_timer.0 - MAX_JUMP_DURATION;
        if diff_t >= 0.0 {
            self.movement_state = MovementState::MovingLanding;
            self.movement_state_timer.reset();
        }
    }

    /// `MovemenetState::InPlaceLanding` 또는 `MovementState::MovingLanding`일 때 `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_timer_when_landing(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        self.movement_state_timer.0 =
            (self.movement_state_timer.0 + elapsed_time_sec).min(MAX_JUMP_DURATION);
    }

    pub fn as_player(&self) -> Player {
        let compressed_state =
            CompressedState::compress(self.action_state, self.movement_state, self.view_state);

        Player {
            user_id: self.user_id,
            character_kind: self.character_kind,
            health_point: self.health_point,
            translation: self.translation.into(),
            rotation: self.rotation.into(),
            compressed_state,
            action_state_timer: self.action_state_timer,
            movement_state_timer: self.movement_state_timer,
            view_state_timer: self.view_state_timer,
            view_rotation: self.view_rotation,
            ..Default::default()
        }
    }
}
