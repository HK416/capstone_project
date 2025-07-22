//! 파티클 효과와 관련된 코드를 관리합니다.
//!

mod muzzle;
mod resources;

use hecs::World;

pub use self::{muzzle::*, resources::*};

/// 파티클의 남은 시간입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LifeTime(pub u32);

/// 파티클 생명 주기를 갱신합니다.
pub fn update_fx_particle_lifetime(world: &mut World, elapsed_time_ms: u32) {
    let mut removed = Vec::default();
    {
        let mut query = world.query::<&mut LifeTime>();
        for (entity, life_time) in query.iter() {
            // 파티클의 생명 주기를 갱신합니다.
            life_time.0 = life_time.0.saturating_sub(elapsed_time_ms);

            // 생명 주기가 0인 파티클을 수집합니다.
            if life_time.0 <= 0 {
                removed.push(entity);
            }
        }
    }

    // 수집한 엔터티를 제거합니다.
    for entity in removed {
        world.despawn(entity).expect("no such entity!");
    }
}
