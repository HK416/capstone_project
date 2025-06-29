mod snapshot;
mod state;
mod update;

pub use self::state::*;

use self::{snapshot::*, update::*};

/// 플레이어 리스폰 대기 시간 (단위: ms)
pub const RESPAWN_DELAY: u16 = 10_000;
