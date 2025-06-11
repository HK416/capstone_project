mod lobby;
mod parser;
mod raw;
mod title;

pub use self::{lobby::*, parser::*, raw::*, title::*};

/// 모든 파생 패킷이 구현해야 하는 `triat`입니다.
pub trait Packet: Sized {
    /// 파생 패킷의 타입입니다.
    fn packet_type() -> PacketType;

    /// 패킷을 `RawPacket`으로 변환합니다.
    fn as_raw(&self) -> RawPacket;

    /// `RawPacket`으로부터 패킷을 생성합니다.
    ///
    /// # Panics
    /// 패킷 종류가 다르거나, 패킷을 생성할 수 없는 경우 [`panic!`]을 호출합니다.
    ///
    fn from_raw(raw: RawPacket) -> Self {
        Self::try_from_raw(raw).expect("invalid data")
    }

    /// `RawPacket`으로부터 패킷을 생성합니다.
    ///
    /// 패킷 종류가 다르거나, 패킷을 생성할 수 없는 경우 `None`을 반환합니다.
    ///
    fn try_from_raw(raw: RawPacket) -> Option<Self>;
}
