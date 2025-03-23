mod enter;

use mod_network::components::{FormationPhasePlayer, LoginToken, UserId};

use crate::config::Locale;

pub use self::enter::*;

/// 인 게임 장면에 진입하기 전 캐릭터를 편성하는 장면입니다.  
pub struct CharacterFormationScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 현재 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 파란 팀에 속한 플레이어 집합
    blue_team_player: Vec<FormationPhasePlayer>,
    /// 빨간 팀에 속한 플레이어 집합
    red_team_player: Vec<FormationPhasePlayer>,
}
