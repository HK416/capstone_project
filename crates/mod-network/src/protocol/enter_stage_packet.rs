use super::PacketType;
use super::RawPacket;
use super::super::components::ClientId;
use super::super::components::CharacterKind;
use super::super::components::BigEndian;


#[derive(Debug, PartialEq)]
pub struct EnterStagePacket {
    pub client_id: ClientId,
    pub character_kind: CharacterKind,
}

impl EnterStagePacket {
    pub fn new(client_id: ClientId, character_kind: CharacterKind) -> Self {
        Self {
            client_id,
            character_kind,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let client_id = ClientId::from_big_endian_bytes(&raw.data());
        let character_kind = CharacterKind::from_big_endian_bytes(&raw.data()[size_of::<ClientId>()..]);

        Self {
            client_id,
            character_kind,
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        let mut bytes = Vec::with_capacity(size_of::<ClientId>() + size_of::<CharacterKind>());
        bytes.extend_from_slice(&self.client_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());

        RawPacket::new(PacketType::ENTERSTAGE, &bytes)
    }
}