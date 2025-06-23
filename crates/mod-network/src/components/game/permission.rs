//! 플레이어 권한과 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 커스텀 게임 대기실에서 플레이어의 권한
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    User = 0,
    Admin = 1,
}

impl Permission {
    /// 주어진 정수로 부터 `Permission`을 생성합니다.  
    ///
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Permission::User),
            1 => Some(Permission::Admin),
            _ => None,
        }
    }
}

impl BigEndian for Permission {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for Permission {
    fn default() -> Self {
        Self::User
    }
}

impl TryFromBigEndian for Permission {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_permission() {
        Permission::new(4).unwrap();
    }

    #[test]
    fn test_creation_permission_user() {
        let val = Permission::User as u8;
        let permission = Permission::new(val).unwrap();
        assert_eq!(Permission::User, permission);
    }

    #[test]
    fn test_creation_permission_admin() {
        let val = Permission::Admin as u8;
        let permission = Permission::new(val).unwrap();
        assert_eq!(Permission::Admin, permission);
    }

    #[test]
    fn test_permission() {
        let origin = Permission::User;
        let bytes = origin.to_big_endian_bytes();
        let other = Permission::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
