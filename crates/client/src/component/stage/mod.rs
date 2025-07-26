//! 스테이지 객체와 관련된 코드를 관리합니다.
//!

mod pipeline;
mod render;
mod spawn;

pub use self::{pipeline::*, render::*, spawn::*};
