/// 버튼의 상태 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ButtonState {
    Idle,
    Hovered,
    Pressed,
    Clicked,
}
