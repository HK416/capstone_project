//! 인게임 장면을 갱신 하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{
        BigEndian, InGameBulletPullData, InGamePlayerPullData, MAX_IN_GAME_BULLETS,
        MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 인게임 장면 갱신 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGamePullPacket {
    /// 현재 클라이언트의 Ping
    pub ping: u16,
    /// 서버의 게임 플레이 경과 시간. (단위: ms)
    pub play_elapsed_time_ms: u32,
    /// 플레이어 데이터
    pub players: Vec<InGamePlayerPullData>,
    /// 총알 데이터
    pub bullets: Vec<InGameBulletPullData>,
}

impl InGamePullPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    /// 주어진 `bullets`의 요소 수가 `MAX_IN_GAME_BULLETS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        play_elapsed_time_ms: u32,
        players: Vec<InGamePlayerPullData>,
        bullets: Vec<InGameBulletPullData>,
    ) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");
        assert!(bullets.len() <= MAX_IN_GAME_BULLETS, "too many bullets!");

        Self {
            ping: 0,
            play_elapsed_time_ms,
            players,
            bullets,
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
        let num_bullets = self.bullets.len();
        let data_size = u16::byte_size()
            + u32::byte_size()
            + u8::byte_size()
            + InGamePlayerPullData::byte_size() * num_players
            + u16::byte_size()
            + InGameBulletPullData::byte_size() * num_bullets;
        let num_players = self.players.len();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.ping.to_big_endian_bytes());
        data.extend_from_slice(&self.play_elapsed_time_ms.to_big_endian_bytes());
        data.extend_from_slice(&(num_players as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }
        data.extend_from_slice(&(num_bullets as u16).to_big_endian_bytes());
        for bullet in self.bullets.iter() {
            data.extend_from_slice(&bullet.to_big_endian_bytes());
        }

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePullPacket)
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

        // 현재 클라이언트의 Ping을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let ping = u16::from_big_endian_bytes(data);

        // 서버의 플레이 경과 시간을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let play_elapsed_time_ms = u32::from_big_endian_bytes(data);

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players <= 0 || num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        // 플레이어 데이터를 가져옵니다.
        let mut players = Vec::with_capacity(num_players);
        for _ in 0..num_players {
            offset = offset + size;
            size = InGamePlayerPullData::byte_size();
            data = &bytes[offset..offset + size];
            players.push(InGamePlayerPullData::from_big_endian_bytes(data));
        }

        // 총알의 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let num_bullets = u16::from_big_endian_bytes(data) as usize;

        // 총알 데이터를 가져옵니다.
        let mut bullets = Vec::with_capacity(num_bullets);
        for _ in 0..num_bullets {
            offset = offset + size;
            size = InGameBulletPullData::byte_size();
            data = &bytes[offset..offset + size];
            bullets.push(InGameBulletPullData::from_big_endian_bytes(data));
        }

        Some(Self {
            ping,
            play_elapsed_time_ms,
            players,
            bullets,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use mod_physics::object3d::Capsule;

    use crate::components::{
        ActionState, ActionStateTimer, BulletKind, CharacterAttributes, Float3, LatLon,
        MovementState, MovementStateTimer, ObjectId, UserId,
    };

    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_in_game_pull_packet() {
        InGamePullPacket::new(30_000, vec![], vec![]);
    }

    #[test]
    fn test_in_game_pull_packet() {
        let attributes = CharacterAttributes {
            speed: 5.0,
            left_weapon: None,
            right_weapon: None,
            attack_head_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            attack_spine_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            attack_spine1_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            skill_head_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            skill_spine_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            skill_spine1_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            normal_idle_duration: 1200,
            cafe_walk_duration: 0,
            move_ing_duration: 0,
            move_end_normal_duration: 0,
            normal_attack_start_duration: 0,
            normal_attack_end_duration: 0,
            normal_attack_ing_duration: 400,
            vital_death_duration: 0,
            normal_reload_duration: 0,
            skill_duration: 0,
            normal_callsign_duration: 0,
            victory_start_duration: 0,
            victory_end_duration: 0,
            normal_attack_timing: vec![],
            normal_attack_count: 0,
            max_bullets: 0,
            max_health_point: 0,
            attack_power: 0,
            defense_power: 0,
            accuracy_stat: 0,
            evasion_stat: 0,
            critical_rate: 0,
            critical_damage: 0,
            max_skill_cost: 0,
            skill_cost: 0,
            attack_range: 0,
            bullet_radius: 0.0,
            collider: Capsule {
                center: glam::vec3(0.0, 0.0, 0.0),
                height: 0.0,
                radius: 0.0,
            },
        };
        let player_0 = InGamePlayerPullData::new(
            UserId::new(13413451),
            NonZeroU32::new(25).unwrap(),
            NonZeroU32::new(25).unwrap(),
            NonZeroU32::new(25).unwrap(),
            glam::vec3a(10.0241, 0.0111, 5.031413),
            glam::quat(0.00134123, 0.0061341, 0.7341341, 0.212341),
            ActionState::Attack,
            ActionStateTimer::new(320),
            MovementState::Landing,
            MovementStateTimer::new(1200),
            &attributes,
            LatLon::new(45f32.to_radians(), 72f32.to_radians()),
        );
        let player_1 = InGamePlayerPullData::new(
            UserId::new(98431),
            NonZeroU32::new(25).unwrap(),
            NonZeroU32::new(25).unwrap(),
            NonZeroU32::new(25).unwrap(),
            glam::vec3a(10.0241, 0.0111, 5.031413),
            glam::quat(0.00134123, 0.0061341, 0.7341341, 0.212341),
            ActionState::Attack,
            ActionStateTimer::new(323),
            MovementState::Landing,
            MovementStateTimer::new(1212),
            &attributes,
            LatLon::new(-11f32.to_radians(), 63f32.to_radians()),
        );

        let bullet_0 = InGameBulletPullData::new(
            ObjectId::new(89431),
            BulletKind::Common,
            NonZeroU32::new(25).unwrap(),
            NonZeroU32::new(25).unwrap(),
            NonZeroU32::new(25).unwrap(),
            glam::vec3a(0.43124, 0.341341, 10.414321),
            glam::quat(0.00134123, 0.0061341, 0.7341341, 0.212341),
        );
        let bullets = vec![bullet_0];

        let players = vec![player_0, player_1];
        let origin = InGamePullPacket::new(42_123, players, bullets);
        let raw = origin.as_raw();
        let other = InGamePullPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
