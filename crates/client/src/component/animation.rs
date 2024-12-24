use ahash::HashMap;
use hecs::Entity;

/// ## Skinning Animation
/// 스키닝 애니메이션에 사용되는 스키닝 메쉬 엔터티와 최상위 뼈 노드 엔터티의 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinningAnimation {
    /// NOTE: `BoneCollection`의 `root`와 다름!
    pub root: Entity,
    pub meshes: HashMap<String, Entity>,
}

/// ## Bone Collection
/// 스키닝된 메쉬를 구성하는 뼈의 엔터티 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoneCollection {
    /// NOTE: 스키닝된 메쉬의 최상위 뼈를 나타냅니다.
    pub root: Entity,
    pub bones: Vec<Entity>,
}

/// ## Animation Timer
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AnimationTimer(pub f32);

impl AnimationTimer {
    /// 타이머를 초기화합니다.
    pub fn reset(&mut self) {
        self.0 = 0.0;
    }
}
