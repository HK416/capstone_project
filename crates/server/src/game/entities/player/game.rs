use mod_network::components::{HealthPoint, Permission, Team};

/// 게임 플레이 데이터 속성을 저장합니다.
#[derive(Debug, Clone)]
pub struct GameComponent {
    /// 플레이어 캐릭터의 체력입니다.
    pub health_point: HealthPoint,

    /// 여러 자료형의 데이터를 저장한 비트 필드입니다.  
    /// 아래 데이터가 포함됩니다.
    /// - Team (1bit): 플레이어가 속한 팀의 종류를 나타냄
    /// - bool (1bit): 플레이어가 준비되었는지 여부를 나타냄
    /// - Permission (1bit): 플레이어의 권한 정보를 나타냄
    ///
    pub bitfield: u8,

    /// 한 공격당 총알의 발사 횟수입니다.
    pub fired_per_attack: u8,
    /// 남은 총알의 개수입니다.
    pub num_remaining_bullets: u8,
}

impl GameComponent {
    /// 플레이어가 속한 팀을 설정합니다.
    pub fn with_team(&mut self, team: Team) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 2)) | (team as u8) << 2;
        self
    }

    /// 플레이어가 속한 팀을 반환합니다.
    pub fn team(&self) -> Team {
        // Safe: 값이 범위를 벗어나지 않음
        unsafe { Team::new((self.bitfield >> 2) & 0x1).unwrap_unchecked() }
    }

    /// 준비 여부를 설정합니다.
    pub fn with_ready(&mut self, ready: bool) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 1)) | (ready as u8) << 1;
        self
    }

    /// 준비 여부를 반환합니다.
    pub fn is_ready(&self) -> bool {
        (self.bitfield >> 1) & 0x1 == 0x1
    }

    /// 플레이어 권한을 설정합니다.
    pub fn with_permission(&mut self, permission: Permission) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 0)) | (permission as u8) << 0;
        self
    }

    /// 플레이어 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        // Safe: 값이 범위를 벗어나지 않음
        unsafe { Permission::new((self.bitfield >> 0) & 0x1).unwrap_unchecked() }
    }
}

impl Default for GameComponent {
    fn default() -> Self {
        Self {
            health_point: HealthPoint::default(),
            bitfield: 0,
            fired_per_attack: 0,
            num_remaining_bullets: 0,
        }
    }
}
