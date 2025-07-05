//! 인게임 장면을 갱신 하는 패킷과 관련된 코드를 관리합니다.
//!

use ahash::{HashMap, RandomState};

use crate::{
    components::{
        BigEndian, InGamePlayerPullData, StateChangeEvent, TryFromBigEndian, UserId,
        MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 인게임 장면 갱신 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGamePullPacket {
    /// 게임 플레이 경과 시간. (단위: ms)
    pub play_elapsed_time_ms: u32,
    /// 플레이어 데이터
    pub players: Vec<InGamePlayerPullData>,
    /// 각 플레이어의 상태 변화 이벤트 목록입니다.
    pub events: HashMap<UserId, Vec<StateChangeEvent>>,
}

impl InGamePullPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        play_elapsed_time_ms: u32,
        players: Vec<InGamePlayerPullData>,
        events: HashMap<UserId, Vec<StateChangeEvent>>,
    ) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");

        Self {
            play_elapsed_time_ms,
            players,
            events,
        }
    }
}

impl Packet for InGamePullPacket {
    fn packet_type() -> PacketType {
        PacketType::InGamePull
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_players = self.players.len();
        let mut data = Vec::new();
        data.extend_from_slice(&self.play_elapsed_time_ms.to_big_endian_bytes());
        data.extend_from_slice(&(num_players as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());

            if let Some(events) = self.events.get(&player.uid) {
                let num_events = events.len() as u8;
                data.extend_from_slice(&num_events.to_big_endian_bytes());
                for event in events {
                    data.extend_from_slice(&event.to_big_endian_bytes());
                }
            } else {
                let num_events = 0u8;
                data.extend_from_slice(&num_events.to_big_endian_bytes());
            }
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

        // 현재 플레이 경과 시간을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let play_elapsed_time_ms = u32::from_big_endian_bytes(data);

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players <= 0 || num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        let mut players = Vec::with_capacity(num_players);
        let mut events = HashMap::with_capacity_and_hasher(num_players, RandomState::new());
        for _ in 0..num_players {
            // 플레이어 데이터를 가져옵니다.
            offset = offset + size;
            size = InGamePlayerPullData::byte_size();
            data = &bytes[offset..offset + size];
            let player = InGamePlayerPullData::from_big_endian_bytes(data);

            // 이벤트의 수를 가져옵니다.
            offset = offset + size;
            size = u8::byte_size();
            data = &bytes[offset..offset + size];
            let num_events = u8::from_big_endian_bytes(data) as usize;
            let mut states_events = Vec::with_capacity(num_events);
            for _ in 0..num_events {
                offset = offset + size;
                size = StateChangeEvent::byte_size();
                data = &bytes[offset..offset + size];
                states_events.push(StateChangeEvent::try_from_big_endian_bytes(data)?);
            }
            events.insert(player.uid, states_events);
            players.push(player);
        }

        Some(Self {
            play_elapsed_time_ms,
            players,
            events,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{
        ActionState, ActionStateTimer, LatLon, MovementState, MovementStateTimer, NetworkState,
        Permission, PlayerStateData, UserId,
    };

    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_in_game_pull_packet() {
        InGamePullPacket::new(30_000, vec![], HashMap::default());
    }

    #[test]
    fn test_in_game_pull_packet() {
        let player_0 = InGamePlayerPullData::new(
            UserId::new(13413451),
            0,
            12,
            10,
            3100,
            4,
            425,
            [10.0241, 0.0111, 5.031413],
            [0.00134123, 0.0061341, 0.7341341, 0.212341],
            [0.0, 0.13414132, 0.513411],
            Permission::Admin,
            true,
            true,
            true,
            NetworkState::Poor,
            PlayerStateData::new()
                .with_action_state(ActionState::Attack)
                .with_movement_state(MovementState::Landing),
            ActionStateTimer::new(320),
            MovementStateTimer::new(1200),
            LatLon::new(45f32.to_radians(), 72f32.to_radians()),
        );
        let player_1 = InGamePlayerPullData::new(
            UserId::new(98431),
            12,
            2,
            210,
            1100,
            25,
            725,
            [10.0241, 0.0111, 5.031413],
            [0.00134123, 0.0061341, 0.7341341, 0.212341],
            [0.0, 0.13414132, 0.513411],
            Permission::User,
            false,
            true,
            false,
            NetworkState::Good,
            PlayerStateData::new()
                .with_action_state(ActionState::Attack)
                .with_movement_state(MovementState::Landing),
            ActionStateTimer::new(323),
            MovementStateTimer::new(1212),
            LatLon::new(-11f32.to_radians(), 63f32.to_radians()),
        );

        let mut events = HashMap::default();
        events.insert(
            UserId::new(13413451),
            vec![
                StateChangeEvent::MovementState {
                    movement_state: MovementState::Idle,
                    play_elapsed_time_ms: 40_921,
                },
                StateChangeEvent::ActionState {
                    action_state: ActionState::Attack,
                    play_elapsed_time_ms: 40_921,
                },
            ],
        );
        events.insert(
            UserId::new(98431),
            vec![
                StateChangeEvent::MovementState {
                    movement_state: MovementState::Moving,
                    play_elapsed_time_ms: 40_121,
                },
                StateChangeEvent::ActionState {
                    action_state: ActionState::Attack,
                    play_elapsed_time_ms: 40_921,
                },
                StateChangeEvent::MovementState {
                    movement_state: MovementState::Idle,
                    play_elapsed_time_ms: 41_221,
                },
            ],
        );

        let players = vec![player_0, player_1];
        let origin = InGamePullPacket::new(42_123, players, events);
        let raw = origin.as_raw();
        let other = InGamePullPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
