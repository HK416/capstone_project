//! 파티클과 관련된 쉐이더 리소스 코드를 관리합니다.
//!

mod muzzle;

use std::sync::Arc;

pub use self::muzzle::*;

/// 파티클 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleResource(Arc<wgpu::BindGroup>);

impl ParticleResource {
    /// 새로운 파티클 쉐이더 리소스를 생성합니다.
    pub const fn new(bind_group: Arc<wgpu::BindGroup>) -> Self {
        Self(bind_group)
    }

    /// [`wgpu::BindGroup`]을 가져옵니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.0
    }
}
