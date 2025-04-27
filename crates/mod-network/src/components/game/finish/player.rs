use crate::components::{BigEndian, CharacterKind, Team, TryFromBigEndian, UserAccount};

/// 게임이 끝나고 결과를 보여줄 때 플레이어 데이터를 저장합니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishPhasePlayer {
    /// 사용자 계정 데이터입니다.
    pub account: UserAccount,
    /// 선택한 캐릭터 종류입니다.
    pub character_kind: CharacterKind,

    /// 상대 팀을 처치한 횟수입니다.
    pub kill_count: u16,
    /// 상대 팀에게 처치당한 횟수입니다.
    pub dead_count: u16,

    /// 상대 팀에게 입힌 총 데미지입니다.
    pub damage_dealt: u32,
    /// 상대 팀에게 입은 총 데미지입니다.
    pub damage_taken: u32,
    /// 같은 팀에게 회복 시킨 회복량입니다.
    pub healing_given: u32,

    /// 여러 자료형의 데이터를 저장한 비트 필드입니다.  
    /// 아래 데이터가 포함됩니다.
    /// - Team (1bit): 플레이어가 속한 팀의 종류를 나타냅니다.
    /// - index (3bit): 플레이어가 속한 팀 내의 인덱스 번호 (결과 창에서 플레이어 위치를 결정함)
    /// - bool (1bit): 플레이어가 게임 도중 연결이 끊어졌는지 여부
    pub bitfield: u8,
}

impl FinishPhasePlayer {
    /// 새로운 플레이어 데이터를 생성합니다.  
    /// 주어진 인덱스가 4보다 클 경우 [`panic!`]을 호출합니다.
    pub fn new(
        account: UserAccount,
        character_kind: CharacterKind,
        kill_count: u16,
        dead_count: u16,
        damage_dealt: u32,
        damage_taken: u32,
        healing_given: u32,
        team: Team,
        team_index: usize,
        connected: bool,
    ) -> Self {
        assert!(team_index < 5, "index out of range!");

        let team_bit = ((team as u8) & 0x1) << 0;
        let team_index_bit = ((team_index as u8) & 0x7) << 1;
        let connected_bit = ((connected as u8) & 0x1) << 4;
        let bitfield = team_bit | team_index_bit | connected_bit;

        Self {
            account,
            character_kind,
            kill_count,
            dead_count,
            damage_dealt,
            damage_taken,
            healing_given,
            bitfield,
        }
    }

    /// 플레이어가 속한 팀을 설정합니다.
    pub fn with_team(&mut self, team: Team) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 0)) | ((team as u8) & 0x1) << 0;
        self
    }

    /// 플레이어가 속한 팀 정보를 가져옵니다.
    pub fn team(&self) -> Team {
        // Safe: 값이 범위를 벗어나지 않음
        unsafe { Team::new((self.bitfield >> 0) & 0x1).unwrap_unchecked() }
    }

    /// 플레이어가 속한 팀 내의 인덱스를 설정합니다.
    pub fn with_team_index(&mut self, team_index: usize) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x7 << 1)) | ((team_index as u8) & 0x7) << 1;
        self
    }

    /// 플레이어가 속한 팀 내의 플레이어 인덱스를 가져옵니다.
    pub fn index(&self) -> usize {
        ((self.bitfield >> 1) & 0x7) as usize
    }

    /// 플레이어 인게임 연결 여부를 설정합니다.
    pub fn with_connected(&mut self, connected: bool) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 4)) | ((connected as u8) & 0x1) << 4;
        self
    }

    /// 플레이어 인게임 연결 여부를 반환합니다.
    pub fn connected(&self) -> bool {
        (self.bitfield >> 4) & 0x1 == 0x1
    }
}

impl BigEndian for FinishPhasePlayer {
    fn byte_size() -> usize {
        UserAccount::byte_size()
            + CharacterKind::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u32::byte_size()
            + u32::byte_size()
            + u32::byte_size()
            + u8::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.account.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.kill_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.dead_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_dealt.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_taken.to_big_endian_bytes());
        bytes.extend_from_slice(&self.healing_given.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FinishPhasePlayer),
            );
        }

        bytes
    }
}

impl Default for FinishPhasePlayer {
    fn default() -> Self {
        Self {
            account: UserAccount::default(),
            character_kind: CharacterKind::default(),
            kill_count: 0,
            dead_count: 0,
            damage_dealt: 0,
            damage_taken: 0,
            healing_given: 0,
            bitfield: 0x00,
        }
    }
}

impl TryFromBigEndian for FinishPhasePlayer {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(FinishPhasePlayer),
        );

        // 사용자 계정 데이터를 가져옵니다.
        let mut offset = 0;
        let mut size = UserAccount::byte_size();
        let mut data = &bytes[offset..offset + size];
        let account = UserAccount::from_big_endian_bytes(data);

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        // 상대 팀을 처치한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let kill_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 처치당한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let dead_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 입힌 데미지량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let damage_dealt = u32::from_big_endian_bytes(data);

        // 상대 팀에게 입은 데미지량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let damage_taken = u32::from_big_endian_bytes(data);

        // 같은 팀을 회복시킨 회복량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let healing_given = u32::from_big_endian_bytes(data);

        // 비트 필드를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = u8::from_big_endian_bytes(data);

        Some(Self {
            account,
            character_kind,
            kill_count,
            dead_count,
            damage_dealt,
            damage_taken,
            healing_given,
            bitfield,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{UserId, UserName};

    use super::*;

    #[test]
    fn test_finish_phase_player() {
        let account = UserAccount::new(UserId::new(1234135), UserName::from_str("Aris"));
        let character_kind = CharacterKind::ArisOriginal;
        let kill_count = 30;
        let dead_count = 20;
        let damage_dealt = 2000;
        let damage_taken = 500;
        let healing_given = 0;
        let team = Team::Red;
        let index = 2;

        let origin = FinishPhasePlayer::new(
            account,
            character_kind,
            kill_count,
            dead_count,
            damage_dealt,
            damage_taken,
            healing_given,
            team,
            index,
            true,
        );
        let bytes = origin.to_big_endian_bytes();
        let other = FinishPhasePlayer::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
