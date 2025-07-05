//! 이벤트성 입력 전송 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, InputKind, LoginToken, TryFromBigEndian, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 비트 필드 데이터
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const INPUT_BIT_MASK: u8 = 0x0F;
    const INPUT_SHIFT: usize = 0;
    const PRESS_BIT_MASK: u8 = 0x01;
    const PRESS_SHIFT: usize = 4;

    /// 새로운 비트 필드 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(0x00)
    }

    /// 입력 종류를 반환합니다.
    pub fn input_kind(self) -> Option<InputKind> {
        InputKind::new((self.0 >> Self::INPUT_SHIFT) & Self::INPUT_BIT_MASK)
    }

    /// 입력 종류를 설정합니다.
    pub const fn with_input_kind(mut self, input: InputKind) -> Self {
        self.0 &= !(Self::INPUT_BIT_MASK << Self::INPUT_SHIFT);
        self.0 |= ((input as u8) & Self::INPUT_BIT_MASK) << Self::INPUT_SHIFT;
        self
    }

    /// 눌림 떼임 여부를 반환합니다.
    pub fn is_pressed(self) -> bool {
        (self.0 >> Self::PRESS_SHIFT) & Self::PRESS_BIT_MASK == Self::PRESS_BIT_MASK
    }

    /// 눌림 떼임 여부를 설정합니다.
    pub const fn with_pressed(mut self, pressed: bool) -> Self {
        self.0 &= !(Self::PRESS_BIT_MASK << Self::PRESS_SHIFT);
        self.0 |= ((pressed as u8) & Self::PRESS_BIT_MASK) << Self::PRESS_SHIFT;
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

/// 입력 이벤트 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputEvent {
    /// 클라이언트의 플레이 경과 시간
    pub play_elapsed_time_ms: u32,
    /// 입력 이벤트 종류
    pub input: InputKind,
    /// 눌림 여부
    pub pressed: bool,
}

impl InputEvent {
    /// 새로운 `InputEvent`를 생성합니다.
    pub const fn new(play_elapsed_time_ms: u32, input: InputKind, pressed: bool) -> Self {
        Self {
            play_elapsed_time_ms,
            input,
            pressed,
        }
    }
}

impl BigEndian for InputEvent {
    fn byte_size() -> usize {
        u32::byte_size() + Bitfield::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 데이터를 생성합니다.
        let bitfield = Bitfield::new()
            .with_input_kind(self.input)
            .with_pressed(self.pressed);

        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.play_elapsed_time_ms.to_big_endian_bytes());
        bytes.extend_from_slice(&bitfield.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InputEvent)
            );
        }

        bytes
    }
}

impl TryFromBigEndian for InputEvent {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InputEvent)
            )
        };

        // 클라이언트 플레이 경과 시간을 가져옵니다.
        let mut offset = 0;
        let mut size = u32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let play_elapsed_time_ms = u32::from_big_endian_bytes(data);

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        Some(Self {
            play_elapsed_time_ms,
            input: bitfield.input_kind()?,
            pressed: bitfield.is_pressed(),
        })
    }
}

/// 한 프레임에서 최대 입력의 수입니다.
pub const MAX_INPUT_EVENTS: usize = u16::MAX as usize;

/// 인게임 장면에서 클라이언트에서 서버로 입력 이벤트를 보내는 패킷입니다.
/// 한 프레임에서 발생하는 모든 입력 이벤트를 전송합니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGameInputEventPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 이벤트 목록입니다.
    pub events: Vec<InputEvent>,
}

impl InGameInputEventPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `events`가 비어있거나 `MAX_INPUT_EVENTS`보다 많은 경우 [`panic`]을 호출합니다.
    ///
    pub const fn new(uid: UserId, token: LoginToken, events: Vec<InputEvent>) -> Self {
        assert!(!events.is_empty(), "the given events is empty!");
        assert!(events.len() <= MAX_INPUT_EVENTS, "too many events!");
        Self { uid, token, events }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `events`가 비어있거나 `MAX_INPUT_EVENTS`보다 많은 경우 [`panic`]을 호출합니다.
    ///
    pub fn from_iter<I>(uid: UserId, token: LoginToken, iter: I) -> Self
    where
        I: IntoIterator<Item = InputEvent>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(uid, token, iter.into_iter().collect())
    }
}

impl Packet for InGameInputEventPacket {
    fn packet_type() -> PacketType {
        PacketType::InGameInputEvent
    }

    fn as_raw(&self) -> RawPacket {
        let num_events = self.events.len();
        assert!(
            0 < num_events && num_events <= MAX_INPUT_EVENTS,
            "invalid data!"
        );

        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size()
            + LoginToken::byte_size()
            + u8::byte_size()
            + InputEvent::byte_size() * num_events;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&(num_events as u8).to_big_endian_bytes());
        for event in self.events.iter() {
            data.extend_from_slice(&event.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameInputEventPacket)
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

        // 이벤트 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_events = u8::from_big_endian_bytes(data) as usize;
        if num_events == 0 || num_events > MAX_INPUT_EVENTS {
            return None;
        }

        // 이벤트 데이터를 가져옵니다.
        let mut events = Vec::with_capacity(num_events);
        for _ in 0..num_events {
            offset = offset + size;
            size = InputEvent::byte_size();
            data = &bytes[offset..offset + size];
            events.push(InputEvent::try_from_big_endian_bytes(data)?);
        }

        Some(Self { uid, token, events })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_game_input_event_packet() {
        let origin = InGameInputEventPacket::from_iter(
            UserId::new(54131),
            LoginToken::new(85312451324),
            [
                InputEvent::new(1210, InputKind::Reload, false),
                InputEvent::new(1290, InputKind::Attack, true),
                InputEvent::new(1310, InputKind::Jump, false),
                InputEvent::new(1330, InputKind::Attack, true),
                InputEvent::new(1401, InputKind::Jump, false),
                InputEvent::new(1420, InputKind::Skill, true),
            ],
        );
        let raw = origin.as_raw();
        let other = InGameInputEventPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
