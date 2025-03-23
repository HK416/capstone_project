pub mod formation;
pub mod play;
pub mod recruit;

use std::fmt;

pub use self::{formation::*, play::*, recruit::*};

impl fmt::Debug for PlayerRecruitPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(PlayerRecruitPhase))
    }
}

impl fmt::Debug for CharacterFormationPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(CharacterFormationPhase))
    }
}

impl fmt::Debug for GamePlayPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GamePlayPhase))
    }
}
