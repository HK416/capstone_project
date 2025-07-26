use mod_physics::collision::Collider;
use serde::{Deserialize, Serialize};

use crate::components::{Float3, Float4, Float4x4};

/// 지형의 속성 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageAttributesData {
    /// 지형의 이름
    pub name: String,

    /// 지역의 x축 방향 개수입니다.
    pub num_area_width: u32,
    /// 지역의 z축 방향 개수입니다.
    pub num_area_depth: u32,

    /// 지역의 x축 방향 길이입니다.
    pub area_width: f32,
    /// 지역의 z축 방향 길이입니다.
    pub area_depth: f32,

    /// 게임 월드 스테이지에서 사용되는 모델의 목록
    pub model_list: Vec<String>,
    /// 게임 월드 스테이지에서 사용되는 배경음 목록
    pub sound_list: Vec<String>,

    /// 전역 조명 데이터입니다.
    pub global_light: Option<GlobalLightData>,
    /// 지역 데이터
    pub area: Vec<AreaAttributeData>,
    /// 소품 데이터입니다.
    pub prop: Option<Box<PropAttributeData>>,

    /// 충돌체 데이터 파일 Uri입니다.
    pub collider: String,
    /// 점령 지역의 충돌체입니다.
    pub capture_zone: Collider,

    /// 블루 팀 스폰 방향입니다.
    pub blue_team_rotation: Float4,
    /// 블루 팀 스폰 위치입니다.
    pub blue_team_positions: Vec<Float3>,
    /// 블루 팀 안전 지역 충돌체 데이터 파일 Uri입니다.
    pub blue_team_collider: String,

    /// 레드 팀 스폰 방향입니다.
    pub red_team_rotation: Float4,
    /// 레드 팀 스폰 위치입니다.
    pub red_team_positions: Vec<Float3>,
    /// 레드 팀 안전 지역 충돌체 데이터 파일 Uri입니다.
    pub red_team_collider: String,

    /// 승리 팀 방향입니다.
    pub winner_rotation: Float4,
    /// 승리 팀 위치입니다.
    pub winner_positions: Vec<Float3>,
    /// 카메라 위치
    pub camera_position: Float3,
    /// 카메라 방향
    pub camera_rotation: Float4,
    /// 카메라 Fov-y
    pub camera_fov_y: f32,
}

/// 게임 월드 전역 조명 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalLightData {
    /// 조명의 색상입니다.
    pub color: Float3,
    /// 조명의 방향입니다.
    pub direction_w: Float3,
    /// 정적 그림자 맵 텍스처입니다.
    pub static_shadow_map: String,
    /// 정적 그림자 맵의 조명 변환 행렬입니다.
    pub static_light_proj_view: Float4x4,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum RotationY {
    Rotate0,
    Rotate90,
    Rotate180,
    Rotate270,
}

impl RotationY {
    pub const fn to_quat(self) -> glam::Quat {
        match self {
            RotationY::Rotate0 => glam::quat(0.0, 0.0, 0.0, 1.0),
            RotationY::Rotate90 => glam::quat(0.0, 0.7071068, 0.0, 0.7071068),
            RotationY::Rotate180 => glam::quat(0.0, 1.0, 0.0, 0.0),
            RotationY::Rotate270 => glam::quat(0.0, 0.7071068, 0.0, -0.7071068),
        }
    }
}

/// 지역의 속성 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AreaAttributeData {
    /// 지역 모델의 Uri입니다.
    pub model: String,
    /// 지역의 높이 텍스처입니다.
    pub height_map: Option<String>,
    /// 월드 공간 위치
    pub translation: Float3,
    /// 월드 공간 방향
    pub rotation: RotationY,
}

/// 장식물의 속성 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PropAttributeData {
    /// 지역 모델의 Uri입니다.
    pub model: String,
    /// 월드 공간 스케일
    pub scale: Float3,
    /// 월드 공간 방향
    pub rotation: Float4,
    /// 월드 공간 위치
    pub translation: Float3,
    /// 충돌 구체의 중심 점
    pub center: Float3,
    /// 충돌 구체의 반경
    pub radius: f32,
    /// 자식 노드
    pub left: Option<Box<PropAttributeData>>,
    /// 자식 노드
    pub right: Option<Box<PropAttributeData>>,
}

/// 높이 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeightData {
    /// 텍스처의 가로 길이
    pub width: u32,
    /// 텍스처의 세로 길이
    pub height: u32,
    /// 높이 데이터
    pub data: Vec<f32>,
}
