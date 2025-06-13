//! 패킷의 종류와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 패킷의 종류를 나타냅니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PacketType {
    /// 시스템 패킷 타입
    #[default]
    Raw = 0x00,
    /// 핑을 측정하기 위해 받은 패킷을 다시 전송하는 패킷
    Ping = 0x01,
    /// 클라이언트에서 서버로 보내는 참여 가능 월드 식별자 목록 질의 패킷
    WorldListQuery = 0x02,
    /// 서버에서 클라이언트로 보내는 참여 가능 월드 식별자 목록 응답 패킷
    WorldListResponse = 0x03,

    /// 클라이언트 유효성 검증을 위해 미리 예약된 패킷 유형
    ClientVerifyType = 0x10,

    /// 로그인에 사용되는 패킷 유형
    LoginGroup = 0x20,
    /// 클라이언트에서 서버로 보내는 로그인 요청 패킷
    LoginRequest = 0x21,
    /// 서버에서 클라이언트로 보내는 로그인 실패 응답 패킷
    LoginFailed = 0x22,
    /// 서버에서 클라이언트로 보내는 로그인 성공 응답 패킷
    LoginSuccess = 0x23,

    /// 게임 로비에서 사용되는 패킷 유형
    LobbyGroup = 0x30,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    LobbyDataUpdate = 0x31,
    /// 클라이언트에서 서버로 보내는 커스텀 게임 참여 요청 패킷
    JoinRoomRequest = 0x32,
    /// 서버에서 클라이언트로 보내는 커스텀 게임 참가 실패 응답 패킷
    JoinRoomFailed = 0x33,

    /// 커스텀 게임에서 사용되는 패킷 유형
    RoomGroup = 0x40,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    RoomDataUpdate = 0x41,
    /// 클라이언트에서 서버로 보내는 커스텀 게임 떠남 알림 패킷
    RoomLeaveNotify = 0x42,
    /// 클라이언트에서 서버로 보내는 커스텀 게임 준비 요청 패킷
    RoomReadyRequest = 0x43,
    /// 클라이언트에서 서버로 보내는 팀 변경 요청 패킷
    ChangeTeamRequest = 0x44,
    /// 서버에서 클라이언트로 보내는 게임 시작 실패 응답 패킷
    StartGameFailed = 0x45,

    /// 캐릭터 편성에서 사용되는 패킷 유형
    FormationGroup = 0x50,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    FormationDataUpdate = 0x51,
    /// 클라이언트에서 서버로 보내는 캐릭터 선택 요청 패킷
    CharacterSelectRequest = 0x52,
    /// 서버에서 클라이언트로 보내는 캐릭터 선택 응답 패킷
    CharacterSelectResponse = 0x53,
    /// 서버에서 클라이언트로 보내는 인게임 진입 실패 알림 패킷
    EnterGameFailed = 0x54,

    /// 인게임 준비에서 사용되는 패킷 유형
    PrepareGroup = 0x60,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    PrepareDataPull = 0x61,
    /// 클라이언트에서 서버로 보내는 데이터 갱신 패킷
    PrepareDataPush = 0x62,

    /// 인게임에서 사용되는 패킷 유형
    InGameGroup = 0x70,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    InGameDataUpdate = 0x71,
}

impl PacketType {
    /// 주어진 정수로 패킷 종류를 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0x00 => Some(PacketType::Raw),
            0x01 => Some(PacketType::Ping),
            0x02 => Some(PacketType::WorldListQuery),
            0x03 => Some(PacketType::WorldListResponse),
            0x21 => Some(PacketType::LoginRequest),
            0x22 => Some(PacketType::LoginFailed),
            0x23 => Some(PacketType::LoginSuccess),
            0x31 => Some(PacketType::LobbyDataUpdate),
            0x32 => Some(PacketType::JoinRoomRequest),
            0x33 => Some(PacketType::JoinRoomFailed),
            0x41 => Some(PacketType::RoomDataUpdate),
            0x42 => Some(PacketType::RoomLeaveNotify),
            0x43 => Some(PacketType::RoomReadyRequest),
            0x44 => Some(PacketType::ChangeTeamRequest),
            0x45 => Some(PacketType::StartGameFailed),
            0x51 => Some(PacketType::FormationDataUpdate),
            0x52 => Some(PacketType::CharacterSelectRequest),
            0x53 => Some(PacketType::CharacterSelectResponse),
            0x54 => Some(PacketType::EnterGameFailed),
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
#[rustfmt::skip]
mod tests {
    use super::*;

    test_packet_type!(test_packet_type_raw, PacketType::Raw);

    test_packet_type!(test_packet_type_ping, PacketType::Ping);

    test_packet_type!(test_packet_type_world_list_query, PacketType::WorldListQuery);

    test_packet_type!(test_packet_type_world_list_response, PacketType::WorldListResponse);

    test_packet_type!(test_packet_type_login_request, PacketType::LoginRequest);

    test_packet_type!(test_packet_type_login_failed, PacketType::LoginFailed);

    test_packet_type!(test_packet_type_login_success, PacketType::LoginSuccess);
    
    test_packet_type!(test_packet_type_lobby_data_update, PacketType::LobbyDataUpdate);

    test_packet_type!(test_packet_type_join_room_request, PacketType::JoinRoomRequest);

    test_packet_type!(test_packet_type_join_room_failed, PacketType::JoinRoomFailed);

    test_packet_type!(test_packet_type_room_data_update, PacketType::RoomDataUpdate);

    test_packet_type!(test_packet_type_room_leave_notify, PacketType::RoomLeaveNotify);

    test_packet_type!(test_packet_type_room_ready_request, PacketType::RoomReadyRequest);

    test_packet_type!(test_packet_type_change_team_request, PacketType::ChangeTeamRequest);

    test_packet_type!(test_packet_type_start_game_failed, PacketType::StartGameFailed);

    test_packet_type!(test_packet_type_formation_data_update, PacketType::FormationDataUpdate);

    test_packet_type!(test_packet_type_character_select_request, PacketType::CharacterSelectRequest);

    test_packet_type!(test_packet_type_character_select_response, PacketType::CharacterSelectResponse);

    test_packet_type!(test_packet_type_enter_game_failed, PacketType::EnterGameFailed);
}
