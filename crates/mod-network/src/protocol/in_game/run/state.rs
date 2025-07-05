//! 일정 주기마다 지속되는 입력과 변위를 전송하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, HeldInput, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 인게임 장면에서 클라이언트에서 서버로 일정 주기마다 지속 입력 상태를 전송하는 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGameInputStatePacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 월드 공간 x축 좌표의 변위
    pub delta_x: f32,
    /// 월드 공간 y축 좌표의 변위
    pub delta_y: f32,
    /// 월드 공간 z축 좌표의 변위
    pub delta_z: f32,
    /// 카메라 위도의 변위
    pub delta_lat: f32,
    /// 카메라 경도의 변위
    pub delta_lon: f32,
    /// 현재 입력 데이터
    pub held_input: HeldInput,
    /// 클라이언트 게임 플레이 경과 시간
    pub play_elapsed_time_ms: u32,
}

impl InGameInputStatePacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(
        uid: UserId,
        token: LoginToken,
        delta_x: f32,
        delta_y: f32,
        delta_z: f32,
        delta_lat: f32,
        delta_lon: f32,
        held_input: HeldInput,
        play_elapsed_time_ms: u32,
    ) -> Self {
        Self {
            uid,
            token,
            delta_x,
            delta_y,
            delta_z,
            delta_lat,
            delta_lon,
            held_input,
            play_elapsed_time_ms,
        }
    }
}

impl Packet for InGameInputStatePacket {
    fn packet_type() -> PacketType {
        PacketType::InGameInputState
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size()
            + LoginToken::byte_size()
            + f32::byte_size()
            + f32::byte_size()
            + f32::byte_size()
            + f32::byte_size()
            + f32::byte_size()
            + HeldInput::byte_size()
            + u32::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.delta_x.to_big_endian_bytes());
        data.extend_from_slice(&self.delta_y.to_big_endian_bytes());
        data.extend_from_slice(&self.delta_z.to_big_endian_bytes());
        data.extend_from_slice(&self.delta_lat.to_big_endian_bytes());
        data.extend_from_slice(&self.delta_lon.to_big_endian_bytes());
        data.extend_from_slice(&self.held_input.to_big_endian_bytes());
        data.extend_from_slice(&self.play_elapsed_time_ms.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameInputStatePacket)
            )
        };

        RawPacket::new(Self::packet_type(), data)
    }

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

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 로그인 토근을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        // x축 이동 변위를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let delta_x = f32::from_big_endian_bytes(data);

        // y축 이동 변위를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let delta_y = f32::from_big_endian_bytes(data);

        // z축 이동 변위를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let delta_z = f32::from_big_endian_bytes(data);

        // 카메라 위도 변위를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let delta_lat = f32::from_big_endian_bytes(data);

        // 카메라 경도 변위를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let delta_lon = f32::from_big_endian_bytes(data);

        // 지속 입력 데이터를 가져옵니다.
        offset = offset + size;
        size = HeldInput::byte_size();
        data = &bytes[offset..offset + size];
        let held_input = HeldInput::from_big_endian_bytes(data);

        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let play_elapsed_time_ms = u32::from_big_endian_bytes(data);

        Some(Self {
            uid,
            token,
            delta_x,
            delta_y,
            delta_z,
            delta_lat,
            delta_lon,
            held_input,
            play_elapsed_time_ms,
        })
    }
}
