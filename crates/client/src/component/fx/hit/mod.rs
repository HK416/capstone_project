//! 총알에 피격될 때 발생하는 이펙트와 관련된 코드를 관리합니다.
//!

mod instance;
mod pipeline;

use hecs::World;

use crate::{
    component::{LifeTime, Parent, PlayerArchetype, ToParentTrans, WorldTransform},
    player_execute,
};

pub use self::{instance::*, pipeline::*};

/// FX_TEX_Hit_00 피격 이펙트의 태그
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FxHit00(pub u32);

/// 피격 이펙트의 색상입니다.
#[derive(Debug, Clone, Copy)]
pub struct FxHitTintColor(pub [f32; 3]);

/// 총구 화염 파티클 이펙트 엔터티를 갱신합니다.
///
/// # Note
/// 이 함수는 캐릭터의 월드 변환 행렬을 갱신 후 호출되어야 합니다.
///
pub fn update_fx_hit_00_particles(
    world: &World,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    instances: &FxHitInstace,
) {
    type Q<'a> = (
        &'a FxHit00,
        &'a LifeTime,
        &'a Parent,
        &'a PlayerArchetype,
        &'a ToParentTrans,
        &'a FxHitTintColor,
    );
    let mut query = world.query::<Q>();
    for (_entity, components) in query.iter() {
        let (&tag, &lifetime, &parent, &archetype, local_transform, tint) = components;

        // 인스턴스 뷰를 가져옵니다.
        let instance_view = instances.get();

        // 부모 엔터티의 월드 변환 행렬을 가져옵니다.
        let entity = parent.0;
        player_execute!(
            archetype,
            world,
            entity,
            &WorldTransform,
            |world_transform| {
                let trans = world_transform.0 * local_transform.0;
                let data = FxHitInstanceDataLayout {
                    x_axis: trans.x_axis.to_array(),
                    y_axis: trans.y_axis.to_array(),
                    z_axis: trans.z_axis.to_array(),
                    w_axis: trans.w_axis.to_array(),
                    tint: [
                        tint.0[0],
                        tint.0[1],
                        tint.0[2],
                        lifetime.maximum as f32 / lifetime.remaining as f32,
                    ],
                    index: tag.0,
                };
                instance_view.write(device, encoder, staging_buffers, &data);
            }
        );
    }
}
