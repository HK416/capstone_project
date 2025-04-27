//! 게임을 진행하기 전에 대기하는 단계와 관련된 코드를 관리합니다.
//!
//
//    +--------+                  +--------+
//    | client |                  | server |
//    +--------+                  +--------+
//         |                           |
//         |<------[PrepareStage]------|
//         |                           |
//   +------------+                    |
//   |   switch   |                    |
//   | game scene |                    |
//   +------------+                    |
//         |                      +----------+
//         |                      |  switch  |
//         |                      |  state   |
//         |                      +----------+
//         |                           |
//         |<-------[PullStage]--------|
//         |                           |
//   +------------+                    |
//   |   switch   |                    |
//   | game scene |                    |
//   +------------+                    |
//         |                           |
//        ...                         ...
//

use crate::{
    components::BigEndian,
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 게임 준비 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct PrepareStagePacket {
    pub elapsed_time_sec: f32,
}

impl PrepareStagePacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(elapsed_time_sec: f32) -> Self {
        Self { elapsed_time_sec }
    }
}

impl Packet for PrepareStagePacket {
    fn packet_type() -> PacketType {
        PacketType::PrepareStage
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = f32::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.elapsed_time_sec.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PrepareStagePacket)
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
                Self::packet_type()
            );
            return None;
        }

        // 경과 시간 데이터를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = f32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let elapsed_time_sec = f32::from_big_endian_bytes(data);

        Some(Self { elapsed_time_sec })
    }
}
