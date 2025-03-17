//! 게임 내 등장하는 개별 객체들을 기능별로 모듈화합니다.
//!

pub mod bullet;
pub mod character;
pub mod player;
pub mod stage;
pub mod state;

pub use self::{bullet::*, character::*, player::*, stage::*, state::*};
