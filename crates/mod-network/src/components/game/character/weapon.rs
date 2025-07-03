//! 캐릭터 무기와 관련된 코드를 관리합니다.
//! 

// 캐릭터 뼈 구조 
// - *: L or R
// - (): Optional
//
// bone_root
//  Bip001
//      Bip001_pelvis
//          Bip001_Spine
//              Bip001_Spine_1
//                  Bip001_*_Clavicle
//                      Bip001_*_UpperArm
//                          Bip001_*_Forearm
//                              Bip001_*_Hand
//                  Bip001_Neck
//                      Bip001_Head
//      Bip001_Weapon(_*)
//          fire_01

use serde::{Deserialize, Serialize};

use crate::components::{Float3, Float4x4};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct WeaponBindposeData {
    pub bone_root: Float4x4,
    pub bip001: Float4x4,
    pub bip001_pelvis: Float4x4,
    pub bip001_spine: Float4x4,
    pub bip001_spine_axis: Float3,
    pub bip001_spine1: Float4x4,
    pub bip001_spine1_axis: Float3,
    pub bip001_clavicle: Float4x4,
    pub bip001_upperarm: Float4x4,
    pub bip001_forearm: Float4x4,
    pub bip001_hand: Float4x4,
    pub bip001_neck: Float4x4,
    pub bip001_head: Float4x4,
    pub bip001_head_axis: Float3,
    pub bip001_weapon: Float4x4,
    pub hand_weapon_offset: Float4x4,
    pub bip001_fire: Float4x4,
}

/// 무기의 월드 변환 행렬 헬퍼
/// 
/// 특정 각도의 무기의 로컬 변환 행렬을 구하거나, 무기의 총구 위치를 계산하는데 사용됩니다.
/// 
pub struct WeaponTransformHelper {
    pub root: Box<WeaponTransformNode>,
}

impl WeaponTransformHelper {
    /// 새로운 무기 변환 행렬 헬퍼를 생성합니다.
    pub fn new(offset: f32, angle: f32, data: &WeaponBindposeData) -> Self {
        let mut bip001_head: glam::Mat4 = data.bip001_head.into();
        let bip001_head_offset = glam::Mat4::from_axis_angle(data.bip001_head_axis.into(), angle / 7.0 * offset);
        bip001_head *= bip001_head_offset;

        let mut bip001_spine: glam::Mat4 = data.bip001_spine.into();
        let bip001_spine_offset = glam::Mat4::from_axis_angle(data.bip001_spine_axis.into(), 3.0 * angle / 7.0 * offset);
        bip001_spine *= bip001_spine_offset;

        let mut bip001_spine1: glam::Mat4 = data.bip001_spine1.into();
        let bip001_spine1_offset = glam::Mat4::from_axis_angle(data.bip001_spine1_axis.into(), 3.0 * angle / 7.0 * offset);
        bip001_spine1 *= bip001_spine1_offset;

        Self { 
            root: Box::new(WeaponTransformNode {
                transform: data.bone_root.into(),
                child: Some(Box::new(WeaponTransformNode {
                    transform: data.bip001.into(),
                    child: Some(Box::new(WeaponTransformNode {
                        transform: data.bip001_pelvis.into(),
                        child: Some(Box::new(WeaponTransformNode {
                            transform: bip001_spine,
                            child: Some(Box::new(WeaponTransformNode {
                                transform: bip001_spine1,
                                child: Some(Box::new(WeaponTransformNode {
                                    transform: data.bip001_clavicle.into(),
                                    child: Some(Box::new(WeaponTransformNode {
                                        transform: data.bip001_upperarm.into(),
                                        child: Some(Box::new(WeaponTransformNode { 
                                            transform: data.bip001_forearm.into(), 
                                            child: Some(Box::new(WeaponTransformNode {
                                                transform: data.bip001_hand.into(),
                                                child: None,
                                                sibling: None,
                                            })), 
                                            sibling: None 
                                        })),
                                        sibling: None,
                                    })),
                                    sibling: Some(Box::new(WeaponTransformNode {
                                        transform: data.bip001_neck.into(),
                                        child: Some(Box::new(WeaponTransformNode {
                                            transform: bip001_head,
                                            child: None,
                                            sibling: None,
                                        })),
                                        sibling: None,
                                    })),
                                })),
                                sibling: None,
                            })),
                            sibling: None,
                        })),
                        sibling: Some(Box::new(WeaponTransformNode {
                            transform: data.bip001_weapon.into(),
                            child: Some(Box::new(WeaponTransformNode {
                                transform: data.bip001_fire.into(),
                                child: None,
                                sibling: None,
                            })),
                            sibling: None,
                        }))
                    })),
                    sibling: None,
                })),
                sibling: None,
            })
        }
    }
}

/// 무기 변환 행렬 노드입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponTransformNode {
    transform: glam::Mat4,
    child: Option<Box<WeaponTransformNode>>,
    sibling: Option<Box<WeaponTransformNode>>,
}


