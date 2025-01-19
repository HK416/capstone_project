/// # Maximum Character Speed
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MaxCharacterSpeed(pub f32);

/// # Velocity
///
/// ## Note
/// SIMD 지원을 위해 4차원 벡터를 사용함.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity(pub glam::Vec4);

impl Default for Velocity {
    fn default() -> Self {
        Self(glam::Vec4::ZERO)
    }
}

/// # Character Inverse Mass
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CharacterInvMass(pub f32);

impl Default for CharacterInvMass {
    fn default() -> Self {
        Self(0.0)
    }
}

/// # Force
///
/// ## Note
/// SIMD 지원을 위해 4차원 벡터를 사용함.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Force(pub glam::Vec4);

impl Default for Force {
    fn default() -> Self {
        Self(glam::Vec4::ZERO)
    }
}

/// # Acceleration
///
/// ## Note
/// SIMD 지원을 위해 4차원 벡터를 사용함.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Acceleration(pub glam::Vec4);

impl Default for Acceleration {
    fn default() -> Self {
        Self(glam::Vec4::ZERO)
    }
}
