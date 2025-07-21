use crate::components::BigEndian;

/// 버전 정보를 저장하는 최대 버퍼 크기
const MAX_BUFFER_SIZE: usize = 16;

/// 클라이언트와 서버의 버전 정보입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version([u8; MAX_BUFFER_SIZE]);

impl Version {
    /// 현재 프로그램의 버전 정보를 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }
}

impl BigEndian for Version {
    fn byte_size() -> usize {
        core::mem::size_of::<u8>() * MAX_BUFFER_SIZE
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(Version)
        );

        // 버전 정보를 가져옵니다.
        const SIZE: usize = core::mem::size_of::<u8>();
        let mut buffer = [0; MAX_BUFFER_SIZE];
        for i in 0..MAX_BUFFER_SIZE {
            let data = &bytes[i..i + SIZE];
            buffer[i] = u8::from_big_endian_bytes(data);
        }

        Self(buffer)
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let src = &self.0;
        let mut buffer = Vec::with_capacity(Self::byte_size());
        for i in 0..MAX_BUFFER_SIZE {
            buffer.extend_from_slice(&src[i].to_big_endian_bytes());
        }
        buffer
    }
}

impl Default for Version {
    fn default() -> Self {
        Self(
            env!("CARGO_PKG_VERSION")
                .as_bytes()
                .try_into()
                .unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let origin = Version::new();
        let bytes = origin.to_big_endian_bytes();
        let other = Version::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
