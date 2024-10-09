#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    pub id: u32,
    pub translation: gmm::Float3, 
    pub rotation: gmm::Float4, 
    pub anim_index: u32, 
    pub anim_timer: f32, 
}

impl Player {
    pub fn new(
        id: u32, 
        translation: gmm::Float3, 
        rotation: gmm::Float4, 
        anim_index: u32, 
        anim_timer: f32, 
    ) -> Self {
        Self {
            id, 
            translation, 
            rotation, 
            anim_index, 
            anim_timer, 
        }
    }
    
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Player>());
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.translation.x.to_be_bytes());
        bytes.extend_from_slice(&self.translation.y.to_be_bytes());
        bytes.extend_from_slice(&self.translation.z.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.x.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.y.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.z.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.w.to_be_bytes());
        bytes.extend_from_slice(&self.anim_index.to_be_bytes());
        bytes.extend_from_slice(&self.anim_timer.to_be_bytes());
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Player {
        let id = u32::from_be_bytes(data[0..4].try_into().unwrap());
        let translation = gmm::Float3::new(
            f32::from_be_bytes(data[4..8].try_into().unwrap()),
            f32::from_be_bytes(data[8..12].try_into().unwrap()),
            f32::from_be_bytes(data[12..16].try_into().unwrap()),
        );
        let rotation = gmm::Float4::new(
            f32::from_be_bytes(data[16..20].try_into().unwrap()),
            f32::from_be_bytes(data[20..24].try_into().unwrap()),
            f32::from_be_bytes(data[24..28].try_into().unwrap()),
            f32::from_be_bytes(data[28..32].try_into().unwrap()),
        );
        let anim_index = u32::from_be_bytes(data[32..36].try_into().unwrap());
        let anim_timer = f32::from_be_bytes(data[36..40].try_into().unwrap());
        Player {
            id, 
            translation, 
            rotation, 
            anim_index, 
            anim_timer, 
        }
    }
}

impl Default for Player {
    #[inline]
    fn default() -> Self {
        Self { 
            id: 0, 
            translation: gmm::Float3::ZERO, 
            rotation: gmm::Float4::W, 
            anim_index: 0, 
            anim_timer: 0.0 
        }
    }
}