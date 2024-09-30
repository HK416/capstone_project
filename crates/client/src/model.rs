use rust_embed::Embed;

/// `Aris_Original` 모델의 경로입니다.
const PATH: &'static str = "characters/aris_original/Aris_Original_Mesh.ron";

/// 임베딩된 에셋 파일 관리자입니다.
#[derive(Embed)]
#[folder = "assets/"]
struct EmbededAssets;
