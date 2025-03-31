use std::sync::Arc;

use mod_app::asset::AssetManager;
use mod_network::components::{Float3, Float4, StageLayoutData};

use super::{AssetError, Root};

/// 에셋으로부터 스테이지를 로드합니다.
pub fn load_stage_from_asset(
    uri: &str,
    asset_manager: &AssetManager,
) -> Result<StageLayoutData, AssetError> {
    let cached_asset = asset_manager.get_or_init(&uri).map_err(|e| {
        log::error!("{} (PATH:{})", &e, &uri);
        AssetError::from(e)
    })?;
    let data: StageLayoutData = serde_json::from_slice(&cached_asset.as_bytes()).map_err(|e| {
        log::error!("{} (PATH:{})", &e, &uri);
        AssetError::from(e)
    })?;
    Ok(data)
}

#[derive(Debug)]
pub struct StageModel {
    pub is_terrain: bool,
    pub model_root: Arc<Root>,
    pub scale: Float3,
    pub rotation: Float4,
    pub translation: Float3,
}
