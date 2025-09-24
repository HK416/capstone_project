//! 이벤트성 입력 전송 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, InputEvent, InputSnapshot, LatLon, LoginToken, TryFromBigEndian, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 한 프레임에서 최대 입력 스냅샷의 수입니다.
pub const MAX_INPUT_SNAPSHOTS: usize = 255;

/// 인게임 장면에서 클라이언트에서 서버로 입력 이벤트를 보내는 패킷입니다.
/// 한 프레임에서 발생하는 모든 입력 이벤트를 전송합니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGameInputPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 클라이언트의 게임 경과 시간
    pub play_elapsed_time_ms: u32,
    /// 스냅샷 목록입니다.
    pub snapshots: Vec<InputSnapshot>,
}

impl InGameInputPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `snapshots`가 비어있거나 `MAX_INPUT_EVENTS`보다 많은 경우 [`panic`]을 호출합니다.
    ///
    pub const fn new(
        uid: UserId,
        token: LoginToken,
        play_elapsed_time_ms: u32,
        snapshots: Vec<InputSnapshot>,
    ) -> Self {
        assert!(!snapshots.is_empty(), "the given events is empty!");
        assert!(snapshots.len() <= MAX_INPUT_SNAPSHOTS, "too many events!");
        Self {
            uid,
            token,
            play_elapsed_time_ms,
            snapshots,
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `events`가 비어있거나 `MAX_INPUT_EVENTS`보다 많은 경우 [`panic`]을 호출합니다.
    ///
    pub fn from_iter<I>(uid: UserId, token: LoginToken, play_elapsed_time_ms: u32, iter: I) -> Self
    where
        I: IntoIterator<Item = InputSnapshot>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(uid, token, play_elapsed_time_ms, iter.into_iter().collect())
    }
}

const TIME_BIT_MASK: u32 = 0x7FFFFFFF;
const TIME_SHIFT: usize = 0;
const KIND_BIT_MASK: u32 = 0x01;
const KIND_SHIFT: usize = 31;

impl Packet for InGameInputPacket {
    fn packet_type() -> PacketType {
        PacketType::InGameInput
    }

    fn as_raw(&self) -> RawPacket {
        let num_events = self.snapshots.len();
        assert!(
            0 < num_events && num_events <= MAX_INPUT_SNAPSHOTS,
            "invalid data!"
        );

        // 데이터 크기를 계산합니다.
        let mut data_size =
            UserId::byte_size() + LoginToken::byte_size() + u32::byte_size() + u8::byte_size();
        for snapshot in self.snapshots.iter() {
            match snapshot {
                InputSnapshot::CameraOrientation { .. } => {
                    data_size += u32::byte_size();
                    data_size += f32::byte_size();
                    data_size += f32::byte_size();
                }
                InputSnapshot::KeyEvent { events, .. } => {
                    data_size += u32::byte_size();
                    data_size += u8::byte_size();
                    data_size += InputEvent::byte_size() * events.len();
                }
            }
        }

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.play_elapsed_time_ms.to_big_endian_bytes());
        data.extend_from_slice(&(num_events as u8).to_big_endian_bytes());
        for snapshot in self.snapshots.iter() {
            match snapshot {
                InputSnapshot::CameraOrientation {
                    play_elapsed_time_ms,
                    latlon,
                } => {
                    let kind = (0 & KIND_BIT_MASK) << KIND_SHIFT;
                    let time = (*play_elapsed_time_ms & TIME_BIT_MASK) << TIME_SHIFT;
                    data.extend_from_slice(&(kind | time).to_big_endian_bytes());
                    data.extend_from_slice(&latlon.to_big_endian_bytes());
                }
                InputSnapshot::KeyEvent {
                    play_elapsed_time_ms,
                    events,
                } => {
                    let kind = (1 & KIND_BIT_MASK) << KIND_SHIFT;
                    let time = (*play_elapsed_time_ms & TIME_BIT_MASK) << TIME_SHIFT;
                    data.extend_from_slice(&(kind | time).to_big_endian_bytes());

                    let num_events = events.len();
                    assert!(num_events != 0, "the given input event is empty!");
                    assert!(num_events <= MAX_INPUT_SNAPSHOTS, "too many input events!");
                    data.extend_from_slice(&(num_events as u8).to_big_endian_bytes());
                    for event in events {
                        data.extend_from_slice(&event.to_big_endian_bytes());
                    }
                }
            }
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

        // 클라이언트 게임 경과 시간을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let play_elapsed_time_ms = u32::from_big_endian_bytes(data);

        // 스냅샷의 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_snapshots = u8::from_big_endian_bytes(data) as usize;
        if num_snapshots == 0 || num_snapshots > MAX_INPUT_SNAPSHOTS {
            return None;
        }

        // 이벤트 데이터를 가져옵니다.
        let mut snapshots = Vec::with_capacity(num_snapshots);
        for _ in 0..num_snapshots {
            // 이벤트 종류와 시간 데이터를 가져옵니다.
            offset = offset + size;
            size = u32::byte_size();
            data = &bytes[offset..offset + size];
            let bits = u32::from_big_endian_bytes(data);

            let kind = (bits >> KIND_SHIFT) & KIND_BIT_MASK;
            let play_elapsed_time_ms = (bits >> TIME_SHIFT) & TIME_BIT_MASK;
            match kind {
                0 => {
                    // 카메라 방향을 가져옵니다.
                    offset = offset + size;
                    size = LatLon::byte_size();
                    data = &bytes[offset..offset + size];
                    let latlon = LatLon::from_big_endian_bytes(data);

                    snapshots.push(InputSnapshot::CameraOrientation {
                        play_elapsed_time_ms,
                        latlon,
                    })
                }
                1 => {
                    // 이벤트의 개수를 가져옵니다.
                    offset = offset + size;
                    size = u8::byte_size();
                    data = &bytes[offset..offset + size];
                    let num_events = u8::from_big_endian_bytes(data) as usize;
                    if num_events == 0 || num_events > MAX_INPUT_SNAPSHOTS {
                        return None;
                    }

                    // 이벤트를 가져옵니다.
                    let mut events = Vec::with_capacity(num_events);
                    for _ in 0..num_events {
                        offset = offset + size;
                        size = InputEvent::byte_size();
                        data = &bytes[offset..offset + size];
                        events.push(InputEvent::try_from_big_endian_bytes(data)?);
                    }

                    snapshots.push(InputSnapshot::KeyEvent {
                        play_elapsed_time_ms,
                        events,
                    })
                }
                _ => return None,
            }
        }

        Some(Self {
            uid,
            token,
            play_elapsed_time_ms,
            snapshots,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::InputKind;

    use super::*;

    #[test]
    fn test_in_game_input_event_packet() {
        let origin = InGameInputPacket::from_iter(
            UserId::new(54131),
            LoginToken::new(85312451324),
            465_910,
            [
                InputSnapshot::KeyEvent {
                    play_elapsed_time_ms: 462_321,
                    events: vec![
                        InputEvent::KeyRelease(InputKind::Left),
                        InputEvent::KeyPress(InputKind::Left),
                        InputEvent::KeyPress(InputKind::Attack),
                        InputEvent::KeyRelease(InputKind::Attack),
                        InputEvent::KeyPress(InputKind::Aiming),
                        InputEvent::KeyRelease(InputKind::Jump),
                        InputEvent::KeyPress(InputKind::Attack),
                        InputEvent::KeyPress(InputKind::Reload),
                        InputEvent::KeyRelease(InputKind::Jump),
                        InputEvent::KeyRelease(InputKind::Right),
                    ],
                },
                InputSnapshot::CameraOrientation {
                    play_elapsed_time_ms: 463_211,
                    latlon: LatLon {
                        lat: -13.12f32.to_radians(),
                        lon: 5.96f32.to_radians(),
                    },
                },
                InputSnapshot::KeyEvent {
                    play_elapsed_time_ms: 464_499,
                    events: vec![
                        InputEvent::KeyRelease(InputKind::Left),
                        InputEvent::KeyPress(InputKind::Left),
                        InputEvent::KeyPress(InputKind::Attack),
                        InputEvent::KeyRelease(InputKind::Attack),
                        InputEvent::KeyPress(InputKind::Aiming),
                        InputEvent::KeyRelease(InputKind::Jump),
                        InputEvent::KeyPress(InputKind::Attack),
                        InputEvent::KeyPress(InputKind::Reload),
                        InputEvent::KeyRelease(InputKind::Jump),
                        InputEvent::KeyRelease(InputKind::Right),
                    ],
                },
            ],
        );
        let raw = origin.as_raw();
        let other = InGameInputPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
