//! 게임 월드와 관련된 질의를 하는 패킷을 관리합니다.
//!

use crate::{
    components::{BigEndian, LoginToken, UserId, WorldId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트에서 서버로 보내는 접속 가능한 월드 리스트를 요청하는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryWorldListPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
}

impl QueryWorldListPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(uid: UserId, token: LoginToken) -> Self {
        Self { uid, token }
    }
}

impl Packet for QueryWorldListPacket {
    fn packet_type() -> PacketType {
        PacketType::QueryWorldLists
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size() + LoginToken::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(QueryWorldListPacket)
            )
        };

        RawPacket::new(Self::packet_type(), data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type! (SRC:{:?}, DST:{:?})",
                raw.packet_type(),
                Self::packet_type(),
            );
            return None;
        }

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self { uid, token })
    }
}

/// 서버에서 클라이언트로 보내는 접속 가능한 월드 리스트 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldListPacket {
    /// 월드 식별자 목록입니다.
    pub worlds: Vec<WorldId>,
}

impl WorldListPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(worlds: Vec<WorldId>) -> Self {
        Self { worlds }
    }

    /// 반복자로부터 새로운 패킷을 생성합니다.
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = WorldId>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(iter.into_iter().collect())
    }
}

impl Packet for WorldListPacket {
    fn packet_type() -> PacketType {
        PacketType::ResponseWorldList
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_worlds = self.worlds.len();
        let data_size = u32::byte_size() + WorldId::byte_size() * num_worlds;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&(num_worlds as u32).to_big_endian_bytes());
        for world in self.worlds.iter().copied() {
            data.extend_from_slice(&world.to_big_endian_bytes());
        }

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(WorldListPacket)
            )
        };

        RawPacket::new(Self::packet_type(), data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type! (SRC:{:?}, DST:{:?})",
                raw.packet_type(),
                Self::packet_type(),
            );
            return None;
        }

        // 월드 식별자 수를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let mut num_worlds = u32::from_big_endian_bytes(data);

        // 월드 식별자 목록을 가져옵니다.
        let mut worlds = Vec::with_capacity(num_worlds as usize);
        while num_worlds > 0 {
            offset = offset + size;
            size = WorldId::byte_size();
            data = &bytes[offset..offset + size];
            worlds.push(WorldId::from_big_endian_bytes(data));
            num_worlds -= 1;
        }

        Some(Self { worlds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_world_list_packet() {
        let origin = QueryWorldListPacket::new(UserId::new(1235135), LoginToken::new(135415611));
        let raw = origin.as_raw();
        let other = QueryWorldListPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }

    #[test]
    fn test_world_list_packet() {
        let origin =
            WorldListPacket::from_iter([WorldId::new(1), WorldId::new(2), WorldId::new(3)]);
        let raw = origin.as_raw();
        let other = WorldListPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
