use mod_network::components::{
    ActionState, ActionStateTimer, CharacterAttributes, CharacterKind, ExSkillCost, GameInputBits,
    HealthPoint, LatLon, MAX_JUMP_DURATION, MovementState, MovementStateTimer, NUM_ACTION_STATES,
    NUM_MOVEMENT_STATES, ObjectId, Permission, RemainingBullet, Team, UserAccount, ViewState,
    ViewStateTimer,
};
use mod_physics::object3d::Capsule;

use crate::{
    data::get_character_attributes,
    world::{GameWorld, GameWorldEvent},
};

use super::BulletObject;

const PLAYER_RADIUS: f32 = 0.25;
const PLAYER_HEIGHT: f32 = 1.0;
/// 최대 입력 지속 시간
const MAX_INPUT_DURATION: f32 = 0.25;
/// 플레이어 리스폰 대기 시간
const RESPAWN_DELAY: f32 = 10.0;

/// 서버에서 관리하는 플레이어 오브젝트 데이터
#[derive(Debug, Clone)]
pub struct PlayerObject {
    /// 플레이어의 사용자 정보
    account: UserAccount,

    /// 여러 자료형의 데이터가 포함된 비트 필드입니다.  
    /// 아래와 같은 자료형이 포함되어있습니다.
    /// - index (3bit): 플레이어가 속한 팀 내의 인덱스
    /// - Team (1bit): 플레이어가 속한 팀
    /// - Permission (1bit): 플레이어 권한
    /// - bool (1bit): 다양한 용도로 사용되는 부울 플래그
    bitfield: u8,

    /// 플레이어 캐릭터 종류
    character_kind: CharacterKind,
    /// 플레이어 캐릭터의 속성 데이터
    attributes: &'static CharacterAttributes,
    /// 플레이어 캐릭터 체력
    health_point: HealthPoint,
    /// 한 공격 당 총알 발사 횟수
    fired_per_attack: u16,
    /// 남은 총알의 개수
    remaining_bullets: u16,

    // /// 현재 일반 스킬 쿨 타임
    // skill_cool_time: SkillKind,
    /// 현재 Ex 스킬 코스트 (최대 100.0)
    ex_skill_cost: f32,

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
    /// 입력 지속 시간 타이머입니다.
    /// 입력 지속 시간 타이머의 값이 `MAX_INPUT_DURATION`인 경우
    /// 플레이어 오브젝트는 최대 속력을 갖습니다.
    input_timer: f32,

    /// 플레이어 카메라 상태
    view_state: ViewState,
    /// 플레이어 카메라 상태 타이머
    view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터 중심으로 바라보는 방향
    view_rotation: LatLon,

    /// 플레이어 충돌체
    collider: Capsule,

    /// 땅을 밟고 있는지
    pub is_grounded: bool,
}

impl PlayerObject {
    /// 새로운 플레이어 오브젝트를 생성합니다.  
    pub fn new(account: UserAccount, permission: Permission, team: Team) -> Self {
        let team_bit = ((team as u8) & 0x1) << 3;
        let permission_bit = ((permission as u8) & 0x1) << 4;
        let flag_bit = ((false as u8) & 0x1) << 5;
        let bitfield = team_bit | permission_bit | flag_bit;

        let attributes = get_character_attributes(CharacterKind::default());
        // let skill_cool_time = match attributes.skill_cool_time {
        //     SkillKind::Active(_) => SkillKind::Active(0.0),
        //     SkillKind::Passive => SkillKind::Passive,
        // };

        Self {
            account,
            bitfield,
            character_kind: CharacterKind::default(),
            attributes,
            health_point: HealthPoint::default(),
            fired_per_attack: 0,
            remaining_bullets: attributes.max_bullets,
            // skill_cool_time,
            ex_skill_cost: 0.0,
            translation: glam::Vec3A::ZERO,
            rotation: glam::Quat::IDENTITY,
            velocity: glam::Vec3A::ZERO,
            direction: glam::Vec3A::Z,
            action_state: ActionState::default(),
            prev_action_state: ActionState::default(),
            action_state_timer: ActionStateTimer::default(),
            movement_state: MovementState::default(),
            movement_state_timer: MovementStateTimer::default(),
            input_timer: 0.0,
            view_state: ViewState::default(),
            view_state_timer: ViewStateTimer::default(),
            view_rotation: LatLon::default(),
            collider: Capsule::new(glam::Vec3::ZERO, PLAYER_HEIGHT, PLAYER_RADIUS),
            is_grounded: false,
        }
    }

    /// 리스폰시 호출하여 플레이어 오브젝트의 상태를 초기화합니다.
    pub fn reset_state(&mut self) {
        self.attributes = get_character_attributes(self.character_kind);
        self.health_point = HealthPoint::splat(self.attributes.health_point);
        self.fired_per_attack = 0;
        self.remaining_bullets = self.attributes.max_bullets;
        self.ex_skill_cost = 0.0;
        // self.skill_cool_time = match self.attributes.skill_cool_time {
        //     SkillKind::Active(_) => SkillKind::Active(0.0),
        //     SkillKind::Passive => SkillKind::Passive,
        // };
        self.action_state = ActionState::Idle;
        self.prev_action_state = ActionState::Idle;
        self.action_state_timer = ActionStateTimer::default();
        self.movement_state = MovementState::Idle;
        self.movement_state_timer = MovementStateTimer::default();
        self.view_state = ViewState::default();
        self.view_state_timer = ViewStateTimer::default();
        self.input_timer = 0.0;
    }

    /// 플레이어 오브젝트의 사용자 정보를 가져옵니다.
    pub fn account(&self) -> &UserAccount {
        &self.account
    }

    /// 플레이어가 속한 팀의 인덱스를 설정합니다.
    pub fn with_index(&mut self, index: usize) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x7 << 0)) | ((index as u8) & 0x7) << 0;
        self
    }

    /// 플레이어가 속한 팀의 인덱스를 가져옵니다.
    pub fn team_index(&self) -> usize {
        ((self.bitfield >> 0) & 0x7) as usize
    }

    /// 플레이어가 속한 팀을 설정합니다.
    #[allow(dead_code)]
    pub fn with_team(&mut self, team: Team) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 3)) | ((team as u8) & 0x1) << 3;
        self
    }

    /// 플레이어가 속한 팀을 가져옵니다.
    pub fn team(&self) -> Team {
        // Safe: 값이 범위를 벗어나지 않음
        let val = (self.bitfield >> 3) & 0x1;
        unsafe { Team::new(val).unwrap_unchecked() }
    }

    /// 플레이어의 권한을 설정합니다.
    pub fn with_permission(&mut self, permission: Permission) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 4)) | ((permission as u8) & 0x1) << 4;
        self
    }

    /// 플레이어의 권한을 가져옵니다.
    pub fn permission(&self) -> Permission {
        // Safe: 값이 범위를 벗어나지 않음
        let val = (self.bitfield >> 4) & 0x1;
        unsafe { Permission::new(val).unwrap_unchecked() }
    }

    /// 부울 플래그 변수의 값을 설정합니다.
    pub fn with_bool_flag(&mut self, flag: bool) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 5)) | ((flag as u8) & 0x1) << 5;
        self
    }

    /// 부울 플래그 변수의 값을 가져옵니다.
    pub fn bool_flag(&self) -> bool {
        (self.bitfield >> 5) & 0x1 == 0x1
    }

    /// 플레이어 캐릭터 종류를 가져옵니다.
    pub fn character_kind(&self) -> CharacterKind {
        self.character_kind
    }

    /// 플레이어 캐릭터 종류를 설정합니다.
    pub fn with_character_kind(&mut self, character_kind: CharacterKind) -> &mut Self {
        self.character_kind = character_kind;
        self.attributes = get_character_attributes(character_kind);
        self.health_point = HealthPoint::splat(self.attributes.health_point);
        self
    }

    /// 플레이어 오브젝트의 캐릭터 속성을 가져옵니다.
    pub fn character_attributes(&self) -> &'static CharacterAttributes {
        self.attributes
    }

    /// 플레이어 오브젝트 체력을 가져옵니다.
    pub fn health_point(&self) -> HealthPoint {
        self.health_point
    }

    /// 플레이어 오브젝트의 체력을 가져옵니다.
    pub fn health_point_mut(&mut self) -> &mut HealthPoint {
        &mut self.health_point
    }

    /// 남은 총알의 개수를 반환합니다.
    pub fn remaining_bullet(&self) -> RemainingBullet {
        RemainingBullet::new(self.remaining_bullets, self.attributes.max_bullets)
    }

    /// 현재 Ex스킬 코스트를 가져옵니다.
    pub fn get_ex_skill_cost(&self) -> ExSkillCost {
        ExSkillCost(self.ex_skill_cost)
    }

    /// Ex스킬 코스트를 더합니다.  
    /// Ex스킬 코스트의 값은 100을 넘지 못합니다.
    pub fn add_ex_skill_cost(&mut self, pt: f32) {
        self.ex_skill_cost = (self.ex_skill_cost + pt).min(100.0);
    }

    // /// 일반 스킬 쿨 타임을 가져옵니다.
    // pub fn get_skill_cool_time(&self) -> SkillKind {
    //     self.skill_cool_time
    // }

    // /// 일반 스킬 쿨 타임을 갱신합니다.
    // /// 일반 스킬의 유형이 "패시브"인 경우 이 함수는 아무 동작을 수행하지 않습니다.
    // pub fn update_skill_cool_time(&mut self, elapsed_time_sec: f32) {
    //     self.skill_cool_time = match self.skill_cool_time {
    //         SkillKind::Active(cool_time) => {
    //             SkillKind::Active((cool_time - elapsed_time_sec).max(0.0))
    //         }
    //         SkillKind::Passive => SkillKind::Passive,
    //     };
    // }

    /// 플레이엉 오브젝트의 위치를 설정합니다.
    pub fn with_translation<T>(&mut self, translation: T) -> &mut Self
    where
        T: Into<glam::Vec3A>,
    {
        self.translation = translation.into();
        self
    }

    /// 플레이어 오브젝트의 위치를 가져옵니다.
    pub fn translation(&self) -> glam::Vec3A {
        self.translation
    }

    /// 플레이어 오브젝트의 위치를 가져옵니다.
    pub fn translation_mut(&mut self) -> &mut glam::Vec3A {
        &mut self.translation
    }

    /// 플레이어 오브젝트 방향을 가져옵니다.
    pub fn rotation(&self) -> glam::Quat {
        self.rotation
    }

    /// 플레이어 오브젝트의 방향을 설정합니다.
    pub fn with_rotation<T>(&mut self, rotation: T) -> &mut Self
    where
        T: Into<glam::Quat>,
    {
        self.rotation = rotation.into();
        self
    }

    /// 플레이어 오브젝트의 방향을 설정합니다.
    pub fn set_rotation(&mut self, q: [f32; 4]) {
        let q = glam::Quat::from_array(q);
        self.rotation = q.normalize();
    }

    /// 플레이어 오브젝트의 이동 방향을 설정합니다.
    pub fn set_direction<T: Into<glam::Vec3A>>(&mut self, v: T) {
        let v: glam::Vec3A = v.into();
        self.direction = v.with_y(0.0).normalize_or(glam::Vec3A::Z);
    }

    /// 플레이어 행동 상태를 재설정합니다.
    pub fn reset_action_state(&mut self, state: ActionState) {
        self.action_state = state;
        self.action_state_timer.reset();
    }

    /// 플레이어 행동 상태를 가져옵니다.
    pub fn action_state(&self) -> ActionState {
        self.action_state
    }

    /// 플레이어 행동 상태 타이머를 가져옵니다.
    pub fn action_state_timer(&self) -> ActionStateTimer {
        self.action_state_timer
    }

    /// 플레이어 움직임 상태를 가져옵니다.
    pub fn movement_state(&self) -> MovementState {
        self.movement_state
    }

    /// 플레이어 움직임 상태 타이머를 가져옵니다.
    pub fn movement_state_timer(&self) -> MovementStateTimer {
        self.movement_state_timer
    }

    /// 플레이어 카메라 움직임 상태를 가져옵니다.
    pub fn view_state(&self) -> ViewState {
        self.view_state
    }

    /// 플레이어 카메라 움직임 상태 타이머를 가져옵니다.
    pub fn view_state_timer(&self) -> ViewStateTimer {
        self.view_state_timer
    }

    /// 플레이어 카메라가 캐릭터를 중심으로 회전한 각도를 설정합니다.
    pub fn with_view_rotation(&mut self, rotation: LatLon) -> &mut Self {
        self.view_rotation = rotation;
        self
    }

    /// 플레이어 카메라가 캐릭터를 중심으로 회전한 각도를 가져옵니다.
    pub fn view_rotation(&self) -> LatLon {
        self.view_rotation
    }

    /// 플레이어 오브젝트의 `ViewState`, `ViewStateTimer`, `Latlon`을 설정합니다.
    pub fn set_view(&mut self, state: ViewState, timer: ViewStateTimer, rotation: LatLon) {
        self.view_state = state;
        self.view_state_timer = timer;
        self.view_rotation = rotation;
    }

    /// 플레이어 오브젝트의 충돌체(캡슐)를 가져옵니다.
    pub fn collider(&self) -> Capsule {
        self.collider.clone()
    }

    /// 플레이어 오브젝트의 충돌체(캡슐)의 위치를 갱신합니다.
    pub fn update_collider(&mut self) {
        self.collider.center = self.translation.into();
    }

    /// 플레이어를 사망 상태로 설정합니다.
    pub fn death(&mut self) {
        self.action_state = ActionState::Dead;
        self.movement_state = MovementState::Idle;
        self.action_state_timer.reset();
        self.movement_state_timer.reset();
    }

    /// 현재 플레이어 오브젝트의 데이터로 총알 오브젝트를 생성합니다.
    pub fn generate_bullet(&self, object_id: ObjectId, delay: f32) -> BulletObject {
        let t = (self.view_rotation.lat + LatLon::LATITUDE_HALF_RANGE) / LatLon::LATITUDE_RANGE;
        let rotate = glam::Mat4::from_rotation_y(self.view_rotation.lon);

        // 총구가 향하는 방향을 계산합니다.
        let mut direction = glam::Vec3A::Z;
        direction = glam::Mat4::from_rotation_y(self.view_rotation.lon)
            .transform_vector3a(direction)
            .normalize_or(glam::Vec3A::Z);
        let right = glam::Vec3A::Y.cross(direction);
        direction = glam::Mat4::from_axis_angle(right.into(), self.view_rotation.lat)
            .transform_vector3a(direction);

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
            shooter_id: self.account.uid,
            shooter_team: self.team(),
            bullet_kind: self.character_kind.into(),
            translation,
            rotation,
            velocity,
            remaining_distance: self.attributes.attack_range as f32,
            radius: self.attributes.bullet_radius,
        }
    }

    /// 플레이어 오브젝트의 가속도를 가져옵니다.
    #[allow(dead_code)]
    pub fn acceleration(&self) -> glam::Vec3A {
        /// 영 벡터를 반환합니다.
        fn none_acceleration(_this: &PlayerObject) -> glam::Vec3A {
            glam::Vec3A::ZERO
        }

        /// 플레이어가 점프할 때 가속도를 반환합니다.
        fn jump_acceleration(this: &PlayerObject) -> glam::Vec3A {
            let s = 1.0 - this.movement_state_timer.0 / MAX_JUMP_DURATION;
            glam::Vec3A::Y * 9.8 * 5.0 * s
        }

        type Func = fn(&PlayerObject) -> glam::Vec3A;
        const FUNC_TABLE: [Func; NUM_MOVEMENT_STATES] = [
            none_acceleration, // `MovementState::Idle`
            none_acceleration, // `MovementState::Moving`
            none_acceleration, // `MovementState::MoveToEnd`
            jump_acceleration, // `MovementState::InPlaceJumping`
            none_acceleration, // `MovementState::InPlaceLanding`
            jump_acceleration, // `MovementState::MovingJumping`
            none_acceleration, // `MovementState::MovingLanding`
        ];

        let i = self.movement_state as usize;
        FUNC_TABLE[i](self)
    }

    /// 플레이어 오브젝트의 속도를 가져옵니다.
    pub fn velocity(&self) -> glam::Vec3A {
        self.velocity
    }

    /// 플레이어 오브젝트의 속도를 가져옵니다.
    pub fn velocity_mut(&mut self) -> &mut glam::Vec3A {
        &mut self.velocity
    }

    /// 플레이어 오브젝트의 속도를 갱신합니다.
    pub fn update_velocity(&mut self) {
        type Func = fn(&mut PlayerObject);
        const FUNC_TABLE: [[Func; NUM_MOVEMENT_STATES]; NUM_ACTION_STATES] = [
            // `ActionState::Idle`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_moving, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::Aiming`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_walking, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::AimAt`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_move_to_aim_move, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::AimOff`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_aim_move_to_move, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::Attack`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_walking, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::Dead
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_walking, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::Reload`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_walking, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::Skill`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_walking, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::ExSkill`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_walking, // `MovementState::Moving`
                PlayerObject::update_velocity_when_move_to_end, // `MovementState::MoveToEnd`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceJumping`
                PlayerObject::maintain_velocity,         // `MovementState::InPlaceLanding`
                PlayerObject::maintain_velocity,         // `MovementState::MovingJumping`
                PlayerObject::maintain_velocity,         // `MovementState::MovingLanding`
            ],
            // `ActionState::Callsign`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_idle, // `MovementState::Moving`
                PlayerObject::update_velocity_when_idle, // `MovementState::MoveToEnd`
                PlayerObject::update_velocity_when_idle, // `MovementState::InPlaceJumping`
                PlayerObject::update_velocity_when_idle, // `MovementState::InPlaceLanding`
                PlayerObject::update_velocity_when_idle, // `MovementState::MovingJumping`
                PlayerObject::update_velocity_when_idle, // `MovementState::MovingLanding`
            ],
            // `ActionState::VictoryStart`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_idle, // `MovementState::Moving`
                PlayerObject::update_velocity_when_idle, // `MovementState::MoveToEnd`
                PlayerObject::update_velocity_when_idle, // `MovementState::InPlaceJumping`
                PlayerObject::update_velocity_when_idle, // `MovementState::InPlaceLanding`
                PlayerObject::update_velocity_when_idle, // `MovementState::MovingJumping`
                PlayerObject::update_velocity_when_idle, // `MovementState::MovingLanding`
            ],
            // `ActionState::VictoryEnd`
            [
                PlayerObject::update_velocity_when_idle, // `MovementState::Idle`
                PlayerObject::update_velocity_when_idle, // `MovementState::Moving`
                PlayerObject::update_velocity_when_idle, // `MovementState::MoveToEnd`
                PlayerObject::update_velocity_when_idle, // `MovementState::InPlaceJumping`
                PlayerObject::update_velocity_when_idle, // `MovementState::InPlaceLanding`
                PlayerObject::update_velocity_when_idle, // `MovementState::MovingJumping`
                PlayerObject::update_velocity_when_idle, // `MovementState::MovingLanding`
            ],
        ];

        let i = self.action_state as usize;
        let j = self.movement_state as usize;
        FUNC_TABLE[i][j](self);
    }

    /// 플레이어 오브젝트의 속도를 유지합니다.
    fn maintain_velocity(&mut self) {
        /* empty */
    }

    /// `MovementState::Idle`일 때 속도를 갱신합니다.
    fn update_velocity_when_idle(&mut self) {
        let s = self.input_timer / MAX_INPUT_DURATION;
        let speed = 0.5 * self.attributes.speed * (s * s * (3.0 - 2.0 * s));
        let new_velocity = self.direction * speed;
        self.velocity.x = new_velocity.x;
        self.velocity.z = new_velocity.z;
    }

    /// `MovementState::MoveToEnd`일 때 속도를 갱신합니다.
    fn update_velocity_when_move_to_end(&mut self) {
        let s = self.input_timer / MAX_INPUT_DURATION;
        let speed = self.attributes.speed * (s * s * (3.0 - 2.0 * s));
        let new_velocity = self.direction * speed;
        self.velocity.x = new_velocity.x;
        self.velocity.z = new_velocity.z;
    }

    /// `ActionState::Idle`이고, `MovementState::Moving`일 때 속도를 갱신합니다.
    fn update_velocity_when_moving(&mut self) {
        let s = self.input_timer / MAX_INPUT_DURATION;
        let speed = self.attributes.speed * (s * s * (3.0 - 2.0 * s));
        let new_velocity = self.direction * speed;
        self.velocity.x = new_velocity.x;
        self.velocity.z = new_velocity.z;
    }

    /// `ActionState::Aiming`이고, `MovementState::Moving`일 때 속도를 갱신합니다.
    fn update_velocity_when_walking(&mut self) {
        let s = self.input_timer / MAX_INPUT_DURATION;
        let speed = 0.5 * self.attributes.speed * (s * s * (3.0 - 2.0 * s));
        let new_velocity = self.direction * speed;
        self.velocity.x = new_velocity.x;
        self.velocity.z = new_velocity.z;
    }

    /// `ActionState::AimAt`이고, `MovementState::Moving`일 때 속도를 갱신합니다.
    fn update_velocity_when_move_to_aim_move(&mut self) {
        let duration = self.attributes.normal_attack_start_duration;
        let s = 1.0 - self.movement_state_timer.0 / duration;
        let speed = (0.5 + 0.5 * s) * self.attributes.speed;
        let new_velocity = self.direction * speed;
        self.velocity.x = new_velocity.x;
        self.velocity.z = new_velocity.z;
    }

    /// `ActionState::AimOff`이고, `MovementState::Moving`일 때 속도를 갱신합니다.
    fn update_velocity_when_aim_move_to_move(&mut self) {
        let duration = self.attributes.normal_attack_end_duration;
        let s = self.movement_state_timer.0 / duration;
        let speed = (0.5 + 0.5 * s) * self.attributes.speed;
        let new_velocity = self.direction * speed;
        self.velocity.x = new_velocity.x;
        self.velocity.z = new_velocity.z;
    }

    /// 플레이어 오브젝트의 상태를 갱신합니다.
    pub fn update_state(&mut self, input_flags: GameInputBits) {
        if self.health_point.current == 0 {
            return;
        }
        self.update_action_state(input_flags);
        self.update_movement_state(input_flags);
    }

    /// 플레이어 오브젝트의 상태 타이머를 갱신합니다.
    pub fn update_state_timer(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        self.update_action_state_timer(world, elapsed_time_sec);
        self.update_movement_state_timer(world, elapsed_time_sec);
        self.update_input_timer(elapsed_time_sec);
        // self.update_skill_cool_time(elapsed_time_sec);
        self.add_ex_skill_cost(self.attributes.cost_recovery_rate * elapsed_time_sec);
    }

    /// 플레이어 오브젝트의 `ActionState`를 갱신합니다.
    fn update_action_state(&mut self, input_flags: GameInputBits) {
        type Func = fn(&mut PlayerObject, GameInputBits);
        const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
            PlayerObject::update_action_state_when_idle,
            PlayerObject::update_action_state_when_aiming,
            PlayerObject::update_action_state_when_aim_at,
            PlayerObject::update_action_state_when_aim_off,
            PlayerObject::update_action_state_when_attack,
            PlayerObject::update_action_state_when_dead,
            PlayerObject::update_action_state_when_reload,
            PlayerObject::update_action_state_when_skill,
            PlayerObject::update_action_state_when_ex_skill,
            PlayerObject::update_action_state_when_callsign,
            PlayerObject::update_action_state_when_victory_start,
            PlayerObject::update_action_state_when_victory_end,
        ];

        let i = self.action_state as usize;
        FUNC_TABLE[i](self, input_flags);
    }

    /// `ActionState::Idle`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_idle(&mut self, input_flags: GameInputBits) {
        // 우선순위
        // ExSkill << Skill << Attack << Reload << Aiming
        //
        if input_flags.contains(GameInputBits::ExSkill) {
            // 모든 코스트를 소모
            println!("cost: {}", self.ex_skill_cost);
            if self.ex_skill_cost == 100.0 {
                println!("ex_skill");
                self.ex_skill_cost = 0.0;

                self.prev_action_state = ActionState::Idle;
                self.action_state = ActionState::ExSkill;
                self.action_state_timer.reset();
            }
        } else if input_flags.contains(GameInputBits::Skill) {
            // if let SkillKind::Active(skill_cool_time) = self.skill_cool_time {
            //     println!("cool: {}", skill_cool_time);
            //     if skill_cool_time == 0.0 {
            //         println!("skill");
            //         self.skill_cool_time = self.attributes.skill_cool_time;

            //         self.prev_action_state = ActionState::Idle;
            //         self.action_state = ActionState::Skill;
            //         self.action_state_timer.reset();
            //     }
            // } else {
            //     println!("skill: passive");
            // }
        } else if input_flags.contains(GameInputBits::Attack) {
            let bullets_per_shot = 1; // 1발사 당 1 탄환

            if self.remaining_bullets >= bullets_per_shot {
                self.remaining_bullets -= bullets_per_shot;
  
        
                self.prev_action_state = ActionState::Idle;
                self.action_state = ActionState::Attack;
                self.action_state_timer.reset();
            } else {
                println!(
                    "not enough bullets! (Remaining: {})",
                    self.remaining_bullets
                );
            }
        } else if input_flags.contains(GameInputBits::Reload) {
            self.prev_action_state = ActionState::Idle;
            self.action_state = ActionState::Reload;
            self.action_state_timer.reset();
        } else if input_flags.contains(GameInputBits::Aiming) {
            self.prev_action_state = ActionState::Idle;
            self.action_state = ActionState::AimAt;
            self.action_state_timer.reset();
        }
    }

    /// `ActionState::Aiming`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_aiming(&mut self, input_flags: GameInputBits) {
        // 우선순위
        // ExSkill << Skill << Attack << Aiming
        //
        if input_flags.contains(GameInputBits::ExSkill) {
            // 모든 코스트를 소모
            println!("cost: {}", self.ex_skill_cost);
            if self.ex_skill_cost == 100.0 {
                println!("ex_skill");
                self.ex_skill_cost = 0.0;

                self.prev_action_state = ActionState::Aiming;
                self.action_state = ActionState::ExSkill;
                self.action_state_timer.reset();
            }
        } else if input_flags.contains(GameInputBits::Skill) {
            // if let SkillKind::Active(skill_cool_time) = self.skill_cool_time {
            //     println!("cool: {}", skill_cool_time);
            //     if skill_cool_time == 0.0 {
            //         println!("skill");
            //         self.skill_cool_time = self.attributes.skill_cool_time;

            //         self.prev_action_state = ActionState::Aiming;
            //         self.action_state = ActionState::Skill;
            //         self.action_state_timer.reset();
            //     }
            // } else {
            //     println!("skill: passive");
            // }
        } else if input_flags.contains(GameInputBits::Attack) {
            self.prev_action_state = ActionState::Aiming;
            self.action_state = ActionState::Attack;
            self.action_state_timer.reset();
        } else if !input_flags.contains(GameInputBits::Aiming) {
            self.prev_action_state = ActionState::Aiming;
            self.action_state = ActionState::AimOff;
            self.action_state_timer.reset();
        }
    }

    /// `ActionState::AimAt`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_aim_at(&mut self, input_flags: GameInputBits) {
        if !input_flags.contains(GameInputBits::Aiming) {
            self.prev_action_state = ActionState::AimAt;
            self.action_state = ActionState::AimOff;

            let length = self.attributes.normal_attack_start_duration;
            let s = self.action_state_timer.0 / length;

            let length = self.attributes.normal_attack_end_duration;
            self.action_state_timer.0 = s * length;
        }
    }

    /// `ActionState::AimOff`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_aim_off(&mut self, input_flags: GameInputBits) {
        if input_flags.contains(GameInputBits::Aiming) {
            self.prev_action_state = ActionState::AimOff;
            self.action_state = ActionState::AimAt;

            let length = self.attributes.normal_attack_end_duration;
            let s = self.action_state_timer.0 / length;

            let length = self.attributes.normal_attack_start_duration;
            self.action_state_timer.0 = s * length;
        }
    }

    /// `ActionState::Attack`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_attack(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `ActionState::Dead`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_dead(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `ActionState::Reload`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_reload(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `ActionState::Skill`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_skill(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `ActionState::ExSkill`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_ex_skill(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `ActionState::Callsign`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_callsign(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `ActionState::VictoryStart`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_victory_start(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `ActionState::VictoryEnd`일 때 `ActionState`를 갱신합니다.
    fn update_action_state_when_victory_end(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /*
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
    */

    /// `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        type Func = fn(&mut PlayerObject, &GameWorld, f32);
        const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
            PlayerObject::update_action_state_timer_when_idle,
            PlayerObject::update_action_state_timer_when_aiming,
            PlayerObject::update_action_state_timer_when_aim_at,
            PlayerObject::update_action_state_timer_when_aim_off,
            PlayerObject::update_action_state_timer_when_attack,
            PlayerObject::update_action_state_timer_when_dead,
            PlayerObject::update_action_state_timer_when_reload,
            PlayerObject::update_action_state_timer_when_skill,
            PlayerObject::update_action_state_timer_when_ex_skill,
            PlayerObject::update_action_state_timer_when_callsign,
            PlayerObject::update_action_state_timer_when_victory_start,
            PlayerObject::update_action_state_timer_when_victory_end,
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
            self.prev_action_state = ActionState::AimAt;
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
            self.prev_action_state = ActionState::AimOff;
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

            let shooter_id = self.account.uid;
            let delay = self.action_state_timer.0 - *time_point;
            world.push_event(GameWorldEvent::AddBullet { shooter_id, delay });
        }

        // 캐릭터의 `*_Normal_Attack_End` 애니메이션 길이보다 클 경우 `ActionState`를 변경합니다.
        let diff_t = self.action_state_timer.0 - duration;
        if diff_t >= 0.0 {
            self.action_state = self.prev_action_state;
            self.prev_action_state = ActionState::Attack;
            self.action_state_timer.0 = diff_t;
            self.fired_per_attack = 0;
        }
    }

    /// `ActionState::Dead`일 때 `ActionStateTimer`를 갱신합니다.  
    fn update_action_state_timer_when_dead(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        // 타이머를 갱신합니다.
        self.action_state_timer.0 += elapsed_time_sec;

        // RESPAWN_DELAY만큼 대기 후 플레이어를 리스폰합니다.
        let diff_t = self.action_state_timer.0 - RESPAWN_DELAY;
        if diff_t >= 0.0 {
            self.prev_action_state = ActionState::Idle;
            self.action_state = ActionState::Idle;
            self.action_state_timer.0 = diff_t;
            world.push_event(GameWorldEvent::RespawnPlayer {
                uid: self.account.uid,
            });
        }
    }

    /// `ActionState::Reload`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_reload(&mut self, _world: &GameWorld, elapsed_time_sec: f32) {
        // 타이머를 갱신합니다.
        self.action_state_timer.0 += elapsed_time_sec;

        // 캐릭터의 `*_Normal_Reload` 애니메이션 길이보다 클 경우 `ActionState`를 변경합니다.
        let duration = self.attributes.normal_reload_duration;
        let max_bullets = self.attributes.max_bullets;
        let diff_t = self.action_state_timer.0 - duration;
        if diff_t >= 0.0 {
            self.action_state = self.prev_action_state;
            self.prev_action_state = ActionState::Reload;
            self.action_state_timer.0 = diff_t;
            self.remaining_bullets = max_bullets;
        }
    }

    /// `ActionState::Skill`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_skill(&mut self, world: &GameWorld, elapsed_time_sec: f32) {
        // 임시로 3초간 총알을 발사하는 스킬을 구현
        let num_bullets_for_fire = 10;
        let duration = 3.0;

        let prev_bullet_fire_count =
            ((self.action_state_timer.0 / duration) * num_bullets_for_fire as f32).floor() as usize;

        // 타이머를 갱신합니다.
        self.action_state_timer.0 += elapsed_time_sec;

        let action_state_timer = self.action_state_timer.0.min(duration);
        let curr_bullet_fire_count =
            ((action_state_timer / duration) * num_bullets_for_fire as f32).floor() as usize;
        let num_bullets_to_fire = curr_bullet_fire_count - prev_bullet_fire_count;
        if num_bullets_to_fire > 0 {
            let last_fire_time =
                (duration / num_bullets_for_fire as f32) * curr_bullet_fire_count as f32;

            let delay = self.action_state_timer.0 - last_fire_time;
            let shooter_id = self.account.uid;

            for i in 0..num_bullets_to_fire {
                let delay = delay + (i as f32 * (duration / num_bullets_for_fire as f32));
                world.push_event(GameWorldEvent::AddBullet { shooter_id, delay });
            }
        }

        let diff_t = self.action_state_timer.0 - duration;
        if diff_t >= 0.0 {
            self.action_state = self.prev_action_state;
            self.prev_action_state = ActionState::Skill;
            self.action_state_timer.0 = diff_t;
        }
    }

    /// `ActionState::ExSkill`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_ex_skill(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        self.action_state_timer.0 += elapsed_time_sec;
        // 1초 후 이전 상태로 돌아감
        let diff_t = self.action_state_timer.0 - 1.0;
        if diff_t >= 0.0 {
            self.action_state = self.prev_action_state;
            self.prev_action_state = ActionState::ExSkill;
            self.action_state_timer.0 = diff_t;
        }
    }

    /// `ActionState::Callsign`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_callsign(
        &mut self,
        _world: &GameWorld,
        elapsed_time_sec: f32,
    ) {
        // 타이머를 갱신합니다.
        self.action_state_timer.0 += elapsed_time_sec;

        // 캐릭터의 `*_Normal_Callsign` 애니메이션 길이보다 클 경우 `ActionState`를 변경합니다.
        let duration = self.attributes.normal_callsign_duration;
        let max_bullets = self.attributes.max_bullets;
        let diff_t = self.action_state_timer.0 - duration;
        if diff_t >= 0.0 {
            self.action_state = ActionState::Idle;
            self.prev_action_state = ActionState::Idle;
            self.action_state_timer.0 = diff_t;
            self.remaining_bullets = max_bullets;
        }
    }

    /// `ActionState::VictoryStart`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_victory_start(
        &mut self,
        _world: &GameWorld,
        _elapsed_time_sec: f32,
    ) {
        /* empty */
    }

    /// `ActionState::Callsign`일 때 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_timer_when_victory_end(
        &mut self,
        _world: &GameWorld,
        _elapsed_time_sec: f32,
    ) {
        /* empty */
    }

    /// 플레이어 오브젝트의 `MovementState`를 갱신합니다.
    fn update_movement_state(&mut self, input_flags: GameInputBits) {
        type Func = fn(&mut PlayerObject, GameInputBits);
        const FUNC_TABLE: [[Func; NUM_MOVEMENT_STATES]; NUM_ACTION_STATES] = [
            // `ActionState::Idle`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_moving,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::Aiming`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_walking,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::AimAt`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_walking,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::AimOff`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_walking,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::Attack`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_walking,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::Dead`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_walking,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::Reload`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_walking,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::Skill`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_walking,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::ExSkill`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_walking,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::Callsign`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_moving,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::VictoryStart`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_moving,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
            // `ActionState::VictoryEnd`
            [
                PlayerObject::update_movement_state_when_idle,
                PlayerObject::update_movement_state_when_moving,
                PlayerObject::update_movement_state_when_move_to_end,
                PlayerObject::update_movement_state_when_in_place_jumping,
                PlayerObject::update_movement_state_when_in_place_landing,
                PlayerObject::update_movement_state_when_moving_jumping,
                PlayerObject::update_movement_state_when_moving_landing,
            ],
        ];

        let i = self.action_state as usize;
        let j = self.movement_state as usize;
        FUNC_TABLE[i][j](self, input_flags);
    }

    /// `MovementState::Idle`일 때 `MovementState`를 갱신합니다.
    fn update_movement_state_when_idle(&mut self, input_flags: GameInputBits) {
        if input_flags.bits() & 0x000F != 0x0000 {
            self.movement_state = MovementState::Moving;
            self.movement_state_timer.reset();
        } else if input_flags.contains(GameInputBits::Jump) {
            self.movement_state = MovementState::InPlaceJumping;
            self.movement_state_timer.reset();
        }
    }

    /// `MovementState::Moving`일 때 `MovementState`를 갱신합니다.
    fn update_movement_state_when_moving(&mut self, input_flags: GameInputBits) {
        if input_flags.bits() & 0x000F == 0x0000 {
            self.movement_state = MovementState::MoveToEnd;
            self.movement_state_timer.reset();
        } else if input_flags.contains(GameInputBits::Jump) {
            self.movement_state = MovementState::MovingJumping;
            self.movement_state_timer.reset();
        }
    }

    /// `ActionState::Aiming`이고, `MovementState::Moving`일 때 `MovementState`를 갱신합니다.
    fn update_movement_state_when_walking(&mut self, input_flags: GameInputBits) {
        if input_flags.bits() & 0x000F == 0x0000 {
            self.movement_state = MovementState::Idle;
            self.movement_state_timer.reset();
        } else if input_flags.contains(GameInputBits::Jump) {
            self.movement_state = MovementState::MovingJumping;
            self.movement_state_timer.reset();
        }
    }

    /// `MovementState::MoveToEnd`일 때 `MovementState`를 갱신합니다.
    fn update_movement_state_when_move_to_end(&mut self, input_flags: GameInputBits) {
        if input_flags.bits() & 0x000F != 0x0000 {
            self.movement_state = MovementState::Moving;
            self.movement_state_timer.reset();
        } else if input_flags.contains(GameInputBits::Jump) {
            self.movement_state = MovementState::InPlaceJumping;
            self.movement_state_timer.reset();
        }
    }

    /// `MovementState::InPlaceJumping`일 때 `MovementState`를 갱신합니다.
    fn update_movement_state_when_in_place_jumping(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `MovementState::InPlaceLanding`일 때 `MovementState`를 갱신합니다.
    fn update_movement_state_when_in_place_landing(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `MovementState::MovingJumping`일 때 `MovementState`를 갱신합니다.
    fn update_movement_state_when_moving_jumping(&mut self, _input_flags: GameInputBits) {
        /* empty */
    }

    /// `MovementState::MovingLanding`일 때 `MovementState`를 갱신합니다.
    fn update_movement_state_when_moving_landing(&mut self, _input_flags: GameInputBits) {
        /* empty */
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
        fn maintain_timer(_: &CharacterAttributes, _: &mut MovementStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut MovementStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut MovementStateTimer);
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
            // (`MovementState::Idle`, `ActionState::Dead`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::Reload`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::Skill`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::ExSkill`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::Callsign`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::VictoryStart`)
            [
                (MovementState::Idle, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),  // `MovementState::Moving`
                (MovementState::Idle, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::Idle, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Idle`, `ActionState::VictoryEnd`)
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
        timer_func(&self.attributes, &mut self.movement_state_timer);
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
        fn maintain_timer(_: &CharacterAttributes, _: &mut MovementStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut MovementStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut MovementStateTimer);
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
            // (`MovementState::Moving`, `ActionState::Dead`)
            [
                (MovementState::Idle, reset_timer),      // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),      // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::Reload`)
            [
                (MovementState::Idle, reset_timer),      // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),      // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::Skill`)
            [
                (MovementState::Idle, reset_timer),      // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),      // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::ExSkill`)
            [
                (MovementState::Idle, reset_timer),      // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),      // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::Callsign`)
            [
                (MovementState::MoveToEnd, reset_timer), // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::MoveToEnd, reset_timer), // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::VictoryStart`)
            [
                (MovementState::MoveToEnd, reset_timer), // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::MoveToEnd, reset_timer), // `MovementState::MoveToEnd`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MovingJumping, reset_timer), // `MovementState::MovingJumping`
                (MovementState::Moving, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::Moving`, `ActionState::VictoryEnd`)
            [
                (MovementState::MoveToEnd, reset_timer), // `MovementState::Idle`
                (MovementState::Moving, maintain_timer), // `MovementState::Moving`
                (MovementState::MoveToEnd, reset_timer), // `MovementState::MoveToEnd`
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
        timer_func(&self.attributes, &mut self.movement_state_timer);
    }

    /// `MovementState::MoveToEnd`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_move_to_end(&mut self, new: MovementState) {
        // 변경 가능한 다음 상태
        // - MovementState::Moving
        // - MovementState::InPlaceJumping
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_: &CharacterAttributes, _: &mut MovementStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut MovementStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut MovementStateTimer);
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
            // (`MovementState::MoveToEnd`, `ActionState::Dead`)
            [
                (MovementState::Idle, reset_timer),   // `MovementState::Idle`
                (MovementState::Moving, reset_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),   // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::InPlaceLanding`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::Reload`)
            [
                (MovementState::Idle, reset_timer),   // `MovementState::Idle`
                (MovementState::Moving, reset_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),   // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::InPlaceLanding`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::Skill`)
            [
                (MovementState::Idle, reset_timer),   // `MovementState::Idle`
                (MovementState::Moving, reset_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),   // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::InPlaceLanding`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::ExSkill`)
            [
                (MovementState::Idle, reset_timer),   // `MovementState::Idle`
                (MovementState::Moving, reset_timer), // `MovementState::Moving`
                (MovementState::Idle, reset_timer),   // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::InPlaceLanding`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingJumping`
                (MovementState::Idle, reset_timer),   // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::Callsign`)
            [
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),       // `MovementState::Moving`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::VictoryStart`)
            [
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),       // `MovementState::Moving`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MovingLanding`
            ],
            // (`MovementState::MoveToEnd`, `ActionState::VictoryEnd`)
            [
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::Idle`
                (MovementState::Moving, reset_timer),       // `MovementState::Moving`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MoveToEnd`
                (MovementState::InPlaceJumping, reset_timer), // `MovementState::InPlaceJumping`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::InPlaceLanding`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MovingJumping`
                (MovementState::MoveToEnd, maintain_timer), // `MovementState::MovingLanding`
            ],
        ];

        let i = self.action_state as usize;
        let j = new as usize;
        let (next_state, timer_func) = TABLE[i][j];

        self.movement_state = next_state;
        timer_func(&self.attributes, &mut self.movement_state_timer);
    }

    /// `MovementState::InPlaceJumping`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_in_place_jumping(&mut self, new: MovementState) {
        // 변경 가능한 다음 상태
        // - MovementState::InPlaceLanding
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_: &CharacterAttributes, _: &mut MovementStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut MovementStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut MovementStateTimer);
        const TABLE: [(MovementState, Func); NUM_MOVEMENT_STATES] = [
            (MovementState::InPlaceJumping, maintain_timer),
            (MovementState::InPlaceJumping, maintain_timer),
            (MovementState::InPlaceJumping, maintain_timer),
            (MovementState::InPlaceJumping, maintain_timer),
            (MovementState::InPlaceLanding, reset_timer),
            (MovementState::InPlaceJumping, maintain_timer),
            (MovementState::InPlaceJumping, maintain_timer),
        ];

        let i = new as usize;
        let (next_state, timer_func) = TABLE[i];

        self.movement_state = next_state;
        timer_func(&self.attributes, &mut self.movement_state_timer);
    }

    /// `MovementState::InPlaceLanding`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_in_place_landing(&mut self, new: MovementState) {
        // 변경 가능한 다음 상태
        // - MovementState::Idle
        // - MovementState::Moving
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_: &CharacterAttributes, _: &mut MovementStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut MovementStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut MovementStateTimer);
        const TABLE: [(MovementState, Func); NUM_MOVEMENT_STATES] = [
            (MovementState::Idle, reset_timer),
            (MovementState::Moving, reset_timer),
            (MovementState::InPlaceLanding, maintain_timer),
            (MovementState::InPlaceLanding, maintain_timer),
            (MovementState::InPlaceLanding, maintain_timer),
            (MovementState::InPlaceLanding, maintain_timer),
            (MovementState::InPlaceLanding, maintain_timer),
        ];

        let i = new as usize;
        let (next_state, timer_func) = TABLE[i];

        self.movement_state = next_state;
        timer_func(&self.attributes, &mut self.movement_state_timer);
    }

    /// `MovementState::MovingJumping`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_moving_jumping(&mut self, new: MovementState) {
        // 변경 가능한 다음 상태
        // - MovementState::MovingLanding
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_: &CharacterAttributes, _: &mut MovementStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut MovementStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut MovementStateTimer);
        const TABLE: [(MovementState, Func); NUM_MOVEMENT_STATES] = [
            (MovementState::MovingJumping, maintain_timer),
            (MovementState::MovingJumping, maintain_timer),
            (MovementState::MovingJumping, maintain_timer),
            (MovementState::MovingJumping, maintain_timer),
            (MovementState::MovingJumping, maintain_timer),
            (MovementState::MovingJumping, maintain_timer),
            (MovementState::MovingLanding, reset_timer),
        ];

        let i = new as usize;
        let (next_state, timer_func) = TABLE[i];

        self.movement_state = next_state;
        timer_func(&self.attributes, &mut self.movement_state_timer);
    }

    /// `MovementState::MovingLanding`일 때 `MovementState` 변경을 시도합니다.
    /// 해당 `MovementState`로 갱신할 수 없는 경우 무시됩니다.
    fn change_movement_state_when_moving_landing(&mut self, new: MovementState) {
        // 변경 가능한 다음 상태
        // - MovementState::Idle
        // - MovementState::Moving
        //

        /// 타이머를 유지합니다.
        fn maintain_timer(_: &CharacterAttributes, _: &mut MovementStateTimer) {
            /* empty */
        }

        /// 타이머를 초기화합니다.
        fn reset_timer(_: &CharacterAttributes, timer: &mut MovementStateTimer) {
            timer.reset();
        }

        type Func = fn(&CharacterAttributes, &mut MovementStateTimer);
        const TABLE: [(MovementState, Func); NUM_MOVEMENT_STATES] = [
            (MovementState::Idle, reset_timer),
            (MovementState::Moving, reset_timer),
            (MovementState::InPlaceLanding, maintain_timer),
            (MovementState::InPlaceLanding, maintain_timer),
            (MovementState::InPlaceLanding, maintain_timer),
            (MovementState::InPlaceLanding, maintain_timer),
            (MovementState::InPlaceLanding, maintain_timer),
        ];

        let i = new as usize;
        let (next_state, timer_func) = TABLE[i];

        self.movement_state = next_state;
        timer_func(&self.attributes, &mut self.movement_state_timer);
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
            // `ActionState::Dead`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_walking,
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::Reload`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_walking,
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::Skill`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_walking,
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::ExSkill`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_walking,
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::Callsign`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_moving,
                PlayerObject::update_movement_state_timer_when_move_to_end,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::VictoryStart`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_moving,
                PlayerObject::update_movement_state_timer_when_move_to_end,
                PlayerObject::update_movement_state_timer_when_in_place_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
                PlayerObject::update_movement_state_timer_when_moving_jumping,
                PlayerObject::update_movement_state_timer_when_landing,
            ],
            // `ActionState::VictoryEnd`
            [
                PlayerObject::update_movement_state_timer_when_idle,
                PlayerObject::update_movement_state_timer_when_moving,
                PlayerObject::update_movement_state_timer_when_move_to_end,
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

    /// 입력 지속시간을 갱신합니다.
    fn update_input_timer(&mut self, elapsed_time_sec: f32) {
        type Func = fn(&mut PlayerObject, f32);
        const FUNC_TABLE: [Func; NUM_MOVEMENT_STATES] = [
            PlayerObject::decrease_input_timer,
            PlayerObject::increase_input_timer,
            PlayerObject::decrease_input_timer,
            PlayerObject::maintain_input_timer,
            PlayerObject::maintain_input_timer,
            PlayerObject::maintain_input_timer,
            PlayerObject::maintain_input_timer,
        ];

        FUNC_TABLE[self.movement_state as usize](self, elapsed_time_sec)
    }

    /// 입력 지속 시간을 유지합니다.
    fn maintain_input_timer(&mut self, _elapsed_time_sec: f32) {
        /* empty */
    }

    /// 입력 지속 시간을 증가시킵니다.
    fn increase_input_timer(&mut self, elapsed_time_sec: f32) {
        self.input_timer = (self.input_timer + elapsed_time_sec).min(MAX_INPUT_DURATION);
    }

    /// 입력 지속 시간을 감소시킵니다.
    fn decrease_input_timer(&mut self, elapsed_time_sec: f32) {
        self.input_timer = (self.input_timer - elapsed_time_sec).max(0.0);
    }
}
