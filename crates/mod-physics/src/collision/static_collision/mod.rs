mod aabb_static_collision;
mod obb_static_collision;
mod capsule_static_collision;
mod orientedcapsule_static_collision;
mod sphere_static_collision;

use super::{CollisionDetails, ConvexHull};


/// 움직이지 않는 물체끼리의 충돌 검사
pub trait StaticCollision<T: ConvexHull> {
    fn check_static_collision(&self, other: &T) -> bool;
    fn check_static_collision_details(&self, other: &T) -> Option<CollisionDetails>;
}
