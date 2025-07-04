//! 인게임 장면을 검증 하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, GameInput, GameInputBits, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 최대 상태 기록의 수
pub const MAX_HISTORIES: usize = 100;

/// 기록 데이터의 비트 필드 데이터입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

#[allow(dead_code)]
impl Bitfield {
    const INPUT_BIT_MASK: u8 = 0x0F;
    const INPUT_SHIFT: usize = 0;
    const KIND_BIT_MASK: u8 = 0x03;
    const KIND_SHIFT: usize = 4;

    /// 새로운 비트 필드 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(0x00)
    }

    /// 게임 입력 키 종류를 가져옵니다.
    pub fn input(&self) -> Option<GameInput> {
        let val = (self.0 >> Self::INPUT_SHIFT) & Self::INPUT_BIT_MASK;
        GameInput::new(val)
    }

    /// 게임 입력 키 종류를 설정합니다.
    pub const fn set_input(&mut self, input: GameInput) {
        self.0 &= !(Self::INPUT_BIT_MASK << Self::INPUT_SHIFT);
        self.0 |= ((input as u8) & Self::INPUT_BIT_MASK) << Self::INPUT_SHIFT;
    }

    /// 게임 입력 키 종류를 설정합니다.
    pub const fn with_input(mut self, input: GameInput) -> Self {
        self.set_input(input);
        self
    }

    /// 기록 데이터 종류를 가져옵니다.
    pub fn kind(&self) -> HistoryKind {
        let val = (self.0 >> Self::KIND_SHIFT) & Self::KIND_BIT_MASK;
        HistoryKind::new(val).unwrap_or_default()
    }

    /// 기록 데이터 종류를 설정합니다.
    pub const fn set_kind(&mut self, kind: HistoryKind) {
        self.0 &= !(Self::KIND_BIT_MASK << Self::KIND_SHIFT);
        self.0 |= ((kind as u8) & Self::KIND_BIT_MASK) << Self::KIND_SHIFT;
    }

    /// 기록 데이터 종류를 설정합니다.
    pub const fn with_kind(mut self, kind: HistoryKind) -> Self {
        self.set_kind(kind);
        self
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

/// 기록 종류 데이터입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HistoryKind {
    #[default]
    ViewControl = 0,
    KeyRelease = 1,
    KeyPress = 2,
}

impl HistoryKind {
    /// 새로운 기록 종류를 생성합니다.
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::ViewControl),
            1 => Some(Self::KeyRelease),
            2 => Some(Self::KeyPress),
            _ => None,
        }
    }
}

/// 기록 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum HistoryData {
    KeyPress(GameInput),
    KeyRelease(GameInput),
    ViewControl { lat: f32, lon: f32 },
}

impl BigEndian for HistoryData {
    fn byte_size() -> usize {
        Bitfield::byte_size() + f32::byte_size() + f32::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(HistoryData)
            )
        };

        // 비트 필드 데이터를 가져옵니다.
        let mut offset = 0;
        let mut size = Bitfield::byte_size();
        let mut data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        // 움직인 위도 각도를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let lat = f32::from_big_endian_bytes(data);

        // 움직인 경도 각도를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let lon = f32::from_big_endian_bytes(data);

        match bitfield.kind() {
            HistoryKind::ViewControl => HistoryData::ViewControl { lat, lon },
            HistoryKind::KeyRelease => match bitfield.input() {
                Some(input) => HistoryData::KeyRelease(input),
                None => HistoryData::ViewControl { lat, lon },
            },
            HistoryKind::KeyPress => match bitfield.input() {
                Some(input) => HistoryData::KeyPress(input),
                None => HistoryData::ViewControl { lat, lon },
            },
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 데이터를 생성합니다.
        let mut bitfiled = Bitfield::new();
        let mut delta_lat = 0.0f32;
        let mut delta_lon = 0.0f32;
        match *self {
            HistoryData::KeyPress(input) => {
                bitfiled.set_kind(HistoryKind::KeyPress);
                bitfiled.set_input(input);
            }
            HistoryData::KeyRelease(input) => {
                bitfiled.set_kind(HistoryKind::KeyRelease);
                bitfiled.set_input(input);
            }
            HistoryData::ViewControl { lat, lon } => {
                bitfiled.set_kind(HistoryKind::ViewControl);
                delta_lat = lat;
                delta_lon = lon;
            }
        }

        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&bitfiled.to_big_endian_bytes());
        bytes.extend_from_slice(&delta_lat.to_big_endian_bytes());
        bytes.extend_from_slice(&delta_lon.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(HistoryData)
            );
        }

        bytes
    }
}

/// 플레이어 상태 기록 데이터입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct StateHistory {
    /// 현재 시대
    pub epoch: u32,
    /// 현재 시대에서 경과한 시간
    pub elapsed_time_ms: u16,
    /// 기록 데이터
    pub data: HistoryData,
}

impl StateHistory {
    /// 새로운 상태 기록을 생성합니다.
    pub const fn new(epoch: u32, elapsed_time_ms: u16, data: HistoryData) -> Self {
        Self {
            epoch,
            elapsed_time_ms,
            data,
        }
    }
}

impl BigEndian for StateHistory {
    fn byte_size() -> usize {
        u32::byte_size() + u16::byte_size() + HistoryData::byte_size()
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
        let mut size = u32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = u32::from_big_endian_bytes(data);

        // 경과 시간을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let elapsed_time_ms = u16::from_big_endian_bytes(data);

        // 데이터를 가져옵니다.
        offset = offset + size;
        size = HistoryData::byte_size();
        data = &bytes[offset..offset + size];
        let data = HistoryData::from_big_endian_bytes(data);

        Self {
            epoch,
            elapsed_time_ms,
            data,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.epoch.to_big_endian_bytes());
        bytes.extend_from_slice(&self.elapsed_time_ms.to_big_endian_bytes());
        bytes.extend_from_slice(&self.data.to_big_endian_bytes());

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

/// 클라이언트에서 서버로 보내는 인게임 장면 갱신 패킷입니다.
/// 위치, 회전 정보를 갱신합니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGamePushNotifyPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 현재 시대
    pub epoch: u32,
    /// 현재 시대에서 경과한 시간
    pub elapsed_time_ms: u16,
    /// 게임 입력 비트 플래그
    pub input_bits: GameInputBits,
    /// 상태 기록
    pub histories: Vec<StateHistory>,
}

impl InGamePushNotifyPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(
        uid: UserId,
        token: LoginToken,
        epoch: u32,
        elapsed_time_ms: u16,
        input_bits: GameInputBits,
        histories: Vec<StateHistory>,
    ) -> Self {
        Self {
            uid,
            token,
            epoch,
            elapsed_time_ms,
            input_bits,
            histories,
        }
    }

    /// 새로운 패킷을 생성합니다.
    pub fn from_iter<I>(
        uid: UserId,
        token: LoginToken,
        epoch: u32,
        elapsed_time_ms: u16,
        input_bits: GameInputBits,
        iter: I,
    ) -> Self
    where
        I: IntoIterator<Item = StateHistory>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(
            uid,
            token,
            epoch,
            elapsed_time_ms,
            input_bits,
            iter.into_iter().collect(),
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
            + u32::byte_size()
            + u16::byte_size()
            + GameInputBits::byte_size()
            + u8::byte_size()
            + StateHistory::byte_size() * num_histories;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&self.elapsed_time_ms.to_big_endian_bytes());
        data.extend_from_slice(&self.input_bits.to_big_endian_bytes());
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
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let epoch = u32::from_big_endian_bytes(data);

        // 현재 시대로부터 경과한 시간을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let elapsed_time_ms = u16::from_big_endian_bytes(data);

        // 입력 비트 플래그 데이터를 가져옵니다.
        offset = offset + size;
        size = GameInputBits::byte_size();
        data = &bytes[offset..offset + size];
        let input_bits = GameInputBits::from_big_endian_bytes(data);

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

        Some(Self {
            uid,
            token,
            epoch,
            elapsed_time_ms,
            input_bits,
            histories,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_history() {
        let origin = StateHistory::new(1, 123, HistoryData::KeyPress(GameInput::Attack));
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
            GameInputBits::empty(),
            vec![],
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
            GameInputBits::Left,
            vec![
                StateHistory::new(31841, 12, HistoryData::KeyPress(GameInput::Left)),
                StateHistory::new(31841, 122, HistoryData::KeyRelease(GameInput::Forward)),
                StateHistory::new(
                    31841,
                    219,
                    HistoryData::ViewControl {
                        lat: 3f32.to_radians(),
                        lon: -1f32.to_radians(),
                    },
                ),
            ],
        );
        let raw = origin.as_raw();
        let other = InGamePushNotifyPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
