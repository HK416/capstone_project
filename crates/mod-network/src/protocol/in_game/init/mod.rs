//! 서버에서 클라이언트로 보내는 스테이지 로드 요청 패킷과 관련된 코드를 관리합니다.
//!

mod player;

use crate::{
    components::{BigEndian, StageKind, TryFromBigEndian, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

pub use self::player::*;

/// 서버에서 클라이언트로 보내는 스테이지 로드 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InitStagePacket {
    /// 스테이지 종류입니다.
    pub stage_kind: StageKind,
    /// 플레이어 초기화 데이터입니다.
    pub players: Vec<PlayerSetupData>,
}

impl InitStagePacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`가 `MAX_IN_GAME_PLAYER`를 초과하거나
    /// 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(stage_kind: StageKind, players: Vec<PlayerSetupData>) -> Self {
        assert!(!players.is_empty(), "the given player is empty!");
        assert!(
            players.len() <= MAX_IN_GAME_PLAYERS,
            "too many players given!"
        );

        Self {
            stage_kind,
            players,
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`가 `MAX_IN_GAME_PLAYER`를 초과하거나
    /// 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(stage_kind: StageKind, players: I) -> Self
    where
        I: IntoIterator<Item = PlayerSetupData>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(stage_kind, players.into_iter().collect())
    }
}

impl Packet for InitStagePacket {
    fn packet_type() -> PacketType {
        PacketType::InitStage
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = StageKind::byte_size()
            + u8::byte_size()
            + PlayerSetupData::byte_size() * self.players.len();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.stage_kind.to_big_endian_bytes());
        data.extend_from_slice(&(self.players.len() as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }

        // 바이트 배열의 크기를 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InitStagePacket)
            )
        };

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

        // 스테이지 종류를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = StageKind::byte_size();
        let mut data = &bytes[offset..offset + size];
        let stage_kind = StageKind::try_from_big_endian_bytes(data)?;

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let mut num_players = u8::from_big_endian_bytes(data);

        // 플레이어 데이터를 가져옵니다.
        let mut players = Vec::with_capacity(num_players as usize);
        while num_players > 0 {
            offset = offset + size;
            size = PlayerSetupData::byte_size();
            data = &bytes[offset..offset + size];
            let player = PlayerSetupData::try_from_big_endian_bytes(data)?;
            players.push(player);
            num_players -= 1;
        }

        Some(Self {
            stage_kind,
            players,
        })
    }
}
