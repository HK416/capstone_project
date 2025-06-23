//! 각 팀의 캐릭터를 편성하는 단계에 진입할 때 플레이어 데이터 갱신과 관련된 코드를 관리합니다.
//!

use crate::components::{
    BigEndian, CharacterKind, NetworkState, Permission, TryFromBigEndian, UserId,
};

/// 플레이어 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - connected     | 1bit | 서버 연결 여부
/// - selected      | 1bit | 캐릭터 선택 여부
/// - network_state | 2bit | 네트워크 상태
/// - permission    | 1bit | 권한
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 0;
    const SELECT_BIT_MASK: u8 = 0x01;
    const SELECT_SHIFT: usize = 1;
    const STATE_BIT_MASK: u8 = 0x03;
    const STATE_SHIFT: usize = 2;
    const PERMISSION_BIT_MASK: u8 = 0x01;
    const PERMISSION_SHIFT: usize = 4;

    /// 새로운 비트 필드 데이터를 생성합니다.
    const fn new() -> Self {
        Self(0x00)
    }

    /// 서버 연결 여부를 반환합니다.
    fn is_connected(&self) -> bool {
        (self.0 >> Self::CONNECT_SHIFT) & Self::CONNECT_BIT_MASK == Self::CONNECT_BIT_MASK
    }

    /// 서버 연결 여부를 설정합니다.
    fn with_connected(mut self, connected: bool) -> Self {
        self.0 &= !(Self::CONNECT_BIT_MASK << Self::CONNECT_SHIFT);
        self.0 |= ((connected as u8) & Self::CONNECT_BIT_MASK) << Self::CONNECT_SHIFT;
        self
    }

    /// 캐릭터 선택 여부를 반환합니다.
    fn is_selected(&self) -> bool {
        (self.0 >> Self::SELECT_SHIFT) & Self::SELECT_BIT_MASK == Self::SELECT_BIT_MASK
    }

    /// 캐릭터 선택 여부를 설정합니다.
    fn with_selected(mut self, selected: bool) -> Self {
        self.0 &= !(Self::SELECT_BIT_MASK << Self::SELECT_SHIFT);
        self.0 |= ((selected as u8) & Self::SELECT_BIT_MASK) << Self::SELECT_SHIFT;
        self
    }

    /// 네트워크 상태를 반환합니다.
    fn network_state(&self) -> NetworkState {
        let val = (self.0 >> Self::STATE_SHIFT) & Self::STATE_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { NetworkState::new(val).unwrap_unchecked() }
    }

    /// 네트워크 상태를 설정합니다.
    fn with_network_state(mut self, state: NetworkState) -> Self {
        self.0 &= !(Self::STATE_BIT_MASK << Self::STATE_SHIFT);
        self.0 |= ((state as u8) & Self::STATE_BIT_MASK) << Self::STATE_SHIFT;
        self
    }

    /// 권한을 반환합니다.
    fn permission(&self) -> Permission {
        let val = (self.0 >> Self::PERMISSION_SHIFT) & Self::PERMISSION_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { Permission::new(val).unwrap_unchecked() }
    }

    /// 권한을 설정합니다.
    fn with_permission(mut self, permission: Permission) -> Self {
        self.0 &= !(Self::PERMISSION_BIT_MASK << Self::PERMISSION_SHIFT);
        self.0 |= ((permission as u8) & Self::PERMISSION_BIT_MASK) << Self::PERMISSION_SHIFT;
        self
    }
}

impl BigEndian for Bitfield {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u8::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for Bitfield {
    fn default() -> Self {
        Self(0x00)
    }
}

/// 캐릭터 편성 단계에서 사용되는 플레이어 갱신 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationPlayerUpdateData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 캐릭터 종류
    character_kind: CharacterKind,
    /// 비트 필드 데이터
    bitfield: Bitfield,
}

impl FormationPlayerUpdateData {
    /// 새로운 캐릭터 편성 단계 플레이어 갱신 데이터를 생성합니다.
    pub fn new(
        uid: UserId,
        connected: bool,
        permission: Permission,
        network_state: NetworkState,
        character_kind: Option<CharacterKind>,
    ) -> Self {
        Self {
            uid,
            character_kind: character_kind.unwrap_or_default(),
            bitfield: Bitfield::new()
                .with_connected(connected)
                .with_permission(permission)
                .with_network_state(network_state)
                .with_selected(character_kind.is_some()),
        }
    }

    /// 서버 연결 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        self.bitfield.is_connected()
    }

    /// 선택한 캐릭터를 반환합니다.
    pub fn character_kind(&self) -> Option<CharacterKind> {
        self.bitfield.is_selected().then_some(self.character_kind)
    }

    /// 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        self.bitfield.network_state()
    }

    /// 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        self.bitfield.permission()
    }
}

impl BigEndian for FormationPlayerUpdateData {
    fn byte_size() -> usize {
        UserId::byte_size() + CharacterKind::byte_size() + Bitfield::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 생성된 바이트가 유효한지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PullFormationPlayerData),
            )
        };

        bytes
    }
}

impl TryFromBigEndian for FormationPlayerUpdateData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 주어진 바이트가 유효한지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PullFormationPlayerData),
            )
        };

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        Some(Self {
            uid,
            character_kind,
            bitfield,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitfield_selected() {
        let bitfield = Bitfield::new().with_selected(false);
        assert_eq!(false, bitfield.is_selected());

        let bitfield = Bitfield::new().with_selected(true);
        assert_eq!(true, bitfield.is_selected());
    }

    #[test]
    fn test_bitfield_network_state() {
        let state = NetworkState::Critical;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(NetworkState::Critical, bitfield.network_state());

        let state = NetworkState::Poor;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(NetworkState::Poor, bitfield.network_state());

        let state = NetworkState::Fair;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(NetworkState::Fair, bitfield.network_state());

        let state = NetworkState::Good;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(NetworkState::Good, bitfield.network_state());
    }

    #[test]
    fn test_bitfield_permission() {
        let val = Permission::User;
        let bitfield = Bitfield::new().with_permission(val);
        assert_eq!(Permission::User, bitfield.permission());

        let val = Permission::Admin;
        let bitfield = Bitfield::new().with_permission(val);
        assert_eq!(Permission::Admin, bitfield.permission());
    }

    #[test]
    fn test_formation_player_pull_data() {
        let origin = FormationPlayerUpdateData::new(
            UserId::new(1234515),
            true,
            Permission::Admin,
            NetworkState::Fair,
            Some(CharacterKind::YuukaOriginal),
        );
        let bytes = origin.to_big_endian_bytes();
        let other = FormationPlayerUpdateData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 비교
        assert_eq!(origin, other);
    }
}
