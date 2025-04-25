//! 스테이지 객체와 관련된 코드를 관리합니다.
//!

mod city;
mod pipeline;
mod spawn;

use mod_network::components::{MAX_IN_GAME_TEAM_PLAYERS, NUM_STAGES};

pub use self::{pipeline::*, spawn::*};

/// 승리 팀의 회전방향입니다.
pub const RESET_ROTATION: [[glam::Quat; 2]; NUM_STAGES] = [city::RESET_ROTATION];

/// 승리 팀의 위치입니다.
pub const RESET_POSITIONS: [[glam::Vec3; MAX_IN_GAME_TEAM_PLAYERS]; NUM_STAGES] =
    [city::RESET_POSITIONS];

/// 스테이지 객체 태그
pub struct StageTag;
