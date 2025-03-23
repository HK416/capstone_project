//! 게임 진행 단계일 때 플레이어 상태와 관련된 코드를 관리합니다.
//!

pub mod action;
pub mod movement;
pub mod view;

pub use self::{action::*, movement::*, view::*};
