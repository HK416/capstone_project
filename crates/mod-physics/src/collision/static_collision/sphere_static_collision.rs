use crate::object3d::{
    BoundingBox, OrientedBoundingBox,
    Capsule, OrientedCapsule,
    Sphere
};
use super::{CollisionDetails, StaticCollision};


impl StaticCollision<BoundingBox> for Sphere {
    fn check_static_collision(&self, other: &BoundingBox) -> bool {
        other.check_static_collision(self)
    }

    fn check_static_collision_details(&self, other: &BoundingBox) -> Option<CollisionDetails> {
        let mut details = other.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<OrientedBoundingBox> for Sphere {
    fn check_static_collision(&self, other: &OrientedBoundingBox) -> bool {
        other.check_static_collision(self)
    }

    fn check_static_collision_details(&self, other: &OrientedBoundingBox) -> Option<CollisionDetails> {
        let mut details = other.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<Capsule> for Sphere {
    fn check_static_collision(&self, capsule: &Capsule) -> bool {
        capsule.check_static_collision(self)
    }

    fn check_static_collision_details(&self, capsule: &Capsule) -> Option<CollisionDetails> {
        let mut details = capsule.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<OrientedCapsule> for Sphere {
    fn check_static_collision(&self, capsule: &OrientedCapsule) -> bool {
        capsule.check_static_collision(self)
    }

    fn check_static_collision_details(&self, capsule: &OrientedCapsule) -> Option<CollisionDetails> {
        let mut details = capsule.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<Sphere> for Sphere {
    fn check_static_collision(&self, other: &Sphere) -> bool {
        let center1 = glam::Vec3A::from(self.center);
        let center2 = glam::Vec3A::from(other.center);
        (center1 - center2).length_squared() <= (self.radius + other.radius).powi(2)
    }

    fn check_static_collision_details(&self, other: &Sphere) -> Option<CollisionDetails> {
        let center1 = glam::Vec3A::from(self.center);
        let center2 = glam::Vec3A::from(other.center);
        let normal = center1 - center2;
        let distance = normal.length();
        let penetration = self.radius + other.radius - distance;
        
        if penetration < 0.0 {
            return None;
        }

        Some(CollisionDetails {
            normal: normal.normalize_or_zero(),
            penetration,
        })
    }
}
