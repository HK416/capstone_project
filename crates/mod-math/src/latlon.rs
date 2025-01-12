/// 위도(Latitude)/경도(Longitude)로 구면좌표를 나타냅니다.  
/// 단위는 radian입니다.  
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon {
    pub lat: f32,
    pub lon: f32,
}

impl LatLon {
    pub fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        let lat = f32::from_be_bytes(bytes[..4].try_into().unwrap());
        let lon = f32::from_be_bytes(bytes[4..].try_into().unwrap());
        Self { lat, lon }
    }

    pub fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&self.lat.to_be_bytes());
        bytes.extend_from_slice(&self.lon.to_be_bytes());
        bytes
    }
}

impl Default for LatLon {
    fn default() -> Self {
        Self { lat: 0.0, lon: 0.0 }
    }
}
