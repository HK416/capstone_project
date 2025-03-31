//! 사용자 계정과 관련된 코드를 관리합니다.
//!

use std::fmt;

use crate::components::{BigEndian, UserId};

/// 사용자 닉네임 문자열 버퍼의 크기입니다.
const MAX_NAME_BUF_SIZE: usize = 16;
/// 사용자 닉네임의 최대 길이입니다.
pub const MAX_NAME_LEN: usize = MAX_NAME_BUF_SIZE - 1;

/// UTF-16 형식의 사용자 닉네임 문자열
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserName {
    len: usize,
    buffer: [u16; MAX_NAME_BUF_SIZE],
}

impl UserName {
    /// 문자열 요소의 크기입니다.
    pub const ELEMENT_SIZE: usize = core::mem::size_of::<u16>();

    /// 문자열로부터 사용자 닉네임을 생성합니다.
    ///
    /// 이 함수는 자동으로 문자열 사이의 공백을 제거하고, 닉네임 길이만큼 문자열을 자릅니다.
    ///
    pub fn from_str<T: AsRef<str>>(s: T) -> Self {
        let mut iter = s.as_ref().trim().encode_utf16();
        let mut buffer = [0; MAX_NAME_BUF_SIZE];
        let mut len = 0;
        for i in 0..MAX_NAME_LEN {
            match iter.next() {
                Some(ch) => {
                    buffer[i] = ch;
                    len += 1;
                }
                None => break,
            };
        }

        Self { len, buffer }
    }
}

impl BigEndian for UserName {
    fn byte_size() -> usize {
        u8::byte_size() + Self::ELEMENT_SIZE * MAX_NAME_BUF_SIZE
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(UserName)
        );

        // 문자열 길이를 가져옵니다.
        let mut offset = 0;
        let mut size = u8::byte_size();
        let mut data = &bytes[offset..offset + size];
        let len = u8::from_big_endian_bytes(data) as usize;

        // 문자열 버퍼를 가져옵니다.
        let mut buffer = [0; MAX_NAME_BUF_SIZE];
        for i in 0..MAX_NAME_LEN.min(len) {
            offset = offset + size;
            size = Self::ELEMENT_SIZE;
            data = &bytes[offset..offset + size];
            buffer[i] = u16::from_big_endian_bytes(data);
        }

        Self { len, buffer }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&(self.len as u8).to_big_endian_bytes());
        for i in 0..MAX_NAME_BUF_SIZE {
            bytes.extend_from_slice(&self.buffer[i].to_big_endian_bytes());
        }
        bytes
    }
}

impl Default for UserName {
    fn default() -> Self {
        Self {
            len: 0,
            buffer: [0; MAX_NAME_BUF_SIZE],
        }
    }
}

impl fmt::Display for UserName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &String::from_utf16_lossy(&self.buffer[..self.len]))
    }
}

/// 사용자 계정 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserAccount {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 닉네임
    pub name: UserName,
}

impl UserAccount {
    /// 새로운 사용자 정보를 생성합니다.
    pub fn new(id: UserId, name: UserName) -> Self {
        Self { uid: id, name }
    }
}

impl BigEndian for UserAccount {
    fn byte_size() -> usize {
        UserId::byte_size() + UserName::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(User)
        );

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 사용자 닉네임을 가져옵니다.
        offset = offset + size;
        size = UserName::byte_size();
        data = &bytes[offset..offset + size];
        let name = UserName::from_big_endian_bytes(data);

        Self { uid: user_id, name }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.name.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(User)
            );
        }

        bytes
    }
}

impl Default for UserAccount {
    fn default() -> Self {
        Self {
            uid: UserId::default(),
            name: UserName::default(),
        }
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

    #[test]
    fn test_user_info() {
        let origin = UserAccount {
            uid: UserId::new(3141592),
            name: UserName::from_str("Hello안녕!"),
        };
        let bytes = origin.to_big_endian_bytes();
        let other = UserAccount::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
