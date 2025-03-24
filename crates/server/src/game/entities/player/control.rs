use mod_network::components::{LatLon, ViewState, ViewStateTimer};

#[derive(Debug, Clone)]
pub struct ControlComponent {
    /// 입력 지속 시간 타이머입니다.
    /// 입력 지속 시간 타이머의 값이 `MAX_INPUT_DURATION`인 경우
    /// 플레이어 오브젝트는 최대 속력을 갖습니다.
    pub input_timer: f32,
    /// 플레이어 카메라의 움직임 상태입니다.
    pub view_state: ViewState,
    /// 플레이어 카메라의 움직임 상태 타이머입니다.
    pub view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터를 중심으로 회전한 각도입니다.
    pub view_rotation: LatLon,
}

impl Default for ControlComponent {
    fn default() -> Self {
        Self {
            input_timer: 0.0,
            view_state: ViewState::default(),
            view_state_timer: ViewStateTimer::default(),
            view_rotation: LatLon::default(),
        }
    }
}
