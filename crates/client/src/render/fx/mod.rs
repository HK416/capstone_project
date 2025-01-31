mod damage;

pub use self::damage::*;

/// 파티클의 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParticleKind {
    Damage = 0,
}

/// 파티클의 남은 시간을 저장합니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LifeTime(pub f32);

impl Default for LifeTime {
    fn default() -> Self {
        Self(0.0)
    }
}
