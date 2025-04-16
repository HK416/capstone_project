//! 데미지 폰트와 관련된 코드를 관리합니다.
//!

mod pipeline;
mod resource;
mod uniform;

pub use self::{pipeline::*, resource::*, uniform::*};

/// 데미지 파티클 데이터를 저장합니다.
#[derive(Debug, Clone)]
pub struct DamageParticle {
    /// 파티클 경과 시간입니다.
    pub elapsed_time_sec: f32,
    /// 파티클 지속 시간입니다.
    pub duration_sec: f32,
    /// 파티클 시작 지점 상대 좌표입니다.
    pub begin_offset: glam::Vec3A,
    /// 파티클 끝 지검 상대 좌표입니다.
    pub end_offset: glam::Vec3A,
    /// 0~9까지의 숫자 데이터입니다.
    pub number: u32,
}
