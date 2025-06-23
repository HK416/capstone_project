//! 플레이어 타이머와 관련된 코드를 관리합니다.
//!

use half::f16;

use crate::components::BigEndian;

/// 플레이어 행동 상태 타이머입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ActionStateTimer(f16);

impl ActionStateTimer {
    /// 새로운 행동 상태 타이머를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `value`가 0보다 작은 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "the given value is less than zero!");
        Self(f16::from_f32_const(value))
    }

    /// 값을 설정합니다.
    pub const fn set(&mut self, value: f32) {
        self.0 = f16::from_f32_const(value)
    }

    /// 값을 가져옵니다.
    pub const fn get(&self) -> f32 {
        self.0.to_f32_const()
    }

    /// 타이머를 갱신합니다.  
    ///
    /// 갱신된 타이머의 값이 `maximum`보다 큰 경우 초과한 만큼의 양수 값을 반환합니다.  
    /// 갱신된 타이머의 값이 `maximum`보다 작은 경우 부족한 만큼의 음수 값을 반환합니다.
    ///
    pub const fn advanced(&mut self, elapsed: f32, maximum: f32) -> f32 {
        let time = self.0.to_f32_const() + elapsed;
        let diff = time - maximum;
        self.0 = f16::from_f32_const(time % maximum);
        diff
    }
}

impl BigEndian for ActionStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(f16::from_bits(u16::from_big_endian_bytes(bytes)))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_bits().to_big_endian_bytes()
    }
}

impl Default for ActionStateTimer {
    fn default() -> Self {
        Self(f16::from_f32(0.0))
    }
}

/// 플레이어 움직임 상태 타이머입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MovementStateTimer(f16);

impl MovementStateTimer {
    /// 새로운 행동 상태 타이머를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `value`가 0보다 작은 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "the given value is less than zero!");
        Self(f16::from_f32_const(value))
    }

    /// 값을 설정합니다.
    pub const fn set(&mut self, value: f32) {
        self.0 = f16::from_f32_const(value)
    }

    /// 값을 가져옵니다.
    pub const fn get(&self) -> f32 {
        self.0.to_f32_const()
    }

    /// 타이머를 갱신합니다.  
    ///
    /// 갱신된 타이머의 값이 `maximum`보다 큰 경우 초과한 만큼의 양수 값을 반환합니다.  
    /// 갱신된 타이머의 값이 `maximum`보다 작은 경우 부족한 만큼의 음수 값을 반환합니다.
    ///
    pub const fn advanced(&mut self, elapsed: f32, maximum: f32) -> f32 {
        let time = self.0.to_f32_const() + elapsed;
        let diff = time - maximum;
        self.0 = f16::from_f32_const(time % maximum);
        diff
    }
}

impl BigEndian for MovementStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(f16::from_bits(u16::from_big_endian_bytes(bytes)))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_bits().to_big_endian_bytes()
    }
}

impl Default for MovementStateTimer {
    fn default() -> Self {
        Self(f16::from_f32(0.0))
    }
}

/// 플레이어 시야 상태 타이머입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ViewStateTimer(f16);

impl ViewStateTimer {
    /// 새로운 행동 상태 타이머를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `value`가 0보다 작은 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "the given value is less than zero!");
        Self(f16::from_f32_const(value))
    }

    /// 값을 설정합니다.
    pub const fn set(&mut self, value: f32) {
        self.0 = f16::from_f32_const(value)
    }

    /// 값을 가져옵니다.
    pub const fn get(&self) -> f32 {
        self.0.to_f32_const()
    }

    /// 타이머를 갱신합니다.  
    ///
    /// 갱신된 타이머의 값이 `maximum`보다 큰 경우 초과한 만큼의 양수 값을 반환합니다.  
    /// 갱신된 타이머의 값이 `maximum`보다 작은 경우 부족한 만큼의 음수 값을 반환합니다.
    ///
    pub const fn advanced(&mut self, elapsed: f32, maximum: f32) -> f32 {
        let time = self.0.to_f32_const() + elapsed;
        let diff = time - maximum;
        self.0 = f16::from_f32_const(time % maximum);
        diff
    }
}

impl BigEndian for ViewStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(f16::from_bits(u16::from_big_endian_bytes(bytes)))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_bits().to_big_endian_bytes()
    }
}

impl Default for ViewStateTimer {
    fn default() -> Self {
        Self(f16::from_f32(0.0))
    }
}
