use crate::components::{
    ActionStateTimer, BigEndian, CompressedState, Epoch, LatLon, LoginToken, MovementStateTimer,
    ViewStateTimer,
};

use super::{Packet, PacketType, RawPacket};

/// 클라이언트에서 서버로 보내는
/// 플레이어 정보를 갱신하기 위한 패킷
#[derive(Debug, Clone, PartialEq)]
pub struct PushStatusPacket {
    /// 클라이언트가 이전에 받은 서버의 시대
    pub epoch: Epoch,
    /// 사용자 로그인 토큰
    pub token: LoginToken,
    /// 플레이어 캐릭터가 바라보는 방향 (이동 방향과 다를 수 있음)
    pub rotation: [f32; 4],
    /// XZ평면상의 플레이어 이동 방향
    pub direction: [f32; 3],
    /// 압축된 플레이어 상태 데이터
    pub compressed_state: CompressedState,
    /// 플레이어 행동 상태 타이머 (서버에서 행동 상태를 검증하는데 사용)
    pub action_state_timer: ActionStateTimer,
    /// 플레이어 움직임 상태 타이머 (서버에서 움직임 상태를 검증하는데 사용)
    pub movement_state_timer: MovementStateTimer,
    /// 카메라 상태 타이머 (서버에서 클라이언트 값을 사용)
    pub view_state_timer: ViewStateTimer,
    /// 카메라 방향 (서버에서 클라이언트 값을 사용)
    pub view_rotation: LatLon,
}

impl Default for PushStatusPacket {
    fn default() -> Self {
        Self {
            epoch: Epoch::default(),
            token: LoginToken::default(),
            rotation: [0.0, 0.0, 0.0, 1.0],
            direction: [0.0, 0.0, 1.0],
            compressed_state: CompressedState::default(),
            action_state_timer: ActionStateTimer::default(),
            movement_state_timer: MovementStateTimer::default(),
            view_state_timer: ViewStateTimer::default(),
            view_rotation: LatLon::default(),
        }
    }
}

impl Packet for PushStatusPacket {
    fn packet_type() -> PacketType {
        PacketType::PushStatus
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = Epoch::byte_size()
            + LoginToken::byte_size()
            + <[f32; 4]>::byte_size()
            + <[f32; 3]>::byte_size()
            + CompressedState::byte_size()
            + ActionStateTimer::byte_size()
            + MovementStateTimer::byte_size()
            + ViewStateTimer::byte_size()
            + LatLon::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.rotation.to_big_endian_bytes());
        data.extend_from_slice(&self.direction.to_big_endian_bytes());
        data.extend_from_slice(&self.compressed_state.to_big_endian_bytes());
        data.extend_from_slice(&self.action_state_timer.to_big_endian_bytes());
        data.extend_from_slice(&self.movement_state_timer.to_big_endian_bytes());
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

        // 서버의 시대를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = Epoch::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = Epoch::from_big_endian_bytes(data);

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

        // 플레이어 압축 상태 데이터를 가져옵니다.
        offset = offset + size;
        size = CompressedState::byte_size();
        data = &bytes[offset..offset + size];
        let compressed_state = CompressedState::from_big_endian_bytes(data);

        // 행동 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = ActionStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let action_state_timer = ActionStateTimer::from_big_endian_bytes(data);

        // 움직임 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = MovementStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let movement_state_timer = MovementStateTimer::from_big_endian_bytes(data);

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
            epoch,
            token,
            rotation,
            direction,
            compressed_state,
            action_state_timer,
            movement_state_timer,
            view_state_timer,
            view_rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_packet() {
        let origin = PushStatusPacket {
            ..Default::default()
        };
        let raw_packet = origin.as_raw();
        let other = PushStatusPacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
