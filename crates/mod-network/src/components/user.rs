use std::mem;

use super::{BigEndian, UserId};

/// 사용자 닉네임 크기입니다.
const MAX_NAME_BUF_SIZE: usize = 16;
/// 최대 사용자 닉네임 길이입니다.
pub const MAX_NAME_LEN: usize = MAX_NAME_BUF_SIZE - 1;

/// UTF-16 형식의 사용자 닉네임 데이터입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserName([u16; MAX_NAME_BUF_SIZE]);

impl UserName {
    /// 문자열로부터 사용자 닉네임을 생성합니다.
    ///
    /// 이 함수는 자동으로 문자열 사이의 공백을 제거하고, 닉네임 길이만큼 문자열을 자릅니다.
    ///
    pub fn new<T: AsRef<str>>(s: T) -> Self {
        let mut iter = s.as_ref().trim().encode_utf16();
        let mut name = [0; MAX_NAME_BUF_SIZE];
        for i in 0..MAX_NAME_LEN {
            match iter.next() {
                Some(ch) => name[i] = ch,
                None => break,
            };
        }
        Self(name)
    }
}

impl BigEndian for UserName {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        const SIZE: usize = mem::size_of::<u16>();
        let mut name = [0; MAX_NAME_BUF_SIZE];
        let mut offset = 0;
        for i in 0..MAX_NAME_LEN {
            let data = &bytes[offset..offset + SIZE];
            name[i] = u16::from_big_endian_bytes(data);
            offset += SIZE;
        }

        Self(name)
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        let name = &self.0;
        for i in 0..MAX_NAME_BUF_SIZE {
            bytes.extend_from_slice(&name[i].to_big_endian_bytes());
        }
        bytes
    }
}

impl Default for UserName {
    fn default() -> Self {
        Self([0; MAX_NAME_BUF_SIZE])
    }
}

impl ToString for UserName {
    fn to_string(&self) -> String {
        String::from_utf16_lossy(&self.0)
    }
}

/// 사용자 정보입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User {
    /// 사용자 식별자
    id: UserId,
    /// 사용자 닉네임 (UTF-16)
    name: UserName,
}

impl User {
    pub fn new(id: UserId, name: UserName) -> Self {
        Self { id, name }
    }

    /// 사용자 식별자를 반환합니다.
    pub fn id(&self) -> UserId {
        self.id
    }

    /// 사용자 이름을 반환합니다.
    pub fn name(&self) -> &UserName {
        &self.name
    }
}

impl BigEndian for User {
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

        Self { id: user_id, name }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.id.to_big_endian_bytes());
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

impl Default for User {
    fn default() -> Self {
        Self {
            id: UserId::default(),
            name: UserName::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_user() {
        let origin = User {
            id: UserId::new(3141592),
            name: UserName::new("Hello안녕!"),
        };
        let bytes = origin.to_big_endian_bytes();
        let other = User::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(User::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
