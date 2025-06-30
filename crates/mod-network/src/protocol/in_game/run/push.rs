//! 인게임 장면을 검증 하는 패킷과 관련된 코드를 관리합니다.
//!

use std::cmp;

use crate::{
    components::{
        ActionState, BigEndian, LatLon, LoginToken, MovementState, PlayerStateData, UserId,
        ViewState,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 최대 상태 기록의 수
pub const MAX_HISTORIES: usize = 100;

/// 플레이어 상태 기록 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateHistory {
    /// 현재 시대
    pub epoch: u64,
    /// 현재 시대에서 경과한 시간
    pub elapsed_time_ms: u16,
    /// 플레이어 상태 데이터
    states: PlayerStateData,
}

impl StateHistory {
    /// 새로운 상태 기록을 생성합니다.
    pub const fn new(
        epoch: u64,
        elapsed_time_ms: u16,
        action_state: ActionState,
        movement_state: MovementState,
        view_state: ViewState,
    ) -> Self {
        Self {
            epoch,
            elapsed_time_ms,
            states: PlayerStateData::new()
                .with_action_state(action_state)
                .with_movement_state(movement_state)
                .with_view_state(view_state),
        }
    }

    /// 행동 상태를 반환합니다.
    pub fn action_state(&self) -> ActionState {
        self.states.action_state()
    }

    /// 움직임 상태를 반환합니다.
    pub fn movement_state(&self) -> MovementState {
        self.states.movement_state()
    }

    /// 시야 상태를 반환합니다.
    pub fn view_state(&self) -> ViewState {
        self.states.view_state()
    }
}

impl BigEndian for StateHistory {
    fn byte_size() -> usize {
        u64::byte_size() + u16::byte_size() + PlayerStateData::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(StateHistory)
            )
        };

        // 현재 시대를 가져옵니다.
        let mut offset = 0;
        let mut size = u64::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = u64::from_big_endian_bytes(data);

        // 경과 시간을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let elapsed_time_ms = u16::from_big_endian_bytes(data);

        // 플레이어 상태 데이터를 가져옵니다.
        offset = offset + size;
        size = PlayerStateData::byte_size();
        data = &bytes[offset..offset + size];
        let states = PlayerStateData::from_big_endian_bytes(data);

        Self {
            epoch,
            elapsed_time_ms,
            states,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.epoch.to_big_endian_bytes());
        bytes.extend_from_slice(&self.elapsed_time_ms.to_big_endian_bytes());
        bytes.extend_from_slice(&self.states.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(StateHistory)
            );
        }

        bytes
    }
}

impl Ord for StateHistory {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.epoch
            .cmp(&other.epoch)
            .then(self.elapsed_time_ms.cmp(&other.elapsed_time_ms))
    }
}

impl PartialOrd<Self> for StateHistory {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.epoch.partial_cmp(&other.epoch)
    }
}

/// 클라이언트에서 서버로 보내는 인게임 장면 갱신 패킷입니다.
/// 위치, 회전 정보를 갱신합니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGamePushNotifyPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 현재 시대
    pub epoch: u64,
    /// 현재 시대에서 경과한 시간
    pub elapsed_time_ms: u16,
    /// 상태 기록
    pub histories: Vec<StateHistory>,
    /// 월드 공간 위치.
    pub translation: [f32; 3],
    /// 카메라 회전 각도
    pub latlon: LatLon,
}

impl InGamePushNotifyPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new<T>(
        uid: UserId,
        token: LoginToken,
        epoch: u64,
        elapsed_time_ms: u16,
        histories: Vec<StateHistory>,
        translation: T,
        latlon: LatLon,
    ) -> Self
    where
        T: Into<[f32; 3]>,
    {
        Self {
            uid,
            token,
            epoch,
            elapsed_time_ms,
            histories,
            translation: translation.into(),
            latlon,
        }
    }

    /// 새로운 패킷을 생성합니다.
    pub fn from_iter<T, I>(
        uid: UserId,
        token: LoginToken,
        epoch: u64,
        elapsed_time_ms: u16,
        iter: I,
        translation: T,
        latlon: LatLon,
    ) -> Self
    where
        T: Into<[f32; 3]>,
        I: IntoIterator<Item = StateHistory>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(
            uid,
            token,
            epoch,
            elapsed_time_ms,
            iter.into_iter().collect(),
            translation,
            latlon,
        )
    }
}

impl Packet for InGamePushNotifyPacket {
    fn packet_type() -> PacketType {
        PacketType::InGamePushNotify
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_histories = self.histories.len();
        let data_size = UserId::byte_size()
            + LoginToken::byte_size()
            + u64::byte_size()
            + u16::byte_size()
            + u8::byte_size()
            + StateHistory::byte_size() * num_histories
            + <[f32; 3]>::byte_size()
            + LatLon::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&self.elapsed_time_ms.to_big_endian_bytes());
        data.extend_from_slice(&(num_histories as u8).to_big_endian_bytes());
        for history in self.histories.iter() {
            data.extend_from_slice(&history.to_big_endian_bytes());
        }
        data.extend_from_slice(&self.translation.to_big_endian_bytes());
        data.extend_from_slice(&self.latlon.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePushNotifyPacket)
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

        // 현재 시대를 가져옵니다.
        offset = offset + size;
        size = u64::byte_size();
        data = &bytes[offset..offset + size];
        let epoch = u64::from_big_endian_bytes(data);

        // 현재 시대로부터 경과한 시간을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let elapsed_time_ms = u16::from_big_endian_bytes(data);

        // 기록의 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_histories = u8::from_big_endian_bytes(data) as usize;

        // 상태 데이터를 가져옵니다.
        let mut histories = Vec::with_capacity(num_histories);
        for _ in 0..num_histories {
            offset = offset + size;
            size = StateHistory::byte_size();
            data = &bytes[offset..offset + size];
            histories.push(StateHistory::from_big_endian_bytes(data));
        }

        // 월드 공간 위치를 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let translation = <[f32; 3]>::from_big_endian_bytes(data);

        // 카메라 회전 각도를 가져옵니다.
        offset = offset + size;
        size = LatLon::byte_size();
        data = &bytes[offset..offset + size];
        let latlon = LatLon::from_big_endian_bytes(data);

        Some(Self {
            uid,
            token,
            epoch,
            elapsed_time_ms,
            histories,
            translation,
            latlon,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_history() {
        let origin = StateHistory::new(
            1,
            123,
            ActionState::Aiming,
            MovementState::Moving,
            ViewState::Aiming,
        );
        let bytes = origin.to_big_endian_bytes();
        let other = StateHistory::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }

    #[test]
    fn test_in_game_push_notify_packet() {
        let origin = InGamePushNotifyPacket::new(
            UserId::new(175462),
            LoginToken::new(8641451),
            31841,
            232,
            vec![],
            glam::vec3a(0.0341341, 1.431413, -10.431412),
            LatLon::new(34f32.to_radians(), 120f32.to_radians()),
        );
        let raw = origin.as_raw();
        let other = InGamePushNotifyPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);

        let origin = InGamePushNotifyPacket::new(
            UserId::new(175462),
            LoginToken::new(8641451),
            31841,
            232,
            vec![
                StateHistory::new(
                    31841,
                    53,
                    ActionState::AimOff,
                    MovementState::Moving,
                    ViewState::ZoomOut,
                ),
                StateHistory::new(
                    31841,
                    153,
                    ActionState::Idle,
                    MovementState::MoveToEnd,
                    ViewState::Idle,
                ),
            ],
            glam::vec3a(0.0341341, 1.431413, -10.431412),
            LatLon::new(34f32.to_radians(), 120f32.to_radians()),
        );
        let raw = origin.as_raw();
        let other = InGamePushNotifyPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
