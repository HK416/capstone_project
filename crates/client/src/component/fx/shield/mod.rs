//! 방어막 이펙트와 관련된 코드를 관리합니다.
//!

mod instance;
mod pipeline;

use hecs::{Entity, World};
use mod_network::components::{CharacterFlags, HealthData};

use crate::{
    component::{PlayerArchetype, WorldTransform},
    player_execute,
};

pub use self::{instance::*, pipeline::*};

/// 방어막 파티클 이펙트 엔터티를 갱신합니다.
///
/// # Note
/// 이 함수는 캐릭터의 월드 변환 행렬을 갱신 후 호출되어야 합니다.
///
pub fn update_fx_shield_particle(
    world: &World,
    entity: Entity,
    archetype: PlayerArchetype,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    shield_instances: &FxShieldInstance,
) {
    let mut query = world
        .query_one::<&CharacterFlags>(entity)
        .expect("invalid entity");
    let character_flags = query.get().expect("invalid entity component!");
    if !character_flags.is_connected() {
        return;
    }

    type Q<'a> = (&'a HealthData, &'a WorldTransform);
    player_execute!(archetype, world, entity, Q, |(
        health_data,
        world_transform,
    )| {
        if health_data.shield > 0 {
            let offset = glam::Mat4::from_scale_rotation_translation(
                glam::Vec3::splat(0.5),
                glam::Quat::IDENTITY,
                glam::vec3(0.0, 0.5, 0.0),
            );
            let transform = world_transform.0 * offset;

            let data = FxShieldInstanceDataLayout {
                x_axis: transform.x_axis.to_array(),
                y_axis: transform.y_axis.to_array(),
                z_axis: transform.z_axis.to_array(),
                w_axis: transform.w_axis.to_array(),
            };
            let instance_view = shield_instances.get();
            instance_view.write(device, encoder, staging_buffers, &data);
        }
    });
}
