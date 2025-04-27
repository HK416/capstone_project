//! 플레이어의 인게임 플레이 데이터와 관련된 코드를 관리합니다.
//!

use mod_network::components::{CharacterKind, Team, UserAccount};

/// 플레이어의 게임 플레이 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayData {
    /// 게임 연결 여부
    pub connected: bool,
    /// 로드 완료 여부
    pub loaded: bool,
    /// 사용자 계정 정보
    pub account: UserAccount,
    /// 캐릭터 종류
    pub character_kind: CharacterKind,
    /// 플레이어 팀
    pub team: Team,
    /// 플레이어 팀 내의 인덱스
    pub team_index: usize,

    /// 상대 팀을 처치한 횟수
    pub kill_count: u16,
    /// 상대 팀에게 처치당한 횟수
    pub dead_count: u16,

    /// 상대 팀에게 입힌 총 데미지 량
    pub damage_dealt: u32,
    /// 상대 팀에게 입은 총 데미지 량
    pub damage_taken: u32,
    /// 같은 팀을 회복 시킨 총 회복량
    pub healing_given: u32,
}
