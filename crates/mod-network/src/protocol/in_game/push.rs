use crate::{
    components::{
        BigEndian, GameInputBits, LatLon, LoginToken, TryFromBigEndian, UserId, ViewState,
        ViewStateTimer,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트에서 서버로 보내는 플레이어 정보를 갱신하기 위한 패킷
#[derive(Debug, Clone, PartialEq)]
pub struct PushStatusPacket {
    /// 사용자 식별자
    pub user_id: UserId,
    /// 사용자 로그인 토큰
    pub token: LoginToken,
    /// 플레이어 캐릭터가 바라보는 방향 (이동 방향과 다를 수 있음)
    pub rotation: [f32; 4],
    /// XZ평면상의 플레이어 이동 방향
    pub direction: [f32; 3],
    /// 클라이언트 컨트롤러 입력 상태 플래그
    pub input_flags: GameInputBits,
    /// 카메라 상태 (서버에서 클라이언트 값을 사용)
    pub view_state: ViewState,
    /// 카메라 상태 타이머 (서버에서 클라이언트 값을 사용)
    pub view_state_timer: ViewStateTimer,
    /// 카메라 방향 (서버에서 클라이언트 값을 사용)
    pub view_rotation: LatLon,
}

impl Packet for PushStatusPacket {
    fn packet_type() -> PacketType {
        PacketType::PushStatus
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size()
            + LoginToken::byte_size()
            + <[f32; 4]>::byte_size()
            + <[f32; 3]>::byte_size()
            + GameInputBits::byte_size()
            + ViewState::byte_size()
            + ViewStateTimer::byte_size()
            + LatLon::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.rotation.to_big_endian_bytes());
        data.extend_from_slice(&self.direction.to_big_endian_bytes());
        data.extend_from_slice(&self.input_flags.to_big_endian_bytes());
        data.extend_from_slice(&self.view_state.to_big_endian_bytes());
        data.extend_from_slice(&self.view_state_timer.to_big_endian_bytes());
        data.extend_from_slice(&self.view_rotation.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PushStatusPacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
    }

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

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 사용자 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        // 플레이어 캐릭터 방향을 가져옵니다.
        offset = offset + size;
        size = <[f32; 4]>::byte_size();
        data = &bytes[offset..offset + size];
        let rotation = <[f32; 4]>::from_big_endian_bytes(data);

        // 플레이어 이동 방향을 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let direction = <[f32; 3]>::from_big_endian_bytes(data);

        // 클라이언트 컨트롤러 입력 상태를 가져옵니다.
        offset = offset + size;
        size = GameInputBits::byte_size();
        data = &bytes[offset..offset + size];
        let input_flags = GameInputBits::from_big_endian_bytes(data);

        // 카메라 상태를 가져옵니다.
        offset = offset + size;
        size = ViewState::byte_size();
        data = &bytes[offset..offset + size];
        let view_state = ViewState::try_from_big_endian_bytes(data)?;

        // 카메라 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = ViewStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let view_state_timer = ViewStateTimer::from_big_endian_bytes(data);

        // 카메라 방향을 가져옵니다.
        offset = offset + size;
        size = LatLon::byte_size();
        data = &bytes[offset..offset + size];
        let view_rotation = LatLon::from_big_endian_bytes(data);

        Some(Self {
            user_id,
            token,
            rotation,
            direction,
            input_flags,
            view_state,
            view_state_timer,
            view_rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_status_packet() {
        let origin = PushStatusPacket {
            user_id: UserId::new(1515161),
            token: LoginToken::new(909515132),
            rotation: [0.15132516, 0.1234165125, 1.251651, 0.15151],
            direction: [0.1512515, 1.241561, 0.1451351],
            input_flags: GameInputBits::Left | GameInputBits::Forward | GameInputBits::Jump,
            view_state: ViewState::Idle,
            view_state_timer: ViewStateTimer(2.134151),
            view_rotation: LatLon {
                lat: 1.1512512,
                lon: 2.1516165,
            },
        };
        let raw_packet = origin.as_raw();
        let other = PushStatusPacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
