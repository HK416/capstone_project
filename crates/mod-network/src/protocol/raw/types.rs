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
    QueryWorldLists = 0x02,
    /// 서버에서 클라이언트로 보내는 참여 가능 월드 식별자 목록 응답 패킷
    ResponseWorldList = 0x03,

    /// 클라이언트 유효성 검증을 위해 미리 예약된 패킷 유형
    ClientVerifyType = 0x10,

    /// 로그인에 사용되는 패킷 유형
    LoginType = 0x20,
    /// 클라이언트에서 서버로 보내는 로그인 요청 패킷
    RequestLogin = 0x21,
    /// 서버에서 클라이언트로 보내는 로그인 실패 응답 패킷
    ResponseLoginFailed = 0x22,
    /// 서버에서 클라이언트로 보내는 로그인 성공 응답 패킷
    ResponseLoginSuccess = 0x23,

    /// 게임 로비에서 사용되는 패킷 유형
    LobbyType = 0x30,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    PullLobbyData = 0x31,
    /// 클라이언트에서 서버로 보내는 커스텀 게임 참여 요청 패킷
    RequestJoinRoom = 0x32,
    /// 서버에서 클라이언트로 보내는 커스텀 게임 참가 실패 응답 패킷
    ResponseJoinFailed = 0x33,

    /// 커스텀 게임에서 사용되는 패킷 유형
    RoomType = 0x40,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    PullRoomData = 0x41,
    /// 클라이언트에서 서버로 보내는 커스텀 게임 떠남 알림 패킷
    NotifyRoomLeave = 0x42,
    /// 클라이언트에서 서버로 보내는 커스텀 게임 준비 요청 패킷
    RequestRoomReady = 0x43,
    /// 클라이언트에서 서버로 보내는 팀 변경 요청 패킷
    RequestChangeTeam = 0x44,
    /// 서버에서 클라이언트로 보내는 게임 시작 실패 응답 패킷
    ResponseStartFailed = 0x45,

    /// 캐릭터 편성에서 사용되는 패킷 유형
    FomationType = 0x50,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    PullFormationData = 0x51,
    /// 클라이언트에서 서버로 보내는 캐릭터 선택 요청 패킷
    RequestCharacterSelect = 0x52,
    /// 서버에서 클라이언트로 보내는 캐릭터 선택 응답 패킷
    ResponseCharacterSelect = 0x53,
    /// 서버에서 클라이언트로 보내는 인게임 진입 실패 알림 패킷
    NotifyEnterFailed = 0x54,

    /// 인게임 준비에서 사용되는 패킷 유형
    PrepareType = 0x60,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    PullPrepareData = 0x61,
    /// 클라이언트에서 서버로 보내는 데이터 갱신 패킷
    PushPrepareData = 0x62,

    /// 인게임에서 사용되는 패킷 유형
    InGameType = 0x70,
    /// 서버에서 클라이언트로 보내는 데이터 갱신 패킷
    PullInGameData = 0x71,
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
            0x02 => Some(PacketType::QueryWorldLists),
            0x03 => Some(PacketType::ResponseWorldList),
            0x21 => Some(PacketType::RequestLogin),
            0x22 => Some(PacketType::ResponseLoginFailed),
            0x23 => Some(PacketType::ResponseLoginSuccess),
            0x31 => Some(PacketType::PullLobbyData),
            0x32 => Some(PacketType::RequestJoinRoom),
            0x33 => Some(PacketType::ResponseJoinFailed),
            0x41 => Some(PacketType::PullRoomData),
            0x42 => Some(PacketType::NotifyRoomLeave),
            0x43 => Some(PacketType::RequestRoomReady),
            0x44 => Some(PacketType::RequestChangeTeam),
            0x45 => Some(PacketType::ResponseStartFailed),
            0x51 => Some(PacketType::PullFormationData),
            0x52 => Some(PacketType::RequestCharacterSelect),
            0x53 => Some(PacketType::ResponseCharacterSelect),
            0x54 => Some(PacketType::NotifyEnterFailed),
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

    test_packet_type!(test_packet_type_query_world_list, PacketType::QueryWorldLists);

    test_packet_type!(test_packet_type_response_world_list, PacketType::ResponseWorldList);

    test_packet_type!(test_packet_type_request_login, PacketType::RequestLogin);

    test_packet_type!(test_packet_type_response_login_failed, PacketType::ResponseLoginFailed);

    test_packet_type!(test_packet_type_response_login_success, PacketType::ResponseLoginSuccess);
    
    test_packet_type!(test_packet_type_pull_lobby_data, PacketType::PullLobbyData);

    test_packet_type!(test_packet_type_request_join_room, PacketType::RequestJoinRoom);

    test_packet_type!(test_packet_type_response_join_failed, PacketType::ResponseJoinFailed);

    test_packet_type!(test_packet_type_pull_room_data, PacketType::PullRoomData);

    test_packet_type!(test_packet_type_notify_room_leave, PacketType::NotifyRoomLeave);

    test_packet_type!(test_packet_type_request_room_ready, PacketType::RequestRoomReady);

    test_packet_type!(test_packet_type_request_change_team, PacketType::RequestChangeTeam);

    test_packet_type!(test_packet_type_response_start_failed, PacketType::ResponseStartFailed);

    test_packet_type!(test_packet_type_pull_formation_data, PacketType::PullFormationData);

    test_packet_type!(test_packet_type_request_character_select, PacketType::RequestCharacterSelect);

    test_packet_type!(test_packet_type_response_character_select, PacketType::ResponseCharacterSelect);

    test_packet_type!(test_packet_type_notify_enter_failed, PacketType::NotifyEnterFailed);
}
