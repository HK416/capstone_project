//! 게임 로그인 성공 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{
        BigEndian, CharacterKind, GameTier, LoginToken, TryFromBigEndian, UserId, UserName,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 로그인 성공 패킷에서 사용되는 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - tier                  | 2bit | 게임 티어
/// - use_profile_character | 1bit | 프로필 캐릭터 설정 여부
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const TIER_BIT_MASK: u8 = 0x03;
    const TIER_SHIFT: usize = 0;
    const PROFILE_BIT_MASK: u8 = 0x01;
    const PROFILE_SHIFT: usize = 2;

    /// 새로운 비트 필드 데이터를 생성합니다.
    const fn new() -> Self {
        Self(0x00)
    }

    /// 프로필 캐릭터 설정 여부를 반환합니다.
    fn use_profile_character(&self) -> bool {
        (self.0 >> Self::PROFILE_SHIFT) & Self::PROFILE_BIT_MASK == Self::PROFILE_BIT_MASK
    }

    /// 프로필 캐릭터 설정 여부를 설정합니다.
    fn with_use_profile_character(mut self, use_profile_character: bool) -> Self {
        self.0 &= !(Self::PROFILE_BIT_MASK << Self::PROFILE_SHIFT);
        self.0 |= ((use_profile_character as u8) & Self::PROFILE_BIT_MASK) << Self::PROFILE_SHIFT;
        self
    }

    /// 게임 티어를 반환합니다.
    fn tier(&self) -> GameTier {
        let val = (self.0 >> Self::TIER_SHIFT) & Self::TIER_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { GameTier::new(val).unwrap_unchecked() }
    }

    /// 게임 티어를 설정합니다.
    fn with_tier(mut self, tier: GameTier) -> Self {
        self.0 &= !(Self::TIER_BIT_MASK << Self::TIER_SHIFT);
        self.0 |= ((tier as u8) & Self::TIER_BIT_MASK) << Self::TIER_SHIFT;
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

/// 서버에서 클라이언트로 보내는 로그인 성공 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSuccessPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub name: UserName,
    /// 사용자 대표 캐릭터 종류
    character_kind: CharacterKind,
    /// 비트 필드 데이터입니다.
    bitfield: Bitfield,
    /// 로그인 토큰
    pub token: LoginToken,
}

impl LoginSuccessPacket {
    /// 새로운 로그인 성공 패킷을 생성합니다.
    pub const fn new(uid: UserId, name: UserName, token: LoginToken) -> Self {
        Self {
            uid,
            name,
            character_kind: CharacterKind::ArisOriginal,
            bitfield: Bitfield::new(),
            token,
        }
    }

    /// 프로필 캐릭터 종류를 반환합니다.
    pub fn profile_character(&self) -> Option<CharacterKind> {
        self.bitfield
            .use_profile_character()
            .then_some(self.character_kind)
    }

    /// 프로필 캐릭터 종류를 설정합니다.
    pub fn set_character_kind(&mut self, character_kind: CharacterKind) {
        self.bitfield = self.bitfield.with_use_profile_character(true);
        self.character_kind = character_kind;
    }

    /// 프로필 캐릭터 종류를 설정합니다.
    pub fn with_character_kind(mut self, character_kind: CharacterKind) -> Self {
        self.bitfield = self.bitfield.with_use_profile_character(true);
        self.character_kind = character_kind;
        self
    }

    /// 게임 티어를 반환합니다.
    pub fn tier(&self) -> GameTier {
        self.bitfield.tier()
    }

    /// 게임 티어를 설정합니다.
    pub fn set_tier(&mut self, tier: GameTier) {
        self.bitfield = self.bitfield.with_tier(tier);
    }

    /// 게임 티어를 설정합니다.
    pub fn with_tier(mut self, tier: GameTier) -> Self {
        self.bitfield = self.bitfield.with_tier(tier);
        self
    }
}

impl Packet for LoginSuccessPacket {
    fn packet_type() -> PacketType {
        PacketType::LoginSuccess
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size()
            + UserName::byte_size()
            + CharacterKind::byte_size()
            + Bitfield::byte_size()
            + LoginToken::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.name.to_big_endian_bytes());
        data.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        data.extend_from_slice(&self.bitfield.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LoginSuccessPacket)
            )
        };

        RawPacket::new(Self::packet_type(), &data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type. (SRC:{:?}, DST:{:?})",
                raw.packet_type(),
                Self::packet_type()
            );
            return None;
        }

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 사용자 이름을 가져옵니다.
        offset = offset + size;
        size = UserName::byte_size();
        data = &bytes[offset..offset + size];
        let name = UserName::from_big_endian_bytes(data);

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

        // 로그인 토큰 데이터를 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self {
            uid,
            name,
            character_kind,
            bitfield,
            token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitfield_use_profile_character() {
        let bitfield = Bitfield::new().with_use_profile_character(false);
        assert_eq!(false, bitfield.use_profile_character());

        let bitfield = Bitfield::new().with_use_profile_character(true);
        assert_eq!(true, bitfield.use_profile_character());
    }

    #[test]
    fn test_bitfield_tier() {
        let val = GameTier::Bronze;
        let bitfield = Bitfield::new().with_tier(val);
        assert_eq!(GameTier::Bronze, bitfield.tier());

        let val = GameTier::Silver;
        let bitfield = Bitfield::new().with_tier(val);
        assert_eq!(GameTier::Silver, bitfield.tier());

        let val = GameTier::Gold;
        let bitfield = Bitfield::new().with_tier(val);
        assert_eq!(GameTier::Gold, bitfield.tier());

        let val = GameTier::Platinum;
        let bitfield = Bitfield::new().with_tier(val);
        assert_eq!(GameTier::Platinum, bitfield.tier());
    }

    #[test]
    fn test_login_success_packet() {
        let origin = LoginSuccessPacket::new(
            UserId::new(12345),
            UserName::from_str("유우카"),
            LoginToken::new(1351616161),
        )
        .with_character_kind(CharacterKind::YuukaOriginal)
        .with_tier(GameTier::Bronze);
        let raw = origin.as_raw();
        let other = LoginSuccessPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
