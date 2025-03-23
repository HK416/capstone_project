mod sphere_dynamic_collision;

use super::ConvexHull;


/// 움직이는 물체(self)와 움직이지 않는 물체(other)의 충돌 검사
pub trait DynamicCollision<T: ConvexHull> {
    fn check_dynamic_collision(&self, velocity: &glam::Vec3A, other: &T) -> bool;
    fn check_dynamic_collision_details(&self, velocity: &glam::Vec3A, other: &T) -> Option<DynamicCollisionDetails>;
}


pub struct DynamicCollisionDetails {
    pub normal: glam::Vec3A,
    pub distance: f32,
}
