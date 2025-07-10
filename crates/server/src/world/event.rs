use std::sync::Arc;

use mod_network::components::{
    BulletKind, CharacterKind, GameTier, HeldInput, InputSnapshot, NetworkState, ProfileIcon,
    UserId, UserName,
};

use crate::session::Session;

/// 게임 월드 이벤트 목록입니다.
#[derive(Debug)]
pub enum GameWorldEvent {
    /// 게임 시스템 이벤트
    System {
        session: Arc<Session>,
        uid: UserId,
        event: GameWorldSystemEvent,
    },

    /// 커스텀 게임 대기실 이벤트
    RoomState {
        session: Arc<Session>,
        uid: UserId,
        event: GameWorldRoomStateEvent,
    },

    /// 캐릭터 편성 이벤트
    FormationState {
        session: Arc<Session>,
        uid: UserId,
        event: GameWorldFormationStateEvent,
    },

    /// 인게임 준비 이벤트
    InGameReadyState {
        session: Arc<Session>,
        uid: UserId,
        event: GameWorldInGameReadyStateEvent,
    },

    /// 인게임 이벤트
    InGameRunState(GameWorldInGameRunStateEvent),
}

/// 게임 월드의 시스템 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWorldSystemEvent {
    /// 플레이어가 게임 월드에 참여할 때 발생되는 이벤트입니다.
    PlayerJoin {
        name: UserName,
        tier: GameTier,
        profile_icon: ProfileIcon,
    },
    /// 플레이어가 게임 월드에서 떠날 때 발생되는 이벤트입니다.
    PlayerLeave,
    UpdatePing(NetworkState),
}

/// 커스텀 게임 대기실 상태의 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWorldRoomStateEvent {
    /// 플레이어가 커스텀 게임 대기실에서 준비를 요청할 때 발생되는 이벤트입니다.
    Ready,
    /// 플레이어가 커스텀 게임 대기실에서 팀을 변경할 때 발생되는 이벤트입니다.
    ChangeTeam(UserId),
    /// 방장이 캐릭터 중복 옵션을 변경할 때 발생되는 이벤트입니다.
    ChangeDuplicateOption,
    /// 방장이 팀 균형 옵션을 변경할 때 발생되는 이벤트입니다.
    ChangeUnbalanceOption,
    /// 방장이 커스텀 게임 대기실에서 플레이어를 차단할 때 발생되는 이벤트입니다.
    PlayerBan(UserId),
}

/// 캐릭터 편성 상태의 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWorldFormationStateEvent {
    /// 플레이어가 캐릭터 편성 장면에서 캐릭터를 선택할 때 발생되는 이벤트입니다.
    CharacterSelect(CharacterKind),
    /// 플레이어가 캐릭터 편성 장면에서 캐릭터 선택을 해제 할 때 발생되는 이벤트입니다.
    CharacterRelease,
}

/// 인게임 준비 상태의 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWorldInGameReadyStateEvent {
    /// 클라이언트가 게임 준비를 마쳤을 때 발생되는 이벤트입니다.
    ReadyToPlay,
}

#[derive(Debug, Clone)]
pub enum GameWorldInGameRunStateEvent {
    /// 플레이어 입력이 발생했을 때 발생되는 입력 이벤트 목록 이벤트입니다.
    InputSnapshot {
        /// 요청 세션
        session: Arc<Session>,
        /// 사용자 식별자
        uid: UserId,
        /// 클라이언트 게임 경과 시간입니다.
        client_play_elapsed_time_ms: u32,
        /// 입력 이벤트 목록
        snapshots: Vec<InputSnapshot>,
    },
}
