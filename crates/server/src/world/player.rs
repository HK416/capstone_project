use mod_network::components::{
    ActionState, ActionStateTimer, CharacterAttributes, CharacterKind, Epoch, HealthPoint, LatLon,
    MovementState, MovementStateTimer, ObjectId, ViewState, ViewStateTimer,
};

/// 서버에서 관리하는 플레이어 데이터
#[derive(Debug, Clone)]
pub struct ServerPlayer {
    /// 플레이어의 시대
    pub epoch: Epoch,
    /// 플레이어 오브젝트 식별자
    pub object_id: ObjectId,
    /// 플레이어 캐릭터 종류
    pub character_kind: CharacterKind,
    /// 플레이어 캐릭터 체력
    pub health_point: HealthPoint,
    /// 플레이어 캐릭터의 월드 공간 위치
    pub translation: glam::Vec3A,
    /// 플레이어 캐릭터의 월드 공간 방향 (캐릭터가 움직이는 방향과 다를 수 있음)
    pub rotation: glam::Quat,
    /// 플레이어 캐릭터의 월드 공간 속도
    pub velocity: glam::Vec3A,
    /// 플레이어 움직임 방향
    pub direction: glam::Vec3A,
    /// 플레이어 캐릭터 행동 상태
    pub action_state: ActionState,
    /// 플레이어 캐릭터 이전 행동 상태
    pub prev_action_state: ActionState,
    /// 플레이어 캐릭터 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 플레이어 캐릭터 움직임 상태
    pub movement_state: MovementState,
    /// 플레이어 캐릭터 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 플레이어 카메라 상태
    pub view_state: ViewState,
    /// 플레이어 카메라 상태 타이머
    pub view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터 중심으로 바라보는 방향
    pub view_rotation: LatLon,
    /// 총알 발사 횟수
    pub shot_count: u32,
}

/// 주어진 시간 만큼 플레이어 캐릭터의 `ActionState`와 `ActionStateTimer`를 갱신합니다.
pub fn update_character_action_state_timer(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    type Func = fn(&CharacterAttributes, &mut ServerPlayer, f32);
    const FUNC_TABLE: [Func; 5] = [
        update_action_state_timer_when_idle,
        update_action_state_timer_when_aiming,
        update_action_state_timer_when_aim_at,
        update_action_state_timer_when_aim_off,
        update_action_state_timer_when_attack,
    ];

    let i = player.action_state as usize;
    FUNC_TABLE[i](attributes, player, elapsed_time_sec);
}

/// `ActionState::Idle`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_idle(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.action_state_timer.0 =
        (player.action_state_timer.0 + elapsed_time_sec) % attributes.normal_idle_duration;
}

/// `ActionState::Aiming`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aiming(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.action_state_timer.0 =
        (player.action_state_timer.0 + elapsed_time_sec) % attributes.normal_idle_duration;
}

/// `ActionState::AimAt`일 때 `ActionState`와 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aim_at(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.action_state_timer.0 += elapsed_time_sec;

    // `*_Normal_Attack_Start` 애니메이션 길이보다 클 경우 `ActionState`를 갱신합니다.
    let diff_t = player.action_state_timer.0 - attributes.normal_attack_start_duration;
    if diff_t >= 0.0 {
        player.action_state = ActionState::Aiming;
        player.action_state_timer.0 = diff_t % attributes.normal_idle_duration;
    }
}

/// `ActionState::AimOff`일 때 `ActionState`와 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aim_off(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.action_state_timer.0 += elapsed_time_sec;

    // `*_Normal_Attack_End` 애니메이션 길이보다 클 경우 `ActionState`를 갱신합니다.
    let diff_t = player.action_state_timer.0 - attributes.normal_attack_end_duration;
    if diff_t >= 0.0 {
        player.action_state = ActionState::Idle;
        player.action_state_timer.0 = diff_t % attributes.normal_idle_duration;
    }
}

/// `ActionState::Attack`일 때 `ActionState`와 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_attack(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.action_state_timer.0 += elapsed_time_sec;

    // `*_Normal_Attack_Ing` 애니메이션 길이보다 클 경우 `ActionState`를 갱신합니다.
    let diff_t = player.action_state_timer.0 - attributes.normal_attack_ing_duration;
    if diff_t >= 0.0 {
        player.action_state = ActionState::Idle;
        player.action_state_timer.0 = diff_t % attributes.normal_idle_duration;
        player.shot_count = 0;
    }
}

/// 캐릭터 모델의 `MovementState`와 `MovementStateTimer`를 갱신합니다.
///
/// # Note
/// - 이 함수를 호출하기 전에 ActionState를 먼저 갱신해야합니다.
///
pub fn update_character_movement_state_timer(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    type Func = fn(&CharacterAttributes, &mut ServerPlayer, f32);
    const FUNC_TABLE: [[Func; 3]; 5] = [
        // `ActionState::Idle`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_moving,
            update_movement_state_timer_when_move_to_end,
        ],
        // `ActionState::Aiming`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
        ],
        // `ActionState::AimAt`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
        ],
        // `ActionState::AimOff`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
        ],
        // `ActionState::Attack`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
        ],
    ];

    let i = player.action_state as usize;
    let j = player.movement_state as usize;
    FUNC_TABLE[i][j](attributes, player, elapsed_time_sec);
}

/// `*_Normal_Idle` 애니메이션 데이터로 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_idle(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.movement_state_timer.0 =
        (player.movement_state_timer.0 + elapsed_time_sec) % attributes.normal_idle_duration;
}

/// `*_Move_Ing` 애니메이션 데이터로 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_moving(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.movement_state_timer.0 =
        (player.movement_state_timer.0 + elapsed_time_sec) % attributes.move_ing_duration;
}

/// `*_Move_End_Normal` 애니메이션 데이터로 `MovementState`와 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_move_to_end(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.movement_state_timer.0 = player.movement_state_timer.0 + elapsed_time_sec;

    // `*_Move_End_Normal` 애니메이션 길이보다 클 경우 `MovementState`를 갱신합니다.
    let diff_t = player.movement_state_timer.0 - attributes.move_end_normal_duration;
    if diff_t >= 0.0 {
        player.movement_state = MovementState::Idle;
        player.movement_state_timer.0 = diff_t;
    }
}

/// `*_Cafe_Walk` 애니메이션 데이터로 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_walking(
    attributes: &CharacterAttributes,
    player: &mut ServerPlayer,
    elapsed_time_sec: f32,
) {
    // 타이머를 갱신합니다.
    player.movement_state_timer.0 =
        (player.movement_state_timer.0 + elapsed_time_sec) % attributes.walk_duration;
}
