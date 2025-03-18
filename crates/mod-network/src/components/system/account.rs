use crate::components::{BigEndian, TryFromBigEndian};

/// 최대 이메일 버퍼의 크기입니다.
const MAX_EMAIL_BUFFER_SIZE: usize = 256;
/// 최대 이메일 문자 수 입니다.
pub const MAX_EMAIL_LENGTH: usize = MAX_EMAIL_BUFFER_SIZE - 1;

/// 계정 이메일 정보를 저장합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Email([u8; MAX_EMAIL_BUFFER_SIZE]);

impl BigEndian for Email {
    fn byte_size() -> usize {
        core::mem::size_of::<u8>() * MAX_EMAIL_BUFFER_SIZE
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(Email)
        );

        // 이메일 정보를 가져옵니다.
        const SIZE: usize = core::mem::size_of::<u8>();
        let mut buffer = [0; MAX_EMAIL_BUFFER_SIZE];
        for i in 0..MAX_EMAIL_LENGTH {
            let data = &bytes[i..i + SIZE];
            buffer[i] = u8::from_big_endian_bytes(data);
        }

        Self(buffer)
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let src = &self.0;
        let mut buffer = Vec::with_capacity(Self::byte_size());
        for i in 0..MAX_EMAIL_BUFFER_SIZE {
            buffer.extend_from_slice(&src[i].to_big_endian_bytes());
        }
        buffer
    }
}

impl Default for Email {
    fn default() -> Self {
        Self([0; MAX_EMAIL_BUFFER_SIZE])
    }
}

/// 최대 비밀번호 버퍼의 크기입니다.
const MAX_PASSWD_BUFFER_SIZE: usize = 48;
/// 최대 비밀번호의 길이입니다.
pub const MAX_PASSWD_LENGTH: usize = MAX_PASSWD_BUFFER_SIZE - 1;

/// 계정 비밀번호 정보를 저장합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Passwd([u8; MAX_PASSWD_BUFFER_SIZE]);

impl BigEndian for Passwd {
    fn byte_size() -> usize {
        core::mem::size_of::<u8>() * MAX_PASSWD_BUFFER_SIZE
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(Passwd)
        );

        // 비밀번호 정보를 가져옵니다.
        const SIZE: usize = core::mem::size_of::<u8>();
        let mut buffer = [0; MAX_PASSWD_BUFFER_SIZE];
        for i in 0..MAX_PASSWD_LENGTH {
            let data = &bytes[i..i + SIZE];
            buffer[i] = u8::from_big_endian_bytes(data);
        }

        Self(buffer)
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let src = &self.0;
        let mut buffer = Vec::with_capacity(Self::byte_size());
        for i in 0..MAX_PASSWD_BUFFER_SIZE {
            buffer.extend_from_slice(&src[i].to_big_endian_bytes());
        }
        buffer
    }
}

impl Default for Passwd {
    fn default() -> Self {
        Self([0; MAX_PASSWD_BUFFER_SIZE])
    }
}

/// 로그인 실패 사유 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoginFailedReason {
    /// 이메일 또는 비밀번호가 잘못됐습니다.
    Invalid = 0,
    /// 이미 사용자 계정이 로그인되어 있습니다.
    AlreadyExists = 1,
    /// 계정이 서버 관리자에 의해 차단당했습니다.
    Banned = 2,
}

impl LoginFailedReason {
    /// 주어진 정수로 새로운 `LoginFailedReason`을 생성합니다.  
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Invalid),
            1 => Some(Self::AlreadyExists),
            2 => Some(Self::Banned),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(LoginFailedReason),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for LoginFailedReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let val = *self as u8;
        val.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for LoginFailedReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email() {
        let mut email = Email::default();
        let arr = email.0.as_mut_slice();
        arr[0] = 52;
        arr[1] = 83;
        arr[2] = 19;
        arr[3] = 19;
        arr[4] = 129;
        arr[5] = 190;

        let origin = email;
        let bytes = origin.to_big_endian_bytes();
        let other = Email::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_passwd() {
        let mut passwd = Passwd::default();
        let arr = passwd.0.as_mut_slice();
        arr[0] = 52;
        arr[1] = 83;
        arr[2] = 19;
        arr[3] = 19;
        arr[4] = 129;
        arr[5] = 190;

        let origin = passwd;
        let bytes = origin.to_big_endian_bytes();
        let other = Passwd::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_login_failed_reason() {
        let origin = LoginFailedReason::Banned;
        let bytes = origin.to_big_endian_bytes();
        let other = LoginFailedReason::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
