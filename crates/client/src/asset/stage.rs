use mod_app::asset::AssetManager;
use mod_network::components::StageLayoutData;

use super::AssetError;

/// 에셋으로부터 스테이지를 로드합니다.
pub fn load_stage_from_asset(
    name: &str,
    workspace: &str,
    asset_manager: &AssetManager,
) -> Result<StageLayoutData, AssetError> {
    let path = format!("{}/{}", workspace, name);
    let cached_asset = asset_manager.get_or_init(&path).map_err(|e| {
        log::error!("{} (PATH:{})", &e, &path);
        AssetError::from(e)
    })?;
    let data: StageLayoutData = serde_json::from_slice(&cached_asset.as_bytes()).map_err(|e| {
        log::error!("{} (PATH:{})", &e, &path);
        AssetError::from(e)
    })?;
    Ok(data)
}
