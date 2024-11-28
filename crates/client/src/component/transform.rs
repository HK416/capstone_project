/// ## To Parent Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToParentTrans(pub glam::Mat4);

impl Default for ToParentTrans {
    fn default() -> Self {
        Self(glam::Mat4::IDENTITY)
    }
}

/// ## World Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldTransform(pub glam::Mat4);

impl Default for WorldTransform {
    fn default() -> Self {
        Self(glam::Mat4::IDENTITY)
    }
}
