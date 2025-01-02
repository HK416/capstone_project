mod animation;
mod camera;
mod character;
mod control;
mod physics;
mod transform;

use std::ops::{Deref, DerefMut};

use hecs::Entity;
use mod_app::app::FIXED_TIME_SEC;

pub use self::{animation::*, camera::*, character::*, control::*, physics::*, transform::*};

/// 최대 컨트롤러 입력 시간입니다.
pub const MAX_CONTROL_INPUT_TIME: f32 = 0.3;

/// 최대 줌 인/아웃 시간입니다.
pub const MAX_IN_OUT_TIME: f32 = 0.3;
static_assertions::const_assert!(MAX_IN_OUT_TIME > FIXED_TIME_SEC);

/// ## Parent Entity
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Parent(pub Entity);

impl Deref for Parent {
    type Target = Entity;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Parent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// ## Child Entity
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Child(pub Entity);

impl Deref for Child {
    type Target = Entity;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Child {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// ## Sibling Entity
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sibling(pub Entity);

impl Deref for Sibling {
    type Target = Entity;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Sibling {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// ## Timer
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Timer(pub f32);

impl Default for Timer {
    fn default() -> Self {
        Self(0.0)
    }
}
