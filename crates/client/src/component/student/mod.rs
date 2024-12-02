use hecs::World;

/// ## Student Tag
/// `Entity`가 학생임을 식별하는 태그입니다.
pub struct StudentTag;

/// ## Student Kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StudentKind {
    ArisOriginal,
}

impl ToString for StudentKind {
    fn to_string(&self) -> String {
        match self {
            StudentKind::ArisOriginal => "Aris Original".to_string(),
        }
    }
}

/// ## Player Behavior States
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BehaviorState {
    Idle,
    Moving,
    MoveToEnd,
}

impl Into<usize> for BehaviorState {
    fn into(self) -> usize {
        match self {
            BehaviorState::Idle => 0,
            BehaviorState::Moving => 1,
            BehaviorState::MoveToEnd => 2,
        }
    }
}
