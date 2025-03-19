pub mod rigid_body;
pub mod object3d;
pub mod collision;


#[derive(Debug, Clone, serde::Deserialize)]
pub struct ColliderTree {
    pub collider: collision::Collider,
    pub left: Option<Box<ColliderTree>>,
    pub right: Option<Box<ColliderTree>>,
}