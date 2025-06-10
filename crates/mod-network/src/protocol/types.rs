//! 패킷의 종류와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 패킷의 종류
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PacketType {
    #[default]
    Raw = 0,
    ClientVerify = 1,
    /// 클라이언트에서 서버로 보내는 로그인 요청 패킷
    LoginRequest = 2,
    /// 서버에서 클라이언트로 보내는 로그인 실패 응답 패킷
    LoginFailed = 3,
    /// 서버에서 클라이언트로 보내는 로그인 성공 응답 패킷
    LoginSuccess = 4,

    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    LobbyPull = 8,
    /// 클라이언트에서 서버로 보내는 데이터 응답 패킷
    LobbyPush = 9,
    /// 클라이언트에서 서버로 보내는 참여 가능 월드 식별자 목록 질의 패킷
    QueryAvailableWorlds = 10,
    /// 서버에서 클라이언트로 보내는 참여 가능 월드 식별자 목록 응답 패킷
    ResponseAvailableWorlds = 11,

    /// 클라이언트에서 서버로 보내는 커스텀 게임 참여 요청 패킷
    JoinRequest = 16,
    /// 서버에서 클라이언트로 보내는 커스텀 게임 참가 실패 응답 패킷
    JoinFailed = 17,

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

impl PacketType {
    /// 주어진 정수로 패킷 종류를 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(PacketType::Raw),
            1 => Some(PacketType::ClientVerify),
            2 => Some(PacketType::LoginRequest),
            3 => Some(PacketType::LoginFailed),
            4 => Some(PacketType::LoginSuccess),
            8 => Some(PacketType::LobbyPull),
            9 => Some(PacketType::LobbyPush),
            10 => Some(PacketType::QueryAvailableWorlds),
            11 => Some(PacketType::ResponseAvailableWorlds),
            16 => Some(PacketType::JoinRequest),
            17 => Some(PacketType::JoinFailed),
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
            _ => None,
        }
    }
}

impl BigEndian for PacketType {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
    }
}

impl TryFromBigEndian for PacketType {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[allow(unused_macros)]
macro_rules! test_packet_type {
    ($name: ident, $e: expr) => {
        #[test]
        fn $name() {
            let val = $e as u8;
            let packet_type = PacketType::new(val).unwrap();
            assert_eq!($e, packet_type);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    test_packet_type!(test_packet_type_raw, PacketType::Raw);

    test_packet_type!(test_packet_type_login_request, PacketType::LoginRequest);

    test_packet_type!(test_packet_type_login_failed, PacketType::LoginFailed);

    test_packet_type!(test_packet_type_login_success, PacketType::LoginSuccess);

    test_packet_type!(test_packet_type_lobby_pull, PacketType::LobbyPull);

    test_packet_type!(test_packet_type_lobby_push, PacketType::LobbyPush);

    test_packet_type!(
        test_packet_type_query_available_worlds,
        PacketType::QueryAvailableWorlds
    );

    test_packet_type!(
        test_packet_type_response_available_worlds,
        PacketType::ResponseAvailableWorlds
    );

    test_packet_type!(test_packet_type_join_request, PacketType::JoinRequest);

    test_packet_type!(test_packet_type_join_failed, PacketType::JoinFailed);
}
