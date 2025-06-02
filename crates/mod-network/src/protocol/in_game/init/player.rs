//! 서버에서 클라이언트로 보내는 스테이지 로드 요청 패킷에서
//! 사용되는 플레이어 데이터와 관련된 코드를 관리합니다.
//!

use crate::components::{
    BigEndian, CharacterKind, HealthData, Permission, SkillCostData, Team, TryFromBigEndian,
    UserId, UserName,
};

/// 초기화 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// 이름                 | 비트수 | 설명
/// team                | 1bit | 팀 정보
/// team_index          | 3bit | 팀 내의 인덱스 번호
/// permission          | 1bit | 유저 권한
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InitBitfield(u8);

impl InitBitfield {
    const TEAM_BIT_MASK: u8 = 0x1;
    const TEAM_SHIFT: usize = 0;
    const INDEX_BIT_MASK: u8 = 0x7;
    const INDEX_SHIFT: usize = 1;
    const PERMISSION_BIT_MASK: u8 = 0x1;
    const PERMISSION_SHIFT: usize = 4;

    /// 새로운 초기화 비트 필드 데이터 생성합니다.
    pub const fn new() -> Self {
        Self(0x00)
    }

    /// 팀을 반환합니다.
    pub fn team(&self) -> Team {
        let val = (self.0 >> Self::TEAM_SHIFT) & Self::TEAM_BIT_MASK;
        Team::new(val).unwrap_or_default()
    }

    /// 팀을 설정합니다.
    pub fn with_team(mut self, team: Team) -> Self {
        self.0 &= !(Self::TEAM_BIT_MASK << Self::TEAM_SHIFT); // 기존 값 지우기
        self.0 |= (team as u8) << Self::TEAM_SHIFT; // 값 덮어쓰기
        self
    }

    /// 인덱스를 반환합니다.
    pub fn index(&self) -> usize {
        ((self.0 >> Self::INDEX_SHIFT) & Self::INDEX_BIT_MASK) as usize
    }

    /// 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 `index`가 4를 초과하는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn with_index(mut self, index: usize) -> Self {
        assert!(index < 5, "index out of ranges!");
        self.0 &= !(Self::INDEX_BIT_MASK << Self::INDEX_SHIFT); // 기존 값 지우기
        self.0 |= (index as u8) << Self::INDEX_SHIFT; // 값 덮어쓰기
        self
    }

    /// 권한을 가져옵니다.
    pub fn permission(&self) -> Permission {
        let val = (self.0 >> Self::PERMISSION_SHIFT) & Self::PERMISSION_BIT_MASK;
        Permission::new(val).unwrap_or_default()
    }

    /// 권한을 설정합니다.
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.0 &= !(Self::PERMISSION_BIT_MASK << Self::PERMISSION_SHIFT); // 기존 값 지우기
        self.0 |= (permission as u8) << Self::PERMISSION_SHIFT; // 값 덮어쓰기
        self
    }
}

impl BigEndian for InitBitfield {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u8::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for InitBitfield {
    fn default() -> Self {
        Self(0x00)
    }
}

/// 플레이어 초기화 데이터입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSetupData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub user_name: UserName,
    /// 초기화 데이터
    pub setup_data: InitBitfield,
    /// 체력 데이터
    pub health: HealthData,
    /// 스킬 코스트 데이터
    pub skill_cost: SkillCostData,
    /// 캐릭터 종류
    pub character_kind: CharacterKind,
    /// 월드 공간 위치
    pub translation: [f32; 3],
    /// 월드 공간 방향
    pub rotation: [f32; 4],
}

impl BigEndian for PlayerSetupData {
    fn byte_size() -> usize {
        UserId::byte_size()
            + UserName::byte_size()
            + InitBitfield::byte_size()
            + HealthData::byte_size()
            + SkillCostData::byte_size()
            + CharacterKind::byte_size()
            + <[f32; 3]>::byte_size()
            + <[f32; 4]>::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.user_name.to_big_endian_bytes());
        bytes.extend_from_slice(&self.setup_data.to_big_endian_bytes());
        bytes.extend_from_slice(&self.health.to_big_endian_bytes());
        bytes.extend_from_slice(&self.skill_cost.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PlayerSetupData)
            );
        }

        bytes
    }
}

impl Default for PlayerSetupData {
    fn default() -> Self {
        Self {
            uid: UserId::default(),
            user_name: UserName::default(),
            setup_data: InitBitfield::default(),
            health: HealthData::default(),
            skill_cost: SkillCostData::default(),
            character_kind: CharacterKind::default(),
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

impl TryFromBigEndian for PlayerSetupData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기를 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PlayerSetupData)
            )
        };

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 사용자 이름을 가져옵니다.
        offset = offset + size;
        size = UserName::byte_size();
        data = &bytes[offset..offset + size];
        let user_name = UserName::from_big_endian_bytes(data);

        // 초기화 데이터를 가져옵니다.
        offset = offset + size;
        size = InitBitfield::byte_size();
        data = &bytes[offset..offset + size];
        let setup_data = InitBitfield::from_big_endian_bytes(data);

        // 체력 데이터를 가져옵니다.
        offset = offset + size;
        size = HealthData::byte_size();
        data = &bytes[offset..offset + size];
        let health = HealthData::try_from_big_endian_bytes(data)?;

        // 스킬 코스트 데이터를 가져옵니다.
        offset = offset + size;
        size = SkillCostData::byte_size();
        data = &bytes[offset..offset + size];
        let skill_cost = SkillCostData::try_from_big_endian_bytes(data)?;

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        // 월드 공간 위치를 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let translation = <[f32; 3]>::from_big_endian_bytes(data);

        // 월드 공간 방향을 가져옵니다.
        offset = offset + size;
        size = <[f32; 4]>::byte_size();
        data = &bytes[offset..offset + size];
        let rotation = <[f32; 4]>::from_big_endian_bytes(data);

        Some(Self {
            uid,
            user_name,
            setup_data,
            health,
            skill_cost,
            character_kind,
            translation,
            rotation,
        })
    }
}
