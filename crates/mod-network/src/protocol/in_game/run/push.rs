//! 인게임 장면을 갱신 하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, GameInputBits, PlayerStateData},
    protocol::{Packet, PacketType, RawPacket},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerHistory {
    /// 경과 시간
    pub elapsed_time_ms: u16,
    /// 플레이어 상태 데이터
    pub player_state: PlayerStateData,
    /// 플레이어 입력 플래그
    pub input_flags: GameInputBits,
}

impl PlayerHistory {
    /// 새로운 플레이어 데이터를 생성합니다.
    pub const fn new(
        elapsed_time_ms: u16,
        player_state: PlayerStateData,
        input_flags: GameInputBits,
    ) -> Self {
        Self {
            elapsed_time_ms,
            player_state,
            input_flags,
        }
    }
}

impl BigEndian for PlayerHistory {
    fn byte_size() -> usize {
        u16::byte_size() // 2byte
        + PlayerStateData::byte_size() // 3byte
        + GameInputBits::byte_size() // 5byte
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PlayerHistory)
            )
        };

        // 경과 시간을 가져옵니다.
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let elapsed_time_ms = u16::from_big_endian_bytes(data);

        // 플레이어 상태를 가져옵니다.
        offset = offset + size;
        size = PlayerStateData::byte_size();
        data = &bytes[offset..offset + size];
        let player_state = PlayerStateData::from_big_endian_bytes(data);

        // 플레이어 입력 플래그를 가져옵니다.
        offset = offset + size;
        size = GameInputBits::byte_size();
        data = &bytes[offset..offset + size];
        let input_flags = GameInputBits::from_big_endian_bytes(data);

        Self {
            elapsed_time_ms,
            player_state,
            input_flags,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.elapsed_time_ms.to_big_endian_bytes());
        bytes.extend_from_slice(&self.player_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.input_flags.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PlayerHistory)
            );
        }

        bytes
    }
}

/// 최대 플레이어 데이터의 수
pub const MAX_HISTORYIES: usize = 100;

/// 클라이언트에서 서버로 보내는 플레이어 데이터 갱신 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGamePushPacket {
    /// 현재 시대
    pub epoch: u64,
    /// 플레이어 데이터
    pub histories: Vec<PlayerHistory>,
}

impl InGamePushPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// - 주어진 `history`가 없거나, `MAX_HISTORYIES`보다 클 큰 경우 [`panic!`]을 호출합니다.
    /// - 주어진 `history`가 시간순으로 정렬되지 않은 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(epoch: u64, histories: Vec<PlayerHistory>) -> Self {
        assert!(
            histories.is_sorted_by_key(|h| h.elapsed_time_ms),
            "the given data must be sorted chronologically"
        );
        assert!(!histories.is_empty(), "the given history data is empty!");
        assert!(histories.len() <= MAX_HISTORYIES, "too many historys!");
        Self { epoch, histories }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `history`가 없거나, `MAX_HISTORYIES`보다 클 큰 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(epoch: u64, iter: I) -> Self
    where
        I: IntoIterator<Item = PlayerHistory>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(epoch, iter.into_iter().collect())
    }
}

impl Packet for InGamePushPacket {
    fn packet_type() -> PacketType {
        PacketType::InGamePush
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_histories = self.histories.len();
        let data_size = u64::byte_size() // 8byte
            + u8::byte_size() // 9byte
            + PlayerHistory::byte_size() * num_histories; // max: 509byte
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&(num_histories as u8).to_big_endian_bytes());
        for history in self.histories.iter() {
            data.extend_from_slice(&history.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePushPacket)
            );
        }

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

        // 현재 시대를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u64::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = u64::from_big_endian_bytes(data);

        // 기록의 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_histories = u8::from_big_endian_bytes(data) as usize;
        if num_histories == 0 || num_histories > MAX_HISTORYIES {
            return None;
        }

        // 기록을 가져옵니다.
        let mut histories = Vec::with_capacity(num_histories);
        for _ in 0..num_histories {
            offset = offset + size;
            size = PlayerHistory::byte_size();
            data = &bytes[offset..offset + size];
            histories.push(PlayerHistory::from_big_endian_bytes(data));
        }

        histories
            .is_sorted_by_key(|h| h.elapsed_time_ms)
            .then_some(Self { epoch, histories })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{ActionState, MovementState, ViewState};

    use super::*;

    #[test]
    fn test_player_history() {
        let origin = PlayerHistory::new(
            123,
            PlayerStateData::new()
                .with_action_state(ActionState::AimOff)
                .with_movement_state(MovementState::InPlaceJumping)
                .with_view_state(ViewState::Idle),
            GameInputBits::Forward | GameInputBits::Left | GameInputBits::Jump,
        );
        let bytes = origin.to_big_endian_bytes();
        let other = PlayerHistory::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }

    #[test]
    #[should_panic]
    fn test_creation_in_game_push_packet() {
        let history_0 = PlayerHistory::new(
            142,
            PlayerStateData::default(),
            GameInputBits::Forward | GameInputBits::Left | GameInputBits::Jump,
        );
        let history_1 = PlayerHistory::new(
            102,
            PlayerStateData::default(),
            GameInputBits::Forward | GameInputBits::Left,
        );
        let histories = vec![history_0, history_1];
        InGamePushPacket::new(53145, histories);
    }

    #[test]
    fn test_in_game_push_packet() {
        let history_0 = PlayerHistory::new(
            123,
            PlayerStateData::new()
                .with_action_state(ActionState::AimOff)
                .with_movement_state(MovementState::InPlaceJumping)
                .with_view_state(ViewState::Idle),
            GameInputBits::Forward | GameInputBits::Left | GameInputBits::Jump,
        );
        let history_1 = PlayerHistory::new(
            156,
            PlayerStateData::new()
                .with_action_state(ActionState::Idle)
                .with_movement_state(MovementState::InPlaceJumping)
                .with_view_state(ViewState::Idle),
            GameInputBits::Forward | GameInputBits::Left | GameInputBits::Skill,
        );
        let histories = vec![history_0, history_1];
        let origin = InGamePushPacket::new(151341, histories);
        let raw = origin.as_raw();
        let other = InGamePushPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
