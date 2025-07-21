//! 플레이어 데이터와 관련된 코드를 관리합니다.
//!

mod finish;
mod init;
mod pull;

pub use self::{finish::*, init::*, pull::*};

/// 플레이어의 최대 도약 시간입니다. (단위: ms)
pub const MAX_JUMP_DURATION: u16 = 250;

/// 플레이어 리스폰 대기 시간 (단위: ms)
pub const RESPAWN_DELAY: u16 = 10_000;
