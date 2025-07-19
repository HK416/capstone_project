//! 위도와 경도 데이터와 관련된 코드를 관리합니다.
//!

use crate::components::BigEndian;

/// 위도(Latitude)/경도(Longitude)로 구면좌표를 나타냅니다. 단위는 radian입니다.  
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon {
    pub lat: f32,
    pub lon: f32,
}

impl LatLon {
    /// 새로운 구면 좌표 데이터를 생성합니다.
    pub const fn new(lat: f32, lon: f32) -> Self {
        Self { lat, lon }
    }
}

impl BigEndian for LatLon {
    fn byte_size() -> usize {
        f32::byte_size() + f32::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LatLon)
            )
        };

        // 위도 데이터를 가져옵니다.
        let mut offset = 0;
        let mut size = f32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let lat = f32::from_big_endian_bytes(data);

        // 경도 데이터를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let lon = f32::from_big_endian_bytes(data);

        Self { lat, lon }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.lat.to_bits().to_big_endian_bytes());
        bytes.extend_from_slice(&self.lon.to_bits().to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LatLon)
            );
        }

        bytes
    }
}

impl Default for LatLon {
    fn default() -> Self {
        Self { lat: 0.0, lon: 0.0 }
    }
}
