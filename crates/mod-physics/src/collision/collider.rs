use crate::{
    object3d::{
        BoundingBox, OrientedBoundingBox, 
        Capsule, OrientedCapsule,
        Sphere
    }, 
    collision::{
        StaticCollision, StaticCollisionDetails, 
        Ray, RayIntersect, RayIntersectInfo
    },
};
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Collider {
    Aabb(BoundingBox),
    Obb(OrientedBoundingBox),
    Capsule(Capsule),
    OrientedCapsule(OrientedCapsule),
    Sphere(Sphere),
}

impl Collider {
    pub fn check_collision(&self, other: &Self) -> bool {
        match other {
            Collider::Aabb(b) => self.check_aabb_collision(b),
            Collider::Obb(b) => self.check_obb_collision(b),
            Collider::Capsule(c) => self.check_capsule_collision(c),
            Collider::OrientedCapsule(c) => self.check_orientedcapsule_collision(c),
            Collider::Sphere(s) => self.check_sphere_collision(s),
        }
    }

    pub fn check_aabb_collision(&self, other: &BoundingBox) -> bool {
        match self {
            Collider::Aabb(b) => b.check_static_collision(other),
            Collider::Obb(b) => b.check_static_collision(other),
            Collider::Capsule(c) => c.check_static_collision(other),
            Collider::OrientedCapsule(c) => c.check_static_collision(other),
            Collider::Sphere(s) => s.check_static_collision(other),
        }
    }

    pub fn check_obb_collision(&self, other: &OrientedBoundingBox) -> bool {
        match self {
            Collider::Aabb(b) => b.check_static_collision(other),
            Collider::Obb(b) => b.check_static_collision(other),
            Collider::Capsule(c) => c.check_static_collision(other),
            Collider::OrientedCapsule(c) => c.check_static_collision(other),
            Collider::Sphere(s) => s.check_static_collision(other),
        }
    }
    
    pub fn check_capsule_collision(&self, other: &Capsule) -> bool {
        match self {
            Collider::Aabb(b) => b.check_static_collision(other),
            Collider::Obb(b) => b.check_static_collision(other),
            Collider::Capsule(c) => c.check_static_collision(other),
            Collider::OrientedCapsule(c) => c.check_static_collision(other),
            Collider::Sphere(s) => s.check_static_collision(other),
        }
    }

    pub fn check_orientedcapsule_collision(&self, other: &OrientedCapsule) -> bool {
        match self {
            Collider::Aabb(b) => b.check_static_collision(other),
            Collider::Obb(b) => b.check_static_collision(other),
            Collider::Capsule(c) => c.check_static_collision(other),
            Collider::OrientedCapsule(c) => c.check_static_collision(other),
            Collider::Sphere(s) => s.check_static_collision(other),
        }
    }

    pub fn check_sphere_collision(&self, other: &Sphere) -> bool {
        match self {
            Collider::Aabb(b) => b.check_static_collision(other),
            Collider::Obb(b) => b.check_static_collision(other),
            Collider::Capsule(c) => c.check_static_collision(other),
            Collider::OrientedCapsule(c) => c.check_static_collision(other),
            Collider::Sphere(s) => s.check_static_collision(other),
        }
    }
}


impl Collider {
    pub fn check_collision_details(&self, other: &Self) -> Option<StaticCollisionDetails> {
        match other {
            Collider::Aabb(b) => self.check_aabb_collision_details(b),
            Collider::Obb(b) => self.check_obb_collision_details(b),
            Collider::Capsule(c) => self.check_capsule_collision_details(c),
            Collider::OrientedCapsule(c) => self.check_orientedcapsule_collision_details(c),
            Collider::Sphere(s) => self.check_sphere_collision_details(s),
        }
    }

    pub fn check_aabb_collision_details(&self, other: &BoundingBox) -> Option<StaticCollisionDetails> {
        match self {
            Collider::Aabb(b) => b.check_static_collision_details(other),
            Collider::Obb(b) => b.check_static_collision_details(other),
            Collider::Capsule(c) => c.check_static_collision_details(other),
            Collider::OrientedCapsule(c) => c.check_static_collision_details(other),
            Collider::Sphere(s) => s.check_static_collision_details(other),
        }
    }

    pub fn check_obb_collision_details(&self, other: &OrientedBoundingBox) -> Option<StaticCollisionDetails> {
        match self {
            Collider::Aabb(b) => b.check_static_collision_details(other),
            Collider::Obb(b) => b.check_static_collision_details(other),
            Collider::Capsule(c) => c.check_static_collision_details(other),
            Collider::OrientedCapsule(c) => c.check_static_collision_details(other),
            Collider::Sphere(s) => s.check_static_collision_details(other),
        }
    }

    pub fn check_capsule_collision_details(&self, other: &Capsule) -> Option<StaticCollisionDetails> {
        match self {
            Collider::Aabb(b) => b.check_static_collision_details(other),
            Collider::Obb(b) => b.check_static_collision_details(other),
            Collider::Capsule(c) => c.check_static_collision_details(other),
            Collider::OrientedCapsule(c) => c.check_static_collision_details(other),
            Collider::Sphere(s) => s.check_static_collision_details(other),
        }
    }

    pub fn check_orientedcapsule_collision_details(&self, other: &OrientedCapsule) -> Option<StaticCollisionDetails> {
        match self {
            Collider::Aabb(b) => b.check_static_collision_details(other),
            Collider::Obb(b) => b.check_static_collision_details(other),
            Collider::Capsule(c) => c.check_static_collision_details(other),
            Collider::OrientedCapsule(c) => c.check_static_collision_details(other),
            Collider::Sphere(s) => s.check_static_collision_details(other),
        }
    }

    pub fn check_sphere_collision_details(&self, other: &Sphere) -> Option<StaticCollisionDetails> {
        match self {
            Collider::Aabb(b) => b.check_static_collision_details(other),
            Collider::Obb(b) => b.check_static_collision_details(other),
            Collider::Capsule(c) => c.check_static_collision_details(other),
            Collider::OrientedCapsule(c) => c.check_static_collision_details(other),
            Collider::Sphere(s) => s.check_static_collision_details(other),
        }
    }
}


impl RayIntersect for Collider {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo> {
        match self {
            Collider::Aabb(b) => ray.intersect(b),
            Collider::Obb(b) => ray.intersect(b),
            Collider::Capsule(c) => ray.intersect(c),
            Collider::OrientedCapsule(c) => ray.intersect(c),
            Collider::Sphere(s) => ray.intersect(s),
        }
    }
}