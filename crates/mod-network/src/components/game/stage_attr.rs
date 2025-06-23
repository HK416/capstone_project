//! 지형의 속성 데이터와 관련된 코드를 관리합니다.
//!

use serde::{Deserialize, Serialize};

use crate::components::{Float2, Float3, Float4, Float4x4};

/// 지형의 속성 데이터입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageLayoutAttributes {
    /// 게임 월드의 각 지역의 크기입니다.
    pub area_size: Float2,
    /// x축 방향의 지역의 수 입니다.
    pub num_area_width: u32,
    /// z축 방향의 지역의 수 입니다.
    pub num_area_depth: u32,
    /// 게임 월드 스테이지에서 사용되는 모델의 이름입니다.
    pub models: Vec<String>,
    /// 게임 월드 스테이지 지역 데이터입니다.
    pub area: Vec<StageLayoutAreaData>,
    /// 게임 월드 스테이지 소품 데이터입니다.
    pub root_prop: Option<Box<StageLayoutPropData>>,
    /// 게임 월드 스테이지 전역 조명 데이터입니다.
    pub global_light: Option<GlobalLightData>,
    /// 블루 팀 스폰 위치입니다.
    pub blue_spawn_pos: Vec<Float3>,
    /// 블루 팀 스폰 방향입니다.
    pub blue_spawn_dir: Float4,
    /// 블루팀 안전 영역의 시작점
    pub blue_safe_area_p0: Float2,
    /// 블루팀 안전 영역의 끝점
    pub blue_safe_area_p1: Float2,
    /// 레드 팀 스폰 위치입니다.
    pub red_spawn_pos: Vec<Float3>,
    /// 레드 팀 스폰 방향입니다.
    pub red_spawn_dir: Float4,
    /// 레드 팀 안전 영역의 시작점
    pub red_safe_area_p0: Float2,
    /// 레드 팀 안전 영역의 끝점
    pub red_safe_area_p1: Float2,
}

/// 게임 월드의 전역 조명 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalLightData {
    pub color: Float3,
    pub direction_w: Float3,
    pub shadow_map: String,
    pub light_proj_view: Float4x4,
}

/// 게임 월드 스테이지를 구성하는 지역 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageLayoutAreaData {
    pub model: String,
    pub height: Option<String>,
    pub translation: Float3,
    pub rotation: Float4,
}

/// 게임 월드 스테이지를 구성하는 소품 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageLayoutPropData {
    pub model: String,
    pub scale: Float3,
    pub rotation: Float4,
    pub translation: Float3,
    pub center: Float3,
    pub radius: f32,
    pub left: Option<Box<StageLayoutPropData>>,
    pub right: Option<Box<StageLayoutPropData>>,
}

/// 게임 월드 스테이지의 높이 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageLayoutAreaHeight {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>,
}
