//! 클라이언트가 인게임 준비 장면에 있을 때 사용되는 패킷을 관리합니다.
//!

mod init;
mod sync;

pub use self::{init::*, sync::*};
