use crate::{
    components::{BigEndian, Bullet, CapturePoint, PlayPhasePlayer, Team, TryFromBigEndian, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는
/// 월드 정보 갱신을 위한 패킷.
#[derive(Debug, Clone, PartialEq)]
pub struct PullStagePacket {
    pub players: Vec<PlayPhasePlayer>,
    pub bullets: Vec<Bullet>,
    pub capture_point: CapturePoint,
}

impl PullStagePacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`가 `MAX_IN_GAME_PLAYER`를 초과할 경우 `panic!`을 호출합니다.
    ///
    pub fn new(
        players: Vec<PlayPhasePlayer>, 
        bullets: Vec<Bullet>, 
        capture_point: CapturePoint
    ) -> Self {
        assert!(
            0 < players.len() && players.len() <= MAX_IN_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );

        Self { 
            players, 
            bullets,
            capture_point,
        }
    }
}

impl Packet for PullStagePacket {
    fn packet_type() -> PacketType {
        PacketType::PullStage
    }

    fn as_raw(&self) -> RawPacket {
        let mut data_size = u8::byte_size()
            + PlayPhasePlayer::byte_size() * self.players.len()
            + u16::byte_size()
            + Bullet::byte_size() * self.bullets.len()
            + CapturePoint::byte_size();
        if self.capture_point.capture_team.is_none() {
            data_size -= size_of::<Team>();
        }

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&(self.players.len() as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }
        data.extend_from_slice(&(self.bullets.len() as u16).to_big_endian_bytes());
        for bullet in self.bullets.iter() {
            data.extend_from_slice(&bullet.to_big_endian_bytes());
        }
        data.extend_from_slice(&self.capture_point.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PullStagePacket)
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

        // 플레이어 수를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u8::byte_size();
        let mut data = &bytes[offset..offset + size];
        let mut num_players = u8::from_big_endian_bytes(data);

        // 플레이어 데이터를 가져옵니다.
        let mut players = Vec::with_capacity(num_players as usize);
        while num_players > 0 {
            offset = offset + size;
            size = PlayPhasePlayer::byte_size();
            data = &bytes[offset..offset + size];
            players.push(PlayPhasePlayer::try_from_big_endian_bytes(data)?);
            num_players -= 1;
        }

        // 총알의 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let mut num_bullets = u16::from_big_endian_bytes(data);

        // 총알 데이터를 가져옵니다.
        let mut bullets = Vec::with_capacity(num_bullets as usize);
        while num_bullets > 0 {
            offset = offset + size;
            size = Bullet::byte_size();
            data = &bytes[offset..offset + size];
            bullets.push(Bullet::try_from_big_endian_bytes(data)?);
            num_bullets -= 1;
        }

        // 점령지 데이터를 가져옵니다.
        offset = offset + size;
        size = CapturePoint::byte_size();
        data = &bytes[offset..offset + size];
        let capture_point = CapturePoint::try_from_big_endian_bytes(data)?;

        Some(Self { players, bullets, capture_point })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use crate::components::{
        ActionState, ActionStateTimer, CharacterKind, HealthPoint, LatLon, MaxHealthPoint,
        MovementState, MovementStateTimer, Team, UserAccount, UserId, UserName, ViewState,
        ViewStateTimer,
    };

    use super::*;

    #[test]
    fn test_pull_stage_packet() {
        let player_0 = PlayPhasePlayer::new(
            UserAccount::new(UserId::new(1412512), UserName::from_str("Aris")),
            CharacterKind::ArisOriginal,
            MaxHealthPoint::new(NonZeroU16::new(1234).unwrap()),
            HealthPoint::new(1413),
            [1.1512351, 2.4151616, 1.16561651],
            [1.5415151, 0.16551351, 0.9513515, 1.0515161],
            Team::Blue,
            ActionState::Aiming,
            ActionStateTimer(3.03151),
            MovementState::InPlaceLanding,
            MovementStateTimer(2.1515),
            ViewState::ZoomIn,
            ViewStateTimer(6.1412),
            LatLon {
                lat: 1.3151613,
                lon: 0.0154123,
            },
        );
        let capture_point = CapturePoint::default();

        let origin = PullStagePacket::new(vec![player_0], vec![], capture_point);
        let raw = origin.as_raw();
        let other = PullStagePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
