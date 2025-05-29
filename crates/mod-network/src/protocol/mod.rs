mod formation;
mod in_game;
mod lobby;
mod room;
mod title;
mod ping;

mod parser;

use std::io::{Error, ErrorKind};

use crate::components::{BigEndian, TryFromBigEndian};

pub use self::{formation::*, in_game::*, lobby::*, parser::*, room::*, title::*, ping::*};

/// 패킷의 종류
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PacketType {
    Raw = 0,
    Connect = 5,

    ClientVerify = 1,
    LoginRequest = 2,
    LoginFailed = 3,
    LoginSuccess = 4,

    LobbyPull = 8,

    RequestAvailableWorlds = 9,
    AvailableWorlds = 10,

    /// 커스텀 게임 참여 또는 생성하기 위해 클라이언트에서 서버로 보내는 패킷
    CustomGameJoinRequest = 24,
    /// 커스텀 게임 참여 실패 사유를 서버에서 클라이언트로 보내는 패킷
    CustomGameJoinFailed = 25,
    /// 커스텀 게임 참여 성공을 서버에서 클라이언트로 보내는 패킷
    CustomGameJoinSuccess = 26,
    /// 매번 커스텀 게임 데이터를 서버에서 클라이언트로 보내는 패킷
    CustomGamePull = 27,
    CustomGameLeave = 28,
    CustomGameReady = 29,
    CustomGameStartFailed = 30,

    FormationSelect = 32,
    FormationSelectResponse = 33,
    FormationPull = 34,
    GamePlayStop = 35,

    /// 게임 시작 전에 대기 상태에서 서버에서 클라이언트로 전송되는 패킷
    PrepareStage = 48,
    InitStage = 49,
    PullStage = 50,
    PushStatus = 51,
    PushSync = 52,

    /// 반응속도 측정을 위한 패킷  
    /// 서버에서 수신시 그대로 클라이언트에 전송(echo)  
    Ping = 53,

    FinishStage = 64,
    FinishStageResponse = 65,

    UdpDamageLog = 128,
}

impl BigEndian for PacketType {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for PacketType {
    fn default() -> Self {
        Self::Raw
    }
}

impl TryFromBigEndian for PacketType {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(PacketType::Raw),
            1 => Some(PacketType::ClientVerify),
            2 => Some(PacketType::LoginRequest),
            3 => Some(PacketType::LoginFailed),
            4 => Some(PacketType::LoginSuccess),
            5 => Some(PacketType::Connect),
            8 => Some(PacketType::LobbyPull),
            9 => Some(PacketType::RequestAvailableWorlds),
            10 => Some(PacketType::AvailableWorlds),
            24 => Some(PacketType::CustomGameJoinRequest),
            25 => Some(PacketType::CustomGameJoinFailed),
            26 => Some(PacketType::CustomGameJoinSuccess),
            27 => Some(PacketType::CustomGamePull),
            28 => Some(PacketType::CustomGameLeave),
            29 => Some(PacketType::CustomGameReady),
            30 => Some(PacketType::CustomGameStartFailed),
            32 => Some(PacketType::FormationSelect),
            33 => Some(PacketType::FormationSelectResponse),
            34 => Some(PacketType::FormationPull),
            35 => Some(PacketType::GamePlayStop),
            48 => Some(PacketType::PrepareStage),
            49 => Some(PacketType::InitStage),
            50 => Some(PacketType::PullStage),
            51 => Some(PacketType::PushStatus),
            52 => Some(PacketType::PushSync),
            53 => Some(PacketType::Ping),
            64 => Some(PacketType::FinishStage),
            65 => Some(PacketType::FinishStageResponse),
            128 => Some(PacketType::UdpDamageLog),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(PacketType),
                    index
                );
                None
            }
        }
    }
}

pub type PacketSize = u16;

/// 고정된 크기를 갖는 패킷의 헤더
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketHeader {
    pub packet_size: PacketSize,
    pub packet_type: PacketType,
}

impl BigEndian for PacketHeader {
    fn byte_size() -> usize {
        PacketSize::byte_size() + PacketType::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.packet_size.to_big_endian_bytes());
        bytes.extend_from_slice(&self.packet_type.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PacketHeader)
            );
        }

        bytes
    }
}

impl Default for PacketHeader {
    fn default() -> Self {
        Self {
            packet_size: 0,
            packet_type: PacketType::default(),
        }
    }
}

impl TryFromBigEndian for PacketHeader {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(PacketHeader)
        );

        // 패킷의 크기를 가져옵니다.
        let mut offset = 0;
        let mut size = PacketSize::byte_size();
        let mut data = &bytes[offset..offset + size];
        let packet_size = PacketSize::from_big_endian_bytes(data);

        // 패킷 종류를 가져옵니다.
        offset = offset + size;
        size = PacketType::byte_size();
        data = &bytes[offset..offset + size];
        let packet_type = PacketType::try_from_big_endian_bytes(data)?;

        Some(Self {
            packet_size,
            packet_type,
        })
    }
}

/// 클라이언트-서버 간의 통신에 사용되는 기본 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPacket {
    header: PacketHeader,
    data: Vec<u8>,
}

impl RawPacket {
    pub fn new(packet_type: PacketType, data: &[u8]) -> Self {
        let data = data.to_vec();
        let packet_size = (PacketHeader::byte_size() + data.len()) as u16;
        let header = PacketHeader {
            packet_type,
            packet_size,
        };
        Self { header, data }
    }

    pub fn packet_type(&self) -> PacketType {
        self.header.packet_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let packet_size = self.header.packet_size as usize;
        let mut bytes = Vec::with_capacity(packet_size);
        bytes.extend_from_slice(&self.header.to_big_endian_bytes());
        bytes.extend_from_slice(&self.data);

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                packet_size,
                "the size of the byte array and the size of the packet are different!"
            );
        }

        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // 바이트 배열의 크기가 패킷 헤더의 크기보다 작은지 확인한다.
        let header_size = PacketHeader::byte_size();
        if bytes.len() < header_size {
            log::warn!(
                "the size of the byte array is smaller than `{}`.",
                stringify!(PacketHeader)
            );
            return Err(Error::new(ErrorKind::InvalidData, "invalid data"));
        }

        // 패킷 헤더를 가져옵니다.
        let result = PacketHeader::try_from_big_endian_bytes(&bytes[0..header_size]);
        let header = match result {
            Some(header) => header,
            None => return Err(Error::new(ErrorKind::InvalidData, "invalid data")),
        };

        // 바이트 배열의 크기가 패킷의 크기보다 작은지 확인한다.
        let packet_size = header.packet_size as usize;
        if bytes.len() < packet_size {
            log::warn!("the size of the byte array is smaller than the size of the packet");
            return Err(Error::new(ErrorKind::InvalidData, "invalid data"));
        }

        // 데이터를 가져옵니다.
        let data = bytes[header_size..packet_size].to_vec();

        Ok(Self { header, data })
    }
}

/// 모든 파생 패킷이 구현해야 하는 `triat`입니다.
pub trait Packet: Sized {
    /// 파생 패킷의 타입입니다.
    fn packet_type() -> PacketType;

    /// 패킷을 `RawPacket`으로 변환합니다.
    fn as_raw(&self) -> RawPacket;

    /// `RawPacket`으로부터 패킷을 생성합니다.
    ///
    /// # Panics
    /// 패킷 종류가 다르거나, 패킷을 생성할 수 없는 경우 [`panic!`]을 호출합니다.
    ///
    fn from_raw(raw: RawPacket) -> Self {
        Self::try_from_raw(raw).expect("invalid data")
    }

    /// `RawPacket`으로부터 패킷을 생성합니다.
    ///
    /// 패킷 종류가 다르거나, 패킷을 생성할 수 없는 경우 `None`을 반환합니다.
    ///
    fn try_from_raw(raw: RawPacket) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_packet_type() {
        let origin = PacketType::InitStage;
        let bytes = origin.to_big_endian_bytes();
        let other = PacketType::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(PacketType::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_packet_header() {
        let origin = PacketHeader {
            packet_size: 65524,
            packet_type: PacketType::PullStage,
        };
        let bytes = origin.to_big_endian_bytes();
        let other = PacketHeader::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(PacketHeader::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_raw_packet() {
        let data = vec![1, 2, 3, 4, 5];
        let origin = RawPacket::new(PacketType::Raw, &data);
        let bytes = origin.as_bytes();
        let other = RawPacket::try_from_bytes(&bytes).unwrap();

        // 바이트 배열이 Big-endian인지 확인합니다.
        assert_eq!(bytes, vec![0, 8, 0, 1, 2, 3, 4, 5]);
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
