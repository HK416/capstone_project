mod convex_hull;
mod collider;
mod ray;

pub use convex_hull::ConvexHull;
pub use collider::Collider;
pub use ray::{Ray, RayIntersect, RayIntersectInfo};


pub struct CollisionDetails {
    pub normal: glam::Vec3A,
    pub penetration: f32,
    // pub contact_point: Vec<glam::Vec3A>,
}


impl RayIntersect for Collider {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo> {
        match self {
            Collider::Aabb(b) => ray.intersect(b),
            Collider::Obb(b) => ray.intersect(b),
            Collider::Capsule(c) => ray.intersect(c),
            Collider::Sphere(s) => ray.intersect(s),
        }
    }
}