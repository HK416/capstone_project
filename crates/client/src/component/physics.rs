/// # Direction
/// 플레이어가 이동하고자 하는 방향을 나타냅니다. (캐릭터가 바라보는 방향과 다를 수 있습니다)
///
/// ## Note
/// SIMD 지원을 위해 4차원 벡터를 사용함.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direction(pub glam::Vec4);

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

/// # Character Inverse Mass
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CharacterInvMass(pub f32);

/// # Force
///
/// ## Note
/// SIMD 지원을 위해 4차원 벡터를 사용함.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Force(pub glam::Vec4);

/// # Acceleration
///
/// ## Note
/// SIMD 지원을 위해 4차원 벡터를 사용함.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Acceleration(pub glam::Vec4);
