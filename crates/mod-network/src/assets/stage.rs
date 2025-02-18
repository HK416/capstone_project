use serde::{Deserialize, Serialize};

use super::{Float2, Float3, Float4};

/// 게임 월드 스테이지 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageLayoutData {
    /// 게임 월드의 각 지역의 크기입니다.
    pub area_size: Float2,
    /// x축 방향의 지역의 수 입니다.
    pub num_area_width: u32,
    /// z축 방향의 지역의 수 입니다.
    pub num_area_depth: u32,
    /// 게임 월드 스테이지에서 사용되는 모델의 이름입니다.
    pub models: Vec<String>,
    /// 게임 월드 스테이지 지역 데이터입니다.
    pub area: Vec<StageAreaData>,
    /// 게임 월드 스테이지 소품 데이터입니다.
    pub props: Vec<StagePropData>,
}

/// 게임 월드 스테이지를 구성하는 지역 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageAreaData {
    pub model: String,
    pub height: String,
    pub translation: Float3,
    pub rotation: Float4,
}

/// 게임 월드 스테이지를 구성하는 소품 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StagePropData {
    pub model: String,
    pub scale: Float3,
    pub translation: Float3,
    pub rotation: Float4,
}

/// 게임 월드 스테이지의 높이 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageHeight {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>,
}
