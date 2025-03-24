use crate::{
    components::{BigEndian, SelectResult, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 캐릭터 선택 응답 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationSelectResponsePacket {
    pub result: SelectResult,
}

impl FormationSelectResponsePacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(result: SelectResult) -> Self {
        Self { result }
    }
}

impl Packet for FormationSelectResponsePacket {
    fn packet_type() -> PacketType {
        PacketType::FormationSelectResponse
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = SelectResult::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.result.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FormationSelectResponsePacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
    }

    #[allow(unused_mut)]
    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::warn!(
                "invalid packet type. (RAW:{:?}, PACKET:{:?})",
                raw.packet_type(),
                Self::packet_type(),
            );
            return None;
        }

        // 선택 결과를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = SelectResult::byte_size();
        let mut data = &bytes[offset..offset + size];
        let result = SelectResult::try_from_big_endian_bytes(data)?;

        Some(Self { result })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formation_select_response_packet() {
        let origin = FormationSelectResponsePacket::new(SelectResult::Duplicates);
        let raw = origin.as_raw();
        let other = FormationSelectResponsePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
