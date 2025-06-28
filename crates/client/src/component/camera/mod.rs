//! 카메라와 관련된 코드를 관리합니다.
//!

mod third_person;
pub use self::third_person::*;

mod render;
mod resource;
mod uniform;

pub use self::{render::*, resource::*, uniform::*};
