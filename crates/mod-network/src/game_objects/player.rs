use crate::components::BigEndian;

use super::super::components::{
    ObjectId,
    CharacterKind,
    ActionState,
    MovementState,
    ViewState,
    AnimationTimer,
};

#[repr(C)]      // 서버에서 사용하기 때문에 packed로 설정하면 속도 저하가 발생할 수 있음
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    pub id: ObjectId,
    pub hp: u32,
    pub translation: gmm::Float3, 
    pub rotation: gmm::Float4, 
    pub character_kind: CharacterKind,      // --+ 4byte로 묶이도록 배치(각각이 1byte여야함)
    pub action_state: ActionState,          //   |
    pub movement_state: MovementState,      //   |
    pub view_state: ViewState,              // --+
    pub anim_timer: AnimationTimer,
}

impl Player {
    pub fn new(
        id: ObjectId, 
        translation: gmm::Float3, 
        rotation: gmm::Float4, 
        character_kind: CharacterKind,
        action_state: ActionState,
        movement_state: MovementState,
        view_state: ViewState,
        anim_timer: AnimationTimer, 
    ) -> Self {
        Self {
            id, 
            hp: 100,
            translation, 
            rotation, 
            character_kind,
            action_state,
            movement_state,
            view_state,
            anim_timer, 
        }
    }
    
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Player>());
        bytes.extend_from_slice(&self.id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.hp.to_be_bytes());
        bytes.extend_from_slice(&self.translation.x.to_be_bytes());
        bytes.extend_from_slice(&self.translation.y.to_be_bytes());
        bytes.extend_from_slice(&self.translation.z.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.x.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.y.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.z.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.w.to_be_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.view_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.anim_timer.to_big_endian_bytes());
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Player {
        let mut start = 0;
        let mut end = start + size_of::<ObjectId>();
        let id = ObjectId::from_big_endian_bytes(&data[start..end]);

        start = end;
        end = start + size_of::<u32>();
        let hp = u32::from_be_bytes(data[start..end].try_into().unwrap());

        start = end;
        end = start + size_of::<gmm::Float3>();
        let translation = gmm::Float3::new(
            f32::from_be_bytes(data[start..start+4].try_into().unwrap()),
            f32::from_be_bytes(data[start+4..start+8].try_into().unwrap()),
            f32::from_be_bytes(data[start+8..start+12].try_into().unwrap()),
        );

        start = end;
        end = start + size_of::<gmm::Float4>();
        let rotation = gmm::Float4::new(
            f32::from_be_bytes(data[start..start+4].try_into().unwrap()),
            f32::from_be_bytes(data[start+4..start+8].try_into().unwrap()),
            f32::from_be_bytes(data[start+8..start+12].try_into().unwrap()),
            f32::from_be_bytes(data[start+12..start+16].try_into().unwrap()),
        );

        start = end;
        end = start + size_of::<CharacterKind>();
        let character_kind = CharacterKind::from_big_endian_bytes(&data[start..end]);

        start = end;
        end = start + size_of::<ActionState>();
        let action_state = ActionState::from_big_endian_bytes(&data[start..end]);

        start = end;
        end = start + size_of::<MovementState>();
        let movement_state = MovementState::from_big_endian_bytes(&data[start..end]);

        start = end;
        end = start + size_of::<ViewState>();
        let view_state = ViewState::from_big_endian_bytes(&data[start..end]);

        start = end;
        end = start + size_of::<AnimationTimer>();
        let anim_timer = AnimationTimer::from_big_endian_bytes(&data[start..end]);

        Player {
            id, 
            hp,
            translation, 
            rotation, 
            character_kind,
            action_state,
            movement_state,
            view_state,
            anim_timer, 
        }
    }
}

impl Default for Player {
    #[inline]
    fn default() -> Self {
        Self { 
            id: ObjectId::new(1), 
            hp: 100,
            translation: gmm::Float3::ZERO, 
            rotation: gmm::Float4::W, 
            character_kind: CharacterKind::ArisOriginal,
            action_state: ActionState::Idle,
            movement_state: MovementState::Idle,
            view_state: ViewState::Idle,
            anim_timer: AnimationTimer(0.0), 
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player() {
        let player = Player::default();
        let bytes = player.as_bytes();
        let player2 = Player::from_bytes(&bytes);

        assert_eq!(size_of::<Player>(), bytes.len());   // 바이트 정렬이 잘 되어있지 않다면 실패한다(패딩이 없어야함)
        assert_eq!(player, player2);
    }
}