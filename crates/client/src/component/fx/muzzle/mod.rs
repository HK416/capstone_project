//! 총알을 발사할 때 총구의 화염 이펙트와 관련된 코드를 관리합니다.
//!

mod instance;
mod pipeline;

use hecs::{Component, With, World};

use crate::{
    component::{Parent, PlayerArchetype, ToParentTrans, WorldTransform},
    player_execute,
};

pub use self::{instance::*, pipeline::*};

/// FX_TEX_Muzzle_00 총구 화염 이펙트의 태그
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FxMuzzle00;

/// FX_TEX_Muzzle_01 총구 화염 이펙트의 태그
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FxMuzzle01;

/// 총구 화염 이펙트의 색상입니다.
#[derive(Debug, Clone, Copy)]
pub struct FxMuzzleTintColor(pub [f32; 3]);

impl Default for FxMuzzleTintColor {
    fn default() -> Self {
        Self([1.0; 3])
    }
}

/// 총구 화염 파티클 이펙트 엔터티를 갱신합니다.
///
/// # Note
/// 이 함수는 캐릭터의 월드 변환 행렬을 갱신 후 호출되어야 합니다.
///
pub fn update_fx_muzzle_particles<Tag: Component>(
    world: &World,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    muzzle_instances: &FxMuzzleInstance,
) {
    type Q<'a> = (
        &'a Parent,
        &'a PlayerArchetype,
        &'a ToParentTrans,
        &'a FxMuzzleTintColor,
    );
    let mut query = world.query::<With<Q, &Tag>>();
    for (_entity, components) in query.iter() {
        let (&parent, &archetype, local_transform, tint) = components;

        // 인스턴스 뷰를 가져옵니다.
        let instance_view = muzzle_instances.get();

        // 부모 엔터티의 월드 변환 행렬을 가져옵니다.
        let entity = parent.0;
        player_execute!(
            archetype,
            world,
            entity,
            &WorldTransform,
            |world_transform| {
                let trans = world_transform.0 * local_transform.0;
                let data = FxMuzzleInstanceDataLayout {
                    x_axis: trans.x_axis.to_array(),
                    y_axis: trans.y_axis.to_array(),
                    z_axis: trans.z_axis.to_array(),
                    w_axis: trans.w_axis.to_array(),
                    tint: tint.0,
                    index: rand::random_range(0..4),
                };
                instance_view.write(device, encoder, staging_buffers, &data);
            }
        );
    }
}
