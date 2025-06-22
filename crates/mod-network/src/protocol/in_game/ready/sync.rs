//! 인게임 준비 장면을 동기화 하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, LoginToken, NetworkState, UserId, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 사용자 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - connected     | 1bit | 서버 연결 여부
/// - network_state | 2bit | 네트워크 상태
/// - ready_to_play | 1bit | 준비 완료 여부
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 0;
    const STATE_BIT_MASK: u8 = 0x03;
    const STATE_SHIFT: usize = 1;
    const READY_BIT_MASK: u8 = 0x01;
    const READY_SHIFT: usize = 3;

    /// 새로운 비트 필드 데이터를 생성합니다.
    const fn new() -> Self {
        Self(0x00)
    }

    /// 서버 연결 여부를 설정합니다.
    const fn with_connected(mut self, connected: bool) -> Self {
        self.0 &= !(Self::CONNECT_BIT_MASK << Self::CONNECT_SHIFT);
        self.0 |= ((connected as u8) & Self::CONNECT_BIT_MASK) << Self::CONNECT_SHIFT;
        self
    }

    /// 서버 연결 여부를 반환합니다.
    fn is_connected(&self) -> bool {
        (self.0 >> Self::CONNECT_SHIFT) & Self::CONNECT_BIT_MASK == Self::CONNECT_BIT_MASK
    }

    /// 네트워크 상태를 설정합니다.
    const fn with_network_state(mut self, state: NetworkState) -> Self {
        self.0 &= !(Self::STATE_BIT_MASK << Self::STATE_SHIFT);
        self.0 |= ((state as u8) & Self::STATE_BIT_MASK) << Self::STATE_SHIFT;
        self
    }

    /// 네트워크 상태를 반환합니다.
    fn network_state(&self) -> NetworkState {
        let val = (self.0 >> Self::STATE_SHIFT) & Self::STATE_BIT_MASK;
        // Safety: 주어진 정수는 범위를 벗어나지 않음
        unsafe { NetworkState::new(val).unwrap_unchecked() }
    }

    /// 준비 여부를 설정합니다.
    const fn with_ready_to_play(mut self, ready: bool) -> Self {
        self.0 &= !(Self::READY_BIT_MASK << Self::READY_SHIFT);
        self.0 |= ((ready as u8) & Self::READY_BIT_MASK) << Self::READY_SHIFT;
        self
    }

    /// 준비 여부를 반환합니다.
    fn is_ready_to_play(&self) -> bool {
        (self.0 >> Self::READY_SHIFT) & Self::READY_BIT_MASK == Self::READY_BIT_MASK
    }
}

impl BigEndian for Bitfield {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u8::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

/// 플레이어 준비 상태 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerReadyStatus {
    /// 사용자 식별자
    pub uid: UserId,
    /// 비트 필드 데이터
    bitfield: Bitfield,
}

impl PlayerReadyStatus {
    /// 새로운 플레이어 준비 상태 데이터를 생성합니다.
    pub const fn new(
        uid: UserId,
        connected: bool,
        network_state: NetworkState,
        ready_to_play: bool,
    ) -> Self {
        Self {
            uid,
            bitfield: Bitfield::new()
                .with_connected(connected)
                .with_network_state(network_state)
                .with_ready_to_play(ready_to_play),
        }
    }

    /// 서버 연결 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        self.bitfield.is_connected()
    }

    /// 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        self.bitfield.network_state()
    }

    /// 준비 여부를 반환합니다.
    pub fn is_ready_to_play(&self) -> bool {
        self.bitfield.is_ready_to_play()
    }
}

impl BigEndian for PlayerReadyStatus {
    fn byte_size() -> usize {
        UserId::byte_size() + Bitfield::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PlayerReadyStatus)
            )
        };

        // 사용자 식별자 데이터를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        Self { uid, bitfield }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PlayerReadyStatus)
            );
        }

        bytes
    }
}

/// 서버에서 클라이언트로 보내는 각 플레이어의 인게임 준비 상태 데이터 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGameReadyStatusPacket {
    /// 준비 완료 까지 남은 시간
    pub remaining_time_sec: f32,
    /// 플레이어 준비 상태 데이터
    pub players: Vec<PlayerReadyStatus>,
}

impl InGameReadyStatusPacket {
    /// 새로운 패킷을 생성합니다
    ///
    /// # Panics
    /// 주어진 `player`가 비어있거나 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(remaining_time_sec: f32, players: Vec<PlayerReadyStatus>) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");
        Self {
            remaining_time_sec,
            players,
        }
    }

    /// 새로운 패킷을 생성합니다
    ///
    /// # Panics
    /// 주어진 `player`가 비어있거나 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(remaining_time_sec: f32, iter: I) -> Self
    where
        I: IntoIterator<Item = PlayerReadyStatus>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(remaining_time_sec, iter.into_iter().collect())
    }
}

impl Packet for InGameReadyStatusPacket {
    fn packet_type() -> PacketType {
        PacketType::InGameReadyStatus
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_players = self.players.len();
        let data_size =
            f32::byte_size() + u8::byte_size() + PlayerReadyStatus::byte_size() * num_players;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.remaining_time_sec.to_big_endian_bytes());
        data.extend_from_slice(&(num_players as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameReadyStatusPacket)
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

        // 남은 시간을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = f32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let remaining_time_sec = f32::from_big_endian_bytes(data);

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players <= 0 || num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        // 플레이어 데이터를 가져옵니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for _ in 0..num_players {
            offset = offset + size;
            size = PlayerReadyStatus::byte_size();
            data = &bytes[offset..offset + size];
            players.push(PlayerReadyStatus::from_big_endian_bytes(data));
        }

        Some(Self {
            remaining_time_sec,
            players,
        })
    }
}

/// 클라이언트에서 서버로 보내는 인게임 준비 완료 알림 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGameReadyNotifyPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
}

impl InGameReadyNotifyPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(uid: UserId, token: LoginToken) -> Self {
        Self { uid, token }
    }
}

impl Packet for InGameReadyNotifyPacket {
    fn packet_type() -> PacketType {
        PacketType::InGameReadyNotify
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size() + LoginToken::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameReadyNotifyPacket)
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

        // 사용자 계정 식별자를 가져옵니다.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitfield_connected() {
        let bitfield = Bitfield::new().with_connected(false);
        assert_eq!(false, bitfield.is_connected());

        let bitfield = Bitfield::new().with_connected(true);
        assert_eq!(true, bitfield.is_connected());
    }

    #[test]
    fn test_bitfield_network_state() {
        let state = NetworkState::Critical;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(state, bitfield.network_state());

        let state = NetworkState::Fair;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(state, bitfield.network_state());

        let state = NetworkState::Poor;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(state, bitfield.network_state());

        let state = NetworkState::Good;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(state, bitfield.network_state());
    }

    #[test]
    fn test_bitfield_ready_to_play() {
        let bitfield = Bitfield::new().with_ready_to_play(false);
        assert_eq!(false, bitfield.is_ready_to_play());

        let bitfield = Bitfield::new().with_ready_to_play(true);
        assert_eq!(true, bitfield.is_ready_to_play());
    }

    #[test]
    fn test_in_game_ready_status_packet() {
        let player_0 = PlayerReadyStatus::new(UserId::new(153141), true, NetworkState::Good, true);
        let player_1 =
            PlayerReadyStatus::new(UserId::new(4511341), true, NetworkState::Good, false);
        let player_2 =
            PlayerReadyStatus::new(UserId::new(2153141), true, NetworkState::Fair, false);
        let player_3 =
            PlayerReadyStatus::new(UserId::new(9153141), false, NetworkState::Critical, true);

        let players = vec![player_0, player_1, player_2, player_3];
        let origin = InGameReadyStatusPacket::new(43.013413, players);
        let raw = origin.as_raw();
        let other = InGameReadyStatusPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }

    #[test]
    fn test_in_game_ready_notify_packet() {
        let origin = InGameReadyNotifyPacket::new(UserId::new(12513241), LoginToken::new(9314351));
        let raw = origin.as_raw();
        let other = InGameReadyNotifyPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
