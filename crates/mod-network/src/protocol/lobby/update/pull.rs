//! 클라이언트가 로비 장면에 있을 때 데이터 갱신을 위한 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, NetworkState, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트가 로비 장면에 있을 때 서버에서 클라이언트로 보내는 데이터 갱신 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbyPullPacket {
    /// 시대 정보 (핑 테스트를 위한 데이터)
    pub epoch: u64,
    /// 네트워크 통신 상태
    pub network_state: NetworkState,
}

impl LobbyPullPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(epoch: u64, network_state: NetworkState) -> Self {
        Self {
            epoch,
            network_state,
        }
    }
}

impl Packet for LobbyPullPacket {
    fn packet_type() -> PacketType {
        PacketType::LobbyPull
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = u64::byte_size() + NetworkState::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&self.network_state.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LobbyPullPacket)
            )
        };

        RawPacket::new(Self::packet_type(), &data)
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

        // 시대를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u64::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = u64::from_big_endian_bytes(data);

        // 네트워크 통신 상태를 가져옵니다.
        offset = offset + size;
        size = NetworkState::byte_size();
        data = &bytes[offset..offset + size];
        let network_state = NetworkState::try_from_big_endian_bytes(data)?;

        Some(Self {
            epoch,
            network_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lobby_pull_packet() {
        let origin = LobbyPullPacket::new(12, NetworkState::Good);
        let raw = origin.as_raw();
        let other = LobbyPullPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
