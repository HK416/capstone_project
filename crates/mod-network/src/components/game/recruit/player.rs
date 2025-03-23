use crate::components::{BigEndian, Permission, Team, UserAccount};

/// 플레이어 모집 단계일 떄 플레이어 데이터를 저장합니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecruitPhasePlayer {
    /// 플레이어 계정 데이터
    pub account: UserAccount,

    /// 여러 자료형의 데이터를 저장한 비트 필드입니다.  
    /// 아래 데이터가 포함됩니다.
    /// - Team (1bit): 플레이어가 속한 팀의 종류를 나타냄
    /// - bool (1bit): 플레이어가 준비되었는지 여부를 나타냄
    /// - Permission (1bit): 플레이어의 권한 정보를 나타냄
    ///
    pub bitfield: u8,
}

// bitfield 구조
// +------+-------------+--------------+-------------------+
// | 5bit | team (1bit) | ready (1bit) | permission (1bit) |
// +------+-------------+--------------+-------------------+
//
impl RecruitPhasePlayer {
    /// 새로운 플레이어 데이터를 생성합니다.
    pub fn new(account: UserAccount, team: Team, ready: bool, permission: Permission) -> Self {
        let team_field = (team as u8) << 2;
        let ready_field = (ready as u8) << 1;
        let permission_field = (permission as u8) << 0;

        Self {
            account,
            bitfield: team_field | ready_field | permission_field,
        }
    }

    /// Team을 설정합니다.
    pub fn with_team(&mut self, team: Team) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 2)) | (team as u8) << 2;
        self
    }

    /// Team을 가져옵니다.
    pub fn team(&self) -> Team {
        // Safe: 값이 범위를 벗어나지 않음
        unsafe { Team::new((self.bitfield >> 2) & 0x1).unwrap_unchecked() }
    }

    /// 준비 여부를 설정합니다.
    pub fn with_ready(&mut self, ready: bool) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 1)) | (ready as u8) << 1;
        self
    }

    /// 준비 여부를 가져옵니다.
    pub fn is_ready(&self) -> bool {
        if (self.bitfield >> 1) & 0x1 == 0 {
            false
        } else {
            true
        }
    }

    /// 권한을 설정합니다.
    pub fn with_permission(&mut self, permission: Permission) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 0)) | (permission as u8) << 0;
        self
    }

    /// 권한을 가져옵니다.
    pub fn permission(&self) -> Permission {
        // Safe: 값이 범위를 벗어나지 않음
        unsafe { Permission::new((self.bitfield >> 0) & 0x1).unwrap_unchecked() }
    }
}

impl BigEndian for RecruitPhasePlayer {
    fn byte_size() -> usize {
        UserAccount::byte_size() + u8::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(RecruitPhasePlayer)
        );

        // 사용자 계정 데이터를 가져옵니다.
        let mut offset = 0;
        let mut size = UserAccount::byte_size();
        let mut data = &bytes[offset..offset + size];
        let account = UserAccount::from_big_endian_bytes(data);

        // 비트 필드를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = u8::from_big_endian_bytes(data);

        Self { account, bitfield }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.account.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(RecruitPhasePlayer)
            );
        }

        bytes
    }
}

impl Default for RecruitPhasePlayer {
    fn default() -> Self {
        Self {
            account: UserAccount::default(),
            bitfield: 0x00,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{UserId, UserName};

    use super::*;

    #[test]
    fn test_recruit_phase_player() {
        let id = UserId::new(1314311);
        let name = UserName::from_str("Aris");
        let account = UserAccount::new(id, name);
        let origin = RecruitPhasePlayer::new(account, Team::Red, false, Permission::Admin);
        let bytes = origin.to_big_endian_bytes();
        let other = RecruitPhasePlayer::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
