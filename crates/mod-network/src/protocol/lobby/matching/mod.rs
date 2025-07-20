//! 클라이언트가 로비 장면에 있을 때 사용자의 이벤트에 의한 랜덤매치 참여와 관련된 패킷을 관리합니다.
//!

mod cancel;
mod reject;
mod request;

pub use self::{cancel::*, reject::*, request::*};