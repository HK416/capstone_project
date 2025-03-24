//! 게임에서 사용되는 플레이어 엔터티 관련 코드를 관리합니다.
//!

pub mod character;
pub mod control;
pub mod game;
pub mod transform;

use std::hash;

use mod_network::components::UserAccount;
use parking_lot::FairMutex;

pub use self::{character::*, control::*, game::*, transform::*};

/// 플레이어 속성을 저장합니다.
#[derive(Debug)]
pub struct Player {
    /// 플레이어 사용자 계정 데이터
    pub account: UserAccount,
    /// 게임 월드 속성
    pub transform: FairMutex<TransformComponent>,
    /// 캐릭터 속성
    pub character: FairMutex<CharacterComponent>,
    /// 게임 속성
    pub game_play: FairMutex<GameComponent>,
    /// 조작감 속성
    pub control: FairMutex<ControlComponent>,
}

impl Player {
    pub fn new(account: UserAccount) -> Self {
        Self {
            account,
            transform: FairMutex::new(TransformComponent::default()),
            character: FairMutex::new(CharacterComponent::default()),
            game_play: FairMutex::new(GameComponent::default()),
            control: FairMutex::new(ControlComponent::default()),
        }
    }
}

impl Eq for Player {}

impl PartialEq<Self> for Player {
    fn eq(&self, other: &Self) -> bool {
        self.account.uid.eq(&other.account.uid)
    }
}

impl hash::Hash for Player {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.account.uid.hash(state);
    }
}
