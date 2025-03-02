mod convex_hull;
pub use convex_hull::ConvexHull;

use crate::{
    bounds::BoundingBox,
    capsule::Capsule,
    sphere::Sphere,
};


#[derive(Debug, Clone)]
pub enum Collider {
    Box(BoundingBox),
    Capsule(Capsule),
    Sphere(Sphere),
}

impl Collider {
    pub fn check_collision(&self, other: &Self) -> bool {
        match other {
            Collider::Box(b) => self.is_collide(b),
            Collider::Capsule(c) => self.is_collide(c),
            Collider::Sphere(s) => self.is_collide(s),
        }
    }

    fn is_collide(&self, other: &impl ConvexHull) -> bool {
        let gjk_result = match self {
            Collider::Box(b) => b.gjk(other),
            Collider::Capsule(c) => c.gjk(other),
            Collider::Sphere(s) => s.gjk(other),
        };
        gjk_result.is_some()
    }

    pub fn check_collision_details(&self, other: &Self) -> Option<CollisionDetails> {
        match other {
            Collider::Box(b) => self.get_collision_details(b),
            Collider::Capsule(c) => self.get_collision_details(c),
            Collider::Sphere(s) => self.get_collision_details(s),
        }
    }

    fn get_collision_details(&self, other: &impl ConvexHull) -> Option<CollisionDetails> {
        match self {
            Collider::Box(b) => b.gjk_epa(other),
            Collider::Capsule(c) => c.gjk_epa(other),
            Collider::Sphere(s) => s.gjk_epa(other),
        }
    }
}


pub struct CollisionDetails {
    pub normal: glam::Vec3A,
    pub penetration: f32,
    // pub contact_point: Vec<glam::Vec3A>,
}