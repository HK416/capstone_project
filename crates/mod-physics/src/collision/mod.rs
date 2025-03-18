mod convex_hull;
mod collider;
mod static_collision;
mod dynamic_collision;
mod ray_intersect;

pub use convex_hull::ConvexHull;
pub use collider::Collider;
pub use static_collision::StaticCollision;
pub use dynamic_collision::DynamicCollision;
pub use ray_intersect::{Ray, RayIntersect, RayIntersectInfo};


pub struct CollisionDetails {
    pub normal: glam::Vec3A,
    pub penetration: f32,
    // pub contact_point: Vec<glam::Vec3A>,
}
