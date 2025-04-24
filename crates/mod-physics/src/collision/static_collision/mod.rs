mod aabb_static_collision;
mod obb_static_collision;
mod capsule_static_collision;
mod orientedcapsule_static_collision;
mod sphere_static_collision;

use super::ConvexHull;


/// 움직이지 않는 물체끼리의 충돌 검사
pub trait StaticCollision<T: ConvexHull> {
    fn check_static_collision(&self, other: &T) -> bool;
    fn check_static_collision_details(&self, other: &T) -> Option<StaticCollisionDetails>;
}


#[derive(Debug)]
pub struct StaticCollisionDetails {
    pub normal: glam::Vec3A,
    pub penetration: f32,
    // pub contact_point: Vec<glam::Vec3A>,
}
