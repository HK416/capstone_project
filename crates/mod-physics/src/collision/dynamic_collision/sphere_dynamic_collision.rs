use crate::{
    object3d::{Capsule, Sphere},
    collision::Ray,
};
use super::{DynamicCollision, DynamicCollisionDetails};


impl DynamicCollision<Capsule> for Sphere {
    fn check_dynamic_collision(&self, velocity: &glam::Vec3A, capsule: &Capsule) -> bool {
        self.check_dynamic_collision_details(velocity, capsule).is_some()
    }

    fn check_dynamic_collision_details(&self, velocity: &glam::Vec3A, capsule: &Capsule) -> Option<DynamicCollisionDetails> {
        let target = capsule.inflated(capsule.radius);
        let ray = Ray::build(self.center.into(), *velocity).unwrap();
        let info = ray.intersect(&target)?;

        Some(DynamicCollisionDetails {
            normal: info.normal,
            distance: info.distance,
        })
    }
}