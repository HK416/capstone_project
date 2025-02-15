use mod_app::asset::AssetManager;
use serde::{Deserialize, Serialize};

use super::{AssetError, Float3, Float4};

/// 게임 월드 스테이지 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameWorldMapData {
    pub plane: Vec<String>,
    pub area: Vec<GameWorldAreaData>,
}

/// 게임 월드 스테이지를 구성하는 지역 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameWorldAreaData {
    pub plane: String,
    pub height: String,
    pub translation: Float3,
    pub rotation: Float4,
}

/// 에셋으로부터 스테이지를 로드합니다.
pub fn load_stage_from_asset(
    name: &str,
    workspace: &str,
    asset_manager: &AssetManager,
) -> Result<GameWorldMapData, AssetError> {
    let path = format!("{}/{}", workspace, name);
    let cached_asset = asset_manager.get_or_init(&path).map_err(|e| {
        log::error!("{} (PATH:{})", &e, &path);
        AssetError::from(e)
    })?;
    let data: GameWorldMapData = serde_json::from_slice(&cached_asset.as_bytes()).map_err(|e| {
        log::error!("{} (PATH:{})", &e, &path);
        AssetError::from(e)
    })?;
    Ok(data)
}
