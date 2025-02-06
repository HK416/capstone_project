//
// # 애니메이션 테이블
// 애니메이션은 `ActionState`와 `MovementState`로 결정된다.
//
// +----------------------+-----------------------+-------------------------+--------------------------+
// |                      | MovementState::Idle   | MovementState::Moving   | MovementState::MoveToEnd |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::Idle    | Idle                  | Moving                  | MoveToEnd                |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::Aiming  | Aim                   | Aim_Move                | Aim                      |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::AimAt   | Idle_To_Aim           | Move_To_Aim_Move        | Idle_To_Aim              |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::AimOff  | Aim_To_Idle           | Aim_Move_To_Move        | Aim_To_Idle              |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::Attack  | Attack_Ing            | Attack_Move             | Attack_Ing               |
// +----------------------+-----------------------+-------------------------+--------------------------+
//
// # 애니메이션 목록
// - Idle
// - Moving
// - MoveToEnd
// - Aim
// - AimMove
// - IdleToAim
// - MoveToAimMove
// - AimToIdle
// - AimMoveToMove
// - Attacking
// - AttackMove
//
use ahash::{HashMap, HashSet};
use hecs::Entity;

/// 모든 캐릭터 모델의 최상위 뼈 노드 이름입니다.
pub const MODEL_BONE_ROOT: &'static str = "Bip001";
/// 모든 캐릭터 모델의 머리 뼈 노드 이름입니다.
pub const MODEL_BONE_HEAD: &'static str = "Bip001_Head";
/// 모든 캐릭터 모델의 아래 척추 뼈 노드 이름입니다.
pub const MODEL_BONE_SPINE: &'static str = "Bip001_Spine";
/// 모든 캐릭터 모델의 윗 척추 뼈 노드 이름입니다.
pub const MODEL_BONE_SPINE_1: &'static str = "Bip001_Spine1";
/// 모든 캐릭터 모델의 왼쪽 허벅지 안쪽 뼈 노드 이름입니다.
pub const MODEL_BONE_L_THIGH: &'static str = "Bip001_L_Thigh";
/// 모든 캐릭터 모델의 오른쪽 허벅지 안쪽 뼈 노드 이름입니다.
pub const MODEL_BONE_R_THIGH: &'static str = "Bip001_R_Thigh";
/// 모든 캐릭터의 오른쪽 손 뼈 노드 이름입니다.
pub const MODEL_BONE_R_HAND: &'static str = "Bip001_R_Hand";
/// 모든 캐릭터 모델의 무기 뼈 노드 이름입니다.
pub const MODEL_BONE_WEAPON: &'static str = "Bip001_Weapon";

/// 모든 캐릭터 모델의 Idle 애니메이션 접미사입니다.
pub const IDLE_ANIMATION_SUFFIX: &'static str = "_Normal_Idle";
/// 모든 캐릭터 모델의 Moving 애니메이션 접미사입니다.
pub const MOVING_ANIMATION_SUFFIX: &'static str = "_Move_Ing";
/// 모든 캐릭터 모델의 MoveToEnd 애니메이션 접미사입니다.
pub const MOVE_TO_END_ANIMATION_SUFFIX: &'static str = "_Move_End_Normal";
/// 모든 캐릭터 모델의 CafeWalk 애니메이션 접미사입니다.
pub const CAFE_WALK_ANIMATION_SUFFIX: &'static str = "_Cafe_Walk";
/// 모든 캐릭터 모델의 AttackStart 애니메이션 접미사입니다.
pub const ATTACK_START_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_Start";
/// 모든 캐릭터 모델의 Attacking 애니메이션 접미사입니다.
pub const ATTACK_ING_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_Ing";
/// 모든 캐릭터 모델의 AttackEnd 애니메이션 접미사입니다.
pub const ATTACK_END_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_End";
// /// 모든 캐릭터 모델의 Reload 애니메이션 접미사입니다.
// pub const RELOAD_ANIMATION_SUFFIX: &'static str = "_Normal_Reload";
// /// 모든 캐릭터 모델의 Ex스킬 애니메이션 접미사입니다.
// pub const EXS_ANIMATION_SUFFIX: &'static str = "_Exs";

/// ## Skinning Animation
/// 스키닝 애니메이션에 사용되는 스키닝 메쉬 엔터티와 최상위 뼈 노드 엔터티의 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinningAnimation {
    /// NOTE: `BoneCollection`의 `root`와 다름!
    pub root: Entity,
    pub head: Entity,
    pub muzzle: Entity,
    pub weapon: Entity,
    pub lower_spine: Entity,
    pub uppper_spine: Entity,
    pub right_hand: Entity,
    pub meshes: HashMap<String, Entity>,
    pub animation_mixing_bones: HashSet<Entity>,
}

impl Default for SkinningAnimation {
    fn default() -> Self {
        Self {
            root: Entity::DANGLING,
            head: Entity::DANGLING,
            muzzle: Entity::DANGLING,
            weapon: Entity::DANGLING,
            lower_spine: Entity::DANGLING,
            uppper_spine: Entity::DANGLING,
            right_hand: Entity::DANGLING,
            meshes: HashMap::default(),
            animation_mixing_bones: HashSet::default(),
        }
    }
}

/// ## Bone Collection
/// 스키닝된 메쉬를 구성하는 뼈의 엔터티 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoneCollection {
    /// NOTE: 스키닝된 메쉬의 최상위 뼈를 나타냅니다.
    pub root: Entity,
    pub bones: Vec<Entity>,
}
