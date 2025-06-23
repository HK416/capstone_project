//! 사용자 계정 이름과 관련된 코드를 관리합니다.
//!

use std::fmt;

use crate::components::BigEndian;

/// 사용자 닉네임 문자열 버퍼의 크기입니다.
const MAX_NAME_BUF_SIZE: usize = 16;
/// 사용자 닉네임의 최대 길이입니다.
pub const MAX_NAME_LEN: usize = MAX_NAME_BUF_SIZE - 1;

/// UTF-16 형식의 사용자 이름 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserName {
    buffer: [u16; MAX_NAME_BUF_SIZE],
    len: u16,
}

impl UserName {
    /// 문자열로부터 사용자 이름을 생성합니다.
    ///
    /// 주어진 문자열에서 자동으로 문자열 사이의 공백을 제거하고,
    /// [`MAX_NAME_LEN`]만큼 문자열을 자릅니다.
    ///
    pub fn from_str<T: AsRef<str>>(s: T) -> Self {
        let mut iter = s.as_ref().trim().encode_utf16();
        let mut buffer = [0; MAX_NAME_BUF_SIZE];
        let mut len = 0;
        for i in 0..MAX_NAME_LEN {
            match iter.next() {
                Some(c) => {
                    buffer[i] = c;
                    len += 1;
                }
                None => break,
            };
        }

        Self { buffer, len }
    }
}

impl BigEndian for UserName {
    fn byte_size() -> usize {
        u8::byte_size() + u16::byte_size() * MAX_NAME_BUF_SIZE // 33byte
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(UserName)
            )
        };

        // 문자열 길이를 가져옵니다.
        let mut offset = 0;
        let mut size = u8::byte_size();
        let mut data = &bytes[offset..offset + size];
        let len = u8::from_big_endian_bytes(data) as usize;

        // 문자열 데이터를 가져옵니다.
        let mut buffer = [0; MAX_NAME_BUF_SIZE];
        for i in 0..MAX_NAME_LEN.min(len) {
            offset = offset + size;
            size = u16::byte_size();
            data = &bytes[offset..offset + size];
            buffer[i] = u16::from_big_endian_bytes(data);
        }

        Self {
            buffer,
            len: len as u16,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&(self.len as u8).to_big_endian_bytes());
        for i in 0..MAX_NAME_BUF_SIZE {
            bytes.extend_from_slice(&self.buffer[i].to_big_endian_bytes());
        }

        // 바이트 스트림이 유효한지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(UserName)
            )
        };

        bytes
    }
}

impl Into<String> for UserName {
    fn into(self) -> String {
        let len = self.len as usize;
        String::from_utf16_lossy(&self.buffer[..len])
    }
}

impl Default for UserName {
    fn default() -> Self {
        Self {
            buffer: [0; MAX_NAME_BUF_SIZE],
            len: 0,
        }
    }
}

impl fmt::Display for UserName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.len as usize;
        write!(f, "{}", &String::from_utf16_lossy(&self.buffer[..len]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_name() {
        let origin = UserName::from_str("Hayase Yuuka");
        let bytes = origin.to_big_endian_bytes();
        let other = UserName::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);

        let origin = UserName::from_str("早瀬 ユウカ");
        let bytes = origin.to_big_endian_bytes();
        let other = UserName::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);

        let origin = UserName::from_str("早濑 优香");
        let bytes = origin.to_big_endian_bytes();
        let other = UserName::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
