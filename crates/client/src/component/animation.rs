use ahash::HashMap;
use hecs::Entity;

/// ## Motion Collection
/// 스키닝 애니메이션에 사용되는 스키닝된 메쉬 `Entity`와 최상위 뼈 노드 `Entity`의 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionCollection {
    /// NOTE: `BoneCollection`의 `root`와 다름!
    pub root: Entity,
    pub meshes: HashMap<String, Entity>,
}

/// ## Bone Collection
/// 스키닝된 메쉬를 구성하는 뼈의 `Entity` 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoneCollection {
    /// NOTE: 스키닝된 메쉬의 최상위 뼈를 나타냅니다.
    pub root: Entity,
    pub bones: Vec<Entity>,
}

/// ## Animation Timer
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AnimationTimer(pub f32);
