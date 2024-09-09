#[derive(Debug, PartialEq)]
pub struct Player {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Player {
    pub fn new(id: u32, x: f32, y: f32, z: f32) -> Self {
        Self {
            id,
            x,
            y,
            z,
        }
    }
    
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 * 4);
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.x.to_be_bytes());
        bytes.extend_from_slice(&self.y.to_be_bytes());
        bytes.extend_from_slice(&self.z.to_be_bytes());
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Player {
        Player {
            id: u32::from_be_bytes(data[0..4].try_into().unwrap()),
            x: f32::from_be_bytes(data[4..8].try_into().unwrap()),
            y: f32::from_be_bytes(data[8..12].try_into().unwrap()),
            z: f32::from_be_bytes(data[12..16].try_into().unwrap()),
        }
    }
}