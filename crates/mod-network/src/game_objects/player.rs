use crate::components::{BigEndian, MovementStateTimer, TryFromBigEndian, ViewStateTimer};

use super::super::components::{
    ObjectId,
    CharacterKind,
    ActionState,
    MovementState,
    ViewState,
    ActionStateTimer,
    HealthPoint,
};

#[repr(C)]      // 서버에서 사용하기 때문에 packed로 설정하면 속도 저하가 발생할 수 있음
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    pub id: ObjectId,
    pub hp: HealthPoint,
    pub translation: [f32; 3], 
    pub rotation: [f32; 4], 
    pub velocity: [f32; 3],
    pub character_kind: CharacterKind,      // --+ 4byte로 묶이도록 배치(각각이 1byte여야함)
    pub action_state: ActionState,          //   |
    pub movement_state: MovementState,      //   |
    pub view_state: ViewState,              // --+
    pub action_state_timer: ActionStateTimer,
    pub movement_state_timer: MovementStateTimer, 
    pub view_state_timer: ViewStateTimer,
}

impl BigEndian for Player {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Player>());
        bytes.extend_from_slice(&self.id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.hp.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.velocity.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.view_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.view_state_timer.to_big_endian_bytes());
        bytes
    }
}

impl TryFromBigEndian for Player {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Player {
            id: ObjectId::try_from_big_endian_bytes(&bytes[0..4])?,
            hp: HealthPoint::from_big_endian_bytes(&bytes[4..8]),
            translation: <[f32; 3]>::from_big_endian_bytes(&bytes[8..20]),
            rotation: <[f32; 4]>::from_big_endian_bytes(&bytes[20..36]),
            velocity: <[f32; 3]>::from_big_endian_bytes(&bytes[36..48]),
            character_kind: CharacterKind::try_from_big_endian_bytes(&bytes[48..49])?,
            action_state: ActionState::try_from_big_endian_bytes(&bytes[49..50])?,
            movement_state: MovementState::try_from_big_endian_bytes(&bytes[50..51])?,
            view_state: ViewState::try_from_big_endian_bytes(&bytes[51..52])?,
            action_state_timer: ActionStateTimer::from_big_endian_bytes(&bytes[52..56]),
            movement_state_timer: MovementStateTimer::from_big_endian_bytes(&bytes[56..60]),
            view_state_timer: ViewStateTimer::from_big_endian_bytes(&bytes[60..64]),
        })
    }
}

impl Default for Player {
    #[inline]
    fn default() -> Self {
        Self { 
            id: ObjectId::new(1), 
            hp: HealthPoint(2000.0),
            translation: [0.0, 0.0, 0.0], 
            rotation: [0.0, 0.0, 0.0, 1.0], 
            velocity: [0.0, 0.0, 0.0],
            character_kind: CharacterKind::ArisOriginal,
            action_state: ActionState::Idle,
            movement_state: MovementState::Idle,
            view_state: ViewState::Idle,
            action_state_timer: ActionStateTimer::default(), 
            movement_state_timer: MovementStateTimer::default(),
            view_state_timer: ViewStateTimer::default(),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player() {
        let player = Player::default();
        let bytes = player.to_big_endian_bytes();
        let player2 = Player::from_big_endian_bytes(&bytes);

        assert_eq!(size_of::<Player>(), bytes.len());   // 바이트 정렬이 잘 되어있지 않다면 실패한다(패딩이 없어야함)
        assert_eq!(player, player2);
    }
}