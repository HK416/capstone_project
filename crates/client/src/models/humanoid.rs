#![allow(unused)]

use ahash::HashMap;
use hecs::Entity;

/// ## General Humanoid Bone Kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoneKind {
    BoneRoot, // bone_root
    Bip001,  // Bip001
    Bip001Pelvis, // Bip001_Pelvis
    Bip001Weapon,  // Bip001_Weapon
    Bip001LeftThigh, // Bip001_L_Thigh
    Bip001RightThigh, // Bip001_R_Thigh
    Bip001Spine,  // Bip001_Spine
    Bip001LeftCalf, // Bip001_L_Calf
    Bip001LeftFoot, // Bip001_L_Foot
    Bip001LeftToe0, // Bip001_L_Toe0
    Bip001RightCalf, // Bip001_R_Calf
    Bip001RightFoot, // Bip001_R_Foot
    Bip001RightToe0, // Bip001_R_Toe0
    Bip001Spine1, // Bip001_Spine1
    Bip001LeftClavicle, // Bip001_L_Clavicle
    Bip001Neck, // Bip001_Neck
    Bip001RightClavicle, // Bip001_R_Clavicle
    Bip001LeftUpperArm, // Bip001_L_UpperArm
    Bip001BoneLeftDeltoid, // Bip001_B_L_Deltoid
    Bip001LeftForearm, // Bip001_L_Forearm
    Bip001LeftHand, // Bip001_L_Hand
    Bip001LeftFinger0, // Bip001_L_Finger0
    Bip001LeftFinger1, // Bip001_L_Finger1
    Bip001LeftFinger2, // Bip001_L_Finger2
    Bip001LeftFinger01, // Bip001_L_Finger01
    Bip001LeftFinger11, // Bip001_L_Finger11
    Bip001LeftFinger21, // Bip001_L_Finger21
    Bip001Head, // Bip001_Head
    Bip001RightUpperArm, // Bip001_R_UpperArm
    Bip001BoneRightDeltoid, // Bip001_B_R_Deltoid
    Bip001RightForearm, // Bip001_R_Forearm
    Bip001RightHand, // Bip001_R_Hand
    Bip001RightFinger0, // Bip001_R_Finger0
    Bip001RightFinger1, // Bip001_R_Finger1
    Bip001RightFinger2, // Bip001_R_Finger2
    Bip001RightFinger01, // Bip001_R_Finger01
    Bip001RightFinger11, // Bip001_R_Finger11
    Bip001RightFinger21, // Bip001_R_Finger21
    Fire01, // fire_01
    Fire02, // fire_02
}

/// ## Humanoid Bones
/// 인간 형태 모델을 구성하는 뼈의 `Entity`를 저장합니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Humanoid(HashMap<BoneKind, Entity>);

impl Humanoid {
    /// 새로운 뼈 `Entity` 집합을 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }

    /// `BoneKind`에 해당하는 `Entity`를 삽입합니다.  
    /// 이미 해당 `Entity`가 존재하는 경우 기존의 `Entity`는 교체됩니다.
    pub fn insert(&mut self, k: BoneKind, v: Entity) -> Option<Entity> {
        self.0.insert(k, v)
    }

    /// `BoneKind`에 해당하는 `Entity`를 제거합니다.  
    /// 해당 `Entity`가 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove(&mut self, k: &BoneKind) -> Option<Entity> {
        self.0.remove(k)
    }

    /// `BoneKind`에 해당하는 `Entity`를 반환합니다.  
    /// 해당 `Entity`가 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get(&self, k: &BoneKind) -> Option<&Entity> {
        self.0.get(k)
    }
}

impl Default for Humanoid {
    fn default() -> Self {
        Self(HashMap::default())
    }
}
