//! 캐릭터 속성과 관련된 코드를 관리합니다.
//!

use ahash::{HashMap, RandomState};
use mod_physics::object3d::Capsule;
use serde::{Deserialize, Serialize};

use crate::components::{Float3, Float4x4, LatLon, ViewState, ViewStateTimer};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeaponAttributes {
    pub bip001: Float4x4,
    pub bip001_pelvis: Float4x4,
    pub bip001_spine: Float4x4,
    pub bip001_spine1: Float4x4,
    pub bip001_clavicle: Float4x4,
    pub bip001_upperarm: Float4x4,
    pub bip001_forearm: Float4x4,
    pub bip001_hand: Float4x4,
    pub hand_to_weapon_offset: Float4x4,
    pub bip001_fire: Float4x4,
}

impl WeaponAttributes {
    const BIP001: &'static str = "Bip001";
    const BIP001_PELVIS: &'static str = "Bip001_Pelvis";
    const BIP001_SPINE: &'static str = "Bip001_Spine";
    const BIP001_SPINE1: &'static str = "Bip001_Spine1";
    const BIP001_CLAVICLE: &'static str = "Bip001_Clavicle";
    const BIP001_UPPER_ARM: &'static str = "Bip001_UpperArm";
    const BIP001_FOREARM: &'static str = "Bip001_Forearm";
    const BIP001_HAND: &'static str = "Bip001_Hand";

    /// 총구의 위치와 총알 발사 방향을 반환합니다.
    pub fn get_position_and_direction(
        &self,
        view_state: ViewState,
        view_state_timer: ViewStateTimer,
        character_attributes: &CharacterAttributes,
        translation: glam::Vec3A,
        rotation: glam::Quat,
        latlon: LatLon,
    ) -> (glam::Vec3A, glam::Quat, glam::Quat) {
        // 카메라가 바라보는 방향을 계산합니다.
        let camera_default_pos: glam::Vec3A = character_attributes.camera_def_rel_pos.into();
        let camera_zoom_pos: glam::Vec3A = character_attributes.camera_zoom_rel_pos.into();
        let camera_rel_pos = match view_state {
            ViewState::Idle => {
                camera_default_pos
            },
            ViewState::ZoomIn => {
                let duration = character_attributes.normal_attack_start_duration;
                let s = view_state_timer.0 as f32 / duration as f32;
                camera_default_pos.lerp(camera_zoom_pos, s)
            },
            ViewState::ZoomOut => {
                let duration = character_attributes.normal_attack_end_duration;
                let s = view_state_timer.0 as f32 / duration as f32;
                camera_zoom_pos.lerp(camera_default_pos, s)
            },
            ViewState::Aiming => {
                camera_zoom_pos
            },
        };

        let distance = camera_rel_pos * glam::Vec3A::NEG_Z;
        let mut transform = glam::Mat4::from_translation(distance.into());
        let rotate = glam::Mat4::from_rotation_y(latlon.lon);
        transform = rotate * transform;

        let forward = glam::Vec3A::from_vec4(transform.z_axis);
        let forward = forward.normalize_or(glam::Vec3A::Z);
        let axis = glam::Vec3A::Y.cross(forward);
        let rotate = glam::Mat4::from_axis_angle(axis.into(), latlon.lat);
        transform = rotate * transform;

        let offset = camera_rel_pos.with_z(0.0);
        let offset = glam::Mat4::from_translation(offset.into());
        transform = transform * offset;

        let parent = glam::Mat4::from_translation(translation.into());
        transform = parent * transform;

        // 총알의 끝 지점을 계산합니다.
        let base = glam::Vec3A::from_vec4(transform.w_axis);
        let direction = glam::Vec3A::from_vec4(transform.z_axis);
        let distination = base + direction * character_attributes.attack_range as f32;

        // 캐릭터의 회전 방향을 보정합니다.
        let z = (distination - translation).normalize_or(glam::Vec3A::Z);
        let x = glam::Vec3A::Y.cross(z);
        let y = z.cross(x);
        let new_rotation = glam::Quat::from_mat3a(&glam::mat3a(x, y, z));

        // 노드 계층 구조를 생성합니다.
        let hierarchy = HashMap::from_iter([
            (Self::BIP001, None),
            (Self::BIP001_PELVIS, Some(Self::BIP001)),
            (Self::BIP001_SPINE, Some(Self::BIP001_PELVIS)),
            (Self::BIP001_SPINE1, Some(Self::BIP001_SPINE)),
            (Self::BIP001_CLAVICLE, Some(Self::BIP001_SPINE1)),
            (Self::BIP001_UPPER_ARM, Some(Self::BIP001_CLAVICLE)),
            (Self::BIP001_FOREARM, Some(Self::BIP001_UPPER_ARM)),
            (Self::BIP001_HAND, Some(Self::BIP001_FOREARM)),
        ]);

        // 로컬 변환 행렬을 생성합니다.
        let parent = glam::Mat4::from_rotation_translation(rotation.into(), translation.into());
        let root_bone = glam::Mat4::from_quat(glam::quat(-0.7071068, 0.0, 0.0, 0.7071068));
        let mut bip001: glam::Mat4 = self.bip001.into();
        bip001 = parent * root_bone * bip001;

        let angle = 3.0 * latlon.lat / 7.0;
        let axis: glam::Vec3 = character_attributes.attack_spine_axis.into();
        let mut spine: glam::Mat4 = self.bip001_spine.into();
        spine *= glam::Mat4::from_axis_angle(axis, angle);

        let axis: glam::Vec3 = character_attributes.attack_spine1_axis.into();
        let mut spine1: glam::Mat4 = self.bip001_spine1.into();
        spine1 *= glam::Mat4::from_axis_angle(axis, angle);

        let local_trans: HashMap<_, glam::Mat4> = HashMap::from_iter([
            (Self::BIP001, bip001),
            (Self::BIP001_PELVIS, self.bip001_pelvis.into()),
            (Self::BIP001_SPINE, spine),
            (Self::BIP001_SPINE1, spine1),
            (Self::BIP001_CLAVICLE, self.bip001_clavicle.into()),
            (Self::BIP001_UPPER_ARM, self.bip001_upperarm.into()),
            (Self::BIP001_FOREARM, self.bip001_forearm.into()),
            (Self::BIP001_HAND, self.bip001_hand.into()),
        ]);

        // 월드 변환 행렬을 계산합니다.
        let mut world_trans = HashMap::with_capacity_and_hasher(8, RandomState::new());
        let current = Self::BIP001_HAND;
        let parent = hierarchy.get(current).cloned().flatten();
        Self::compute_world_transform(current, parent, &hierarchy, &local_trans, &mut world_trans);

        // 무기의 위치를 계산합니다.
        let hand_world_transform = world_trans
            .get(Self::BIP001_HAND)
            .cloned()
            .unwrap_or_default();
        let hand_to_weapon_trans: glam::Mat4 = self.hand_to_weapon_offset.into();
        let bip001_fire: glam::Mat4 = self.bip001_fire.into();
        let weapon_trans = hand_world_transform * hand_to_weapon_trans * bip001_fire;

        let translation = glam::Vec3A::from_vec4(weapon_trans.w_axis);
        // let mut rotation = glam::Quat::from_mat4(&weapon_trans).normalize();
        let z = (distination - translation).normalize_or(glam::Vec3A::Z);
        // let z = rotation.mul_vec3a(glam::Vec3A::Z);
        let x = glam::Vec3A::Y.cross(z);
        let y = z.cross(x);
        let rotation = glam::Quat::from_mat3a(&glam::mat3a(x, y, z));

        (translation, rotation, new_rotation)
    }

    /// 월드 변환 행렬을 계산합니다.
    fn compute_world_transform(
        current: &'static str,
        parent: Option<&'static str>,
        hierarchy: &HashMap<&'static str, Option<&'static str>>,
        local_transforms: &HashMap<&'static str, glam::Mat4>,
        world_transforms: &mut HashMap<&'static str, glam::Mat4>,
    ) -> glam::Mat4 {
        if let Some(parent) = parent {
            match world_transforms.get(parent).cloned() {
                Some(parent_trans) => {
                    let local_trans = local_transforms.get(current).cloned().unwrap_or_default();
                    let world_trans = parent_trans * local_trans;
                    world_transforms.insert(current, world_trans);
                    world_trans
                }
                None => {
                    let grand_parent = hierarchy.get(parent).cloned().flatten();
                    let parent_trans = Self::compute_world_transform(
                        parent,
                        grand_parent,
                        hierarchy,
                        local_transforms,
                        world_transforms,
                    );
                    let local_trans = local_transforms.get(&current).cloned().unwrap_or_default();
                    let world_trans = parent_trans * local_trans;
                    world_transforms.insert(current, world_trans);
                    world_trans
                }
            }
        } else {
            let local_trans = local_transforms.get(current).cloned().unwrap_or_default();
            world_transforms.insert(current, local_trans);
            local_trans
        }
    }
}

/// 캐릭터 속성 데이터를 저장합니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterAttributes {
    /// 캐릭터 이동 속도
    pub speed: f32,

    /// 왼쪽 무기 속성 데이터
    pub left_weapon: Option<WeaponAttributes>,
    /// 오른쪽 무기 속성 데이터
    pub right_weapon: Option<WeaponAttributes>,

    /// 일반 공격 상태 일 때 `Head` 뼈 노드의 축
    pub attack_head_axis: Float3,
    /// 일반 공격 상태 일 때 `Spine` 뼈 노드의 축
    pub attack_spine_axis: Float3,
    /// 일반 공격 상태 일 때 `Spine1` 뼈 노드의 축
    pub attack_spine1_axis: Float3,

    /// 스킬 시전 상태 일 때 `Head` 뼈 노드의 축
    pub skill_head_axis: Float3,
    /// 스킬 시전 상태 일 때 `Spine` 뼈 노드의 축
    pub skill_spine_axis: Float3,
    /// 스킬 시전 상태 일 때 `Spine1` 뼈 노드의 축
    pub skill_spine1_axis: Float3,

    /// 기본 카메라 상대 위치
    pub camera_def_rel_pos: Float3,
    /// 기본 카메라 Fov-y
    pub camera_def_fov_y: f32,
    /// 줌인 상태 카메라 상대 위치
    pub camera_zoom_rel_pos: Float3,
    /// 줌인 상태 카메라 Fov-y
    pub camera_zoom_fov_y: f32,

    /// `ActionState::Idle` 애니메이션 시간 (단위: ms)
    pub normal_idle_duration: u16,
    /// 걷기 애니메이션 시간 (단위: ms)
    pub cafe_walk_duration: u16,
    /// `MovementState::Moving` 애니메이션 시간 (단위: ms)
    pub move_ing_duration: u16,
    /// `MovementState::MoveToEnd` 애니메이션 시간 (단위: ms)
    pub move_end_normal_duration: u16,
    /// `ActionState::Aiming` 애니메이션 시간 (단위: ms)
    pub normal_attack_start_duration: u16,
    /// `ActionState::AimOff` 애니메이션 시간 (단위: ms)
    pub normal_attack_end_duration: u16,
    /// `ActionState::Attack` 애니메이션 시간 (단위: ms)
    pub normal_attack_ing_duration: u16,
    /// `ActionState::Dead` 애니메이션 시간 (단위: ms)
    pub vital_death_duration: u16,
    /// `ActionState::Reload` 애니메이션 시간 (단위: ms)
    pub normal_reload_duration: u16,
    /// `ActionState::Skill` 애니메이션 시간 (단위: ms)
    pub skill_duration: u16,
    /// `AcstionState::Callsign` 애니메이션 시간 (단위: ms)
    pub normal_callsign_duration: u16,
    /// `ActionState::VictoryStart` 애니메이션 시간 (단위: ms)
    pub victory_start_duration: u16,
    /// `ActionState::VictoryStart` 애니메이션 시간 (단위: ms)
    pub victory_end_duration: u16,

    /// 일반 공격 총알 발사 시간 (단위: ms)
    pub normal_attack_timing: Vec<u16>,
    /// 일반 공격 총알 발사 수
    pub normal_attack_count: u16,
    /// 총알의 최대 개수
    pub max_bullets: u16,
    /// 캐릭터의 최대 체력
    pub max_health_point: u16,
    /// 캐릭터의 공격력
    pub attack_power: u16,
    /// 캐릭터의 방어력
    pub defense_power: u16,
    /// 캐릭터의 명중 수치
    pub accuracy_stat: u16,
    /// 캐릭터의 회피 수치
    pub evasion_stat: u16,
    /// 캐릭터의 치명 수치
    pub critical_rate: u16,
    /// 캐릭터의 치명 데미지
    pub critical_damage: u16,
    /// 캐릭터의 최대 스킬 코스트
    pub max_skill_cost: u16,
    /// 캐릭터의 사용 스킬 코스트
    pub skill_cost: u16,
    /// 캐릭터의 공격 사거리
    pub attack_range: u16,
    /// 캐릭터 총알의 반지름
    pub bullet_radius: f32,

    /// 캐릭터 충돌체
    pub collider: Capsule,
}
