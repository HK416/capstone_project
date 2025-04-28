use crate::{
    components::{BigEndian, PlayPhasePlayer, TryFromBigEndian, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 월드 정보 갱신을 위한 패킷
#[derive(Debug, Clone, PartialEq)]
pub struct PrepareStagePacket {
    pub players: Vec<PlayPhasePlayer>,
    pub remaining_time_sec: f32,
}

impl PrepareStagePacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`가 없거나, `MAX_IN_GAME_PLAYERS`를 초과할 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(players: Vec<PlayPhasePlayer>, remaining_time_sec: f32) -> Self {
        assert!(!players.is_empty(), "player data is empty!");
        assert!(
            players.len() <= MAX_IN_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );

        Self {
            players,
            remaining_time_sec,
        }
    }
}

impl Packet for PrepareStagePacket {
    fn packet_type() -> PacketType {
        PacketType::PrepareStage
    }

    fn as_raw(&self) -> RawPacket {
        let data_size =
            u8::byte_size() + PlayPhasePlayer::byte_size() * self.players.len() + f32::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&(self.players.len() as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }
        data.extend_from_slice(&self.remaining_time_sec.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PrepareStagePacket)
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

        // 남은 시간 데이터를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let remaining_time_sec = f32::from_big_endian_bytes(data);

        Some(Self {
            players,
            remaining_time_sec,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{
        ActionState, ActionStateTimer, CharacterKind, ExSkillCost, GamePlayData, HealthPoint,
        LatLon, MovementState, MovementStateTimer, RemainingBullet, Team, UserAccount, UserId,
        UserName, ViewState, ViewStateTimer,
    };

    use super::*;

    #[test]
    fn test_prepare_stage_packet() {
        let player_0 = PlayPhasePlayer::new(
            true,
            UserAccount::new(UserId::new(1412512), UserName::from_str("Aris")),
            GamePlayData {
                kill_count: 0,
                dead_count: 0,
            },
            CharacterKind::ArisOriginal,
            RemainingBullet::new(30, 30),
            HealthPoint::new(1413, 1413),
            [1.1512351, 2.4151616, 1.16561651],
            [1.5415151, 0.16551351, 0.9513515, 1.0515161],
            Team::Blue,
            0,
            ExSkillCost(0.0),
            ActionState::Aiming,
            ActionStateTimer(3.03151),
            MovementState::Idle,
            MovementStateTimer(2.1515),
            ViewState::Idle,
            ViewStateTimer(6.1412),
            LatLon {
                lat: 1.3151613,
                lon: 0.0154123,
            },
        );

        let origin = PrepareStagePacket::new(vec![player_0], 7.1345);
        let raw = origin.as_raw();
        let other = PrepareStagePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
