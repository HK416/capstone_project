#![allow(unused)]

use std::{path::Path, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_render::{
    Attributes, Indices, Mesh, MeshPool, MeshResource, SkinningDataLayout, TexturePool, Vertices,
    MAX_BONES,
};

use crate::{
    asset::blob::Matrix,
    component::{BoneCollection, Child, Parent, Sibling, ToParentTrans, WorldTransform},
};

use super::blob::{Float2, Float3, Float4, MeshBlob, NodeBlob, TextureBlob, Uint4};

/// ## General Humanoid Bone Kind
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoneKind {
    BoneRoot,               // bone_root
    Bip001,                 // Bip001
    Bip001Pelvis,           // Bip001_Pelvis
    Bip001Weapon,           // Bip001_Weapon
    Bip001LeftThigh,        // Bip001_L_Thigh
    Bip001RightThigh,       // Bip001_R_Thigh
    Bip001Spine,            // Bip001_Spine
    Bip001LeftCalf,         // Bip001_L_Calf
    Bip001LeftFoot,         // Bip001_L_Foot
    Bip001LeftToe0,         // Bip001_L_Toe0
    Bip001RightCalf,        // Bip001_R_Calf
    Bip001RightFoot,        // Bip001_R_Foot
    Bip001RightToe0,        // Bip001_R_Toe0
    Bip001Spine1,           // Bip001_Spine1
    Bip001LeftClavicle,     // Bip001_L_Clavicle
    Bip001Neck,             // Bip001_Neck
    Bip001RightClavicle,    // Bip001_R_Clavicle
    Bip001LeftUpperArm,     // Bip001_L_UpperArm
    Bip001BoneLeftDeltoid,  // Bip001_B_L_Deltoid
    Bip001LeftForearm,      // Bip001_L_Forearm
    Bip001LeftHand,         // Bip001_L_Hand
    Bip001LeftFinger0,      // Bip001_L_Finger0
    Bip001LeftFinger1,      // Bip001_L_Finger1
    Bip001LeftFinger2,      // Bip001_L_Finger2
    Bip001LeftFinger01,     // Bip001_L_Finger01
    Bip001LeftFinger11,     // Bip001_L_Finger11
    Bip001LeftFinger21,     // Bip001_L_Finger21
    Bip001Head,             // Bip001_Head
    Bip001RightUpperArm,    // Bip001_R_UpperArm
    Bip001BoneRightDeltoid, // Bip001_B_R_Deltoid
    Bip001RightForearm,     // Bip001_R_Forearm
    Bip001RightHand,        // Bip001_R_Hand
    Bip001RightFinger0,     // Bip001_R_Finger0
    Bip001RightFinger1,     // Bip001_R_Finger1
    Bip001RightFinger2,     // Bip001_R_Finger2
    Bip001RightFinger01,    // Bip001_R_Finger01
    Bip001RightFinger11,    // Bip001_R_Finger11
    Bip001RightFinger21,    // Bip001_R_Finger21
    Fire01,                 // fire_01
    Fire02,                 // fire_02
    Other(String),
}

impl BoneKind {
    /// 뼈 이름으로부터 `Humanoid` 뼈 종류를 생성합니다.
    pub fn from_str(name: &str) -> Self {
        match name {
            "bone_root" => BoneKind::BoneRoot,
            "Bip001" => BoneKind::Bip001,
            "Bip001_Pelvis" => BoneKind::Bip001Pelvis,
            "Bip001_Weapon" => BoneKind::Bip001Weapon,
            "Bip001_L_Thigh" => BoneKind::Bip001LeftThigh,
            "Bip001_R_Thigh" => BoneKind::Bip001RightThigh,
            "Bip001_Spine" => BoneKind::Bip001Spine,
            "Bip001_L_Calf" => BoneKind::Bip001LeftCalf,
            "Bip001_L_Foot" => BoneKind::Bip001LeftFoot,
            "Bip001_L_Toe0" => BoneKind::Bip001LeftToe0,
            "Bip001_R_Calf" => BoneKind::Bip001RightCalf,
            "Bip001_R_Foot" => BoneKind::Bip001RightFoot,
            "Bip001_R_Toe0" => BoneKind::Bip001RightToe0,
            "Bip001_Spine1" => BoneKind::Bip001Spine1,
            "Bip001_L_Clavicle" => BoneKind::Bip001LeftClavicle,
            "Bip001_Neck" => BoneKind::Bip001Neck,
            "Bip001_R_Clavicle" => BoneKind::Bip001RightClavicle,
            "Bip001_L_UpperArm" => BoneKind::Bip001LeftUpperArm,
            "Bip001_B_L_Deltoid" => BoneKind::Bip001BoneLeftDeltoid,
            "Bip001_L_Forearm" => BoneKind::Bip001LeftForearm,
            "Bip001_L_Hand" => BoneKind::Bip001LeftHand,
            "Bip001_L_Finger0" => BoneKind::Bip001LeftFinger0,
            "Bip001_L_Finger1" => BoneKind::Bip001LeftFinger1,
            "Bip001_L_Finger2" => BoneKind::Bip001LeftFinger2,
            "Bip001_L_Finger01" => BoneKind::Bip001LeftFinger01,
            "Bip001_L_Finger11" => BoneKind::Bip001LeftFinger11,
            "Bip001_L_Finger21" => BoneKind::Bip001LeftFinger21,
            "Bip001_Head" => BoneKind::Bip001Head,
            "Bip001_R_UpperArm" => BoneKind::Bip001RightUpperArm,
            "Bip001_B_R_Deltoid" => BoneKind::Bip001BoneRightDeltoid,
            "Bip001_R_Forearm" => BoneKind::Bip001RightForearm,
            "Bip001_R_Hand" => BoneKind::Bip001RightHand,
            "Bip001_R_Finger0" => BoneKind::Bip001RightFinger0,
            "Bip001_R_Finger1" => BoneKind::Bip001RightFinger1,
            "Bip001_R_Finger2" => BoneKind::Bip001RightFinger2,
            "Bip001_R_Finger01" => BoneKind::Bip001RightFinger01,
            "Bip001_R_Finger11" => BoneKind::Bip001RightFinger11,
            "Bip001_R_Finger21" => BoneKind::Bip001RightFinger21,
            "fire_01" => BoneKind::Fire01,
            "fire_02" => BoneKind::Fire02,
            _ => BoneKind::Other(name.to_string()),
        }
    }
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
