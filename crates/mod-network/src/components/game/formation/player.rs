use crate::components::{BigEndian, CharacterKind, Team, TryFromBigEndian, UserAccount};

/// 각 팀의 캐릭터를 편성할 떄 플레이어 데이터를 저장합니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationPhasePlayer {
    /// 사용자 계정 데이터
    pub account: UserAccount,

    /// 선택한 캐릭터의 종류  
    /// `None`을 바이트 스트림으로 변환할 때 `0xFF`로 저장됩니다.
    pub character_kind: Option<CharacterKind>,

    /// 여러 자료형의 데이터를 저장한 비트 필드입니다.  
    /// 아래 데이터가 포함됩니다.
    /// - bool (1bit): 플레이어가 준비되었는지 여부를 나타냄
    /// - Team (1bit): 플레이어가 속한 팀의 종류를 나타냄
    ///
    pub bitfield: u8,
}

// bitfield 구조
// +------+--------------+-------------+
// | 6bit | ready (1bit) | team (1bit) |
// +------+--------------+-------------+
//
impl FormationPhasePlayer {
    /// 새로운 플레이어 데이터를 생성합니다.
    pub fn new(
        account: UserAccount,
        character_kind: Option<CharacterKind>,
        ready: bool,
        team: Team,
    ) -> Self {
        let ready_field = (ready as u8) << 1;
        let team_field = (team as u8) << 0;

        Self {
            account,
            character_kind,
            bitfield: ready_field | team_field,
        }
    }

    /// 캐릭터 종류를 설정합니다.
    pub fn with_character(&mut self, character_kind: Option<CharacterKind>) -> &mut Self {
        self.character_kind = character_kind;
        self
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

    /// Team을 설정합니다.
    pub fn with_team(&mut self, team: Team) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 0)) | (team as u8) << 0;
        self
    }

    /// Team을 가져옵니다.
    pub fn team(&self) -> Team {
        // Safe: 값이 범위를 벗어나지 않음
        unsafe { Team::new((self.bitfield >> 0) & 0x1).unwrap_unchecked() }
    }
}

impl BigEndian for FormationPhasePlayer {
    fn byte_size() -> usize {
        UserAccount::byte_size() + CharacterKind::byte_size() + u8::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(FormationPhasePlayer)
        );

        // 사용자 계정 데이터를 가져옵니다.
        let mut offset = 0;
        let mut size = UserAccount::byte_size();
        let mut data = &bytes[offset..offset + size];
        let account = UserAccount::from_big_endian_bytes(data);

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data);

        // 비트 필드를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = u8::from_big_endian_bytes(data);

        Self {
            account,
            character_kind,
            bitfield,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.account.to_big_endian_bytes());
        bytes.extend_from_slice(&match self.character_kind {
            Some(kind) => kind.to_big_endian_bytes(),
            None => (0xFF as u8).to_big_endian_bytes(),
        });
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FormationPhasePlayer)
            );
        }

        bytes
    }
}

impl Default for FormationPhasePlayer {
    fn default() -> Self {
        Self {
            account: UserAccount::default(),
            character_kind: None,
            bitfield: 0x00,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{UserId, UserName};

    use super::*;

    #[test]
    fn test_formation_phase_player() {
        let id = UserId::new(1314311);
        let name = UserName::from_str("Aris");
        let account = UserAccount::new(id, name);
        let origin = FormationPhasePlayer::new(account, None, true, Team::Blue);
        let bytes = origin.to_big_endian_bytes();
        let other = FormationPhasePlayer::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
