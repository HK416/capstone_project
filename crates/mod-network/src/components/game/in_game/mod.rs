//! 게임 진행 단계와 관련된 코드를 관리합니다.
//!

mod bullet;
mod capture_point;
mod damage;
mod player;
mod snapshot;

pub use self::{bullet::*, capture_point::*, damage::*, player::*, snapshot::*};

/// 게임에 참여 가능한 최대 플레이어 수 입니다.
pub const MAX_IN_GAME_PLAYERS: usize = 10;
static_assertions::const_assert!(MAX_IN_GAME_PLAYERS > 0);

/// 게임에 참여 가능한 한 팀당 최대 플레이어 수 입니다.
pub const MAX_IN_GAME_TEAM_PLAYERS: usize = MAX_IN_GAME_PLAYERS / 2;
static_assertions::const_assert!(MAX_IN_GAME_TEAM_PLAYERS > 0);
