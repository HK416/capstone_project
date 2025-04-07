use crate::{
    object3d::{BoundingBox, Capsule, Sphere, OrientedBoundingBox, OrientedCapsule},
    collision::Ray,
};
use super::{DynamicCollision, DynamicCollisionDetails};


impl DynamicCollision<Sphere> for Sphere {
    fn check_dynamic_collision(&self, velocity: &glam::Vec3A, sphere: &Sphere) -> bool {
        self.check_dynamic_collision_details(velocity, sphere).is_some()
    }

    fn check_dynamic_collision_details(&self, velocity: &glam::Vec3A, sphere: &Sphere) -> Option<DynamicCollisionDetails> {
        let target = sphere.inflated(sphere.radius);
        let ray = Ray::build(self.center.into(), *velocity).unwrap();
        let info = ray.intersect(&target)?;

        Some(DynamicCollisionDetails {
            normal: info.normal,
            distance: info.distance,
        })
    }
}


impl DynamicCollision<BoundingBox> for Sphere {
    /// 구의 반지름만큼 aabb의 x, y, z를 확장해서 ray-cast
    /// 
    /// 모서리부분에서는 정확하지 않습니다.
    fn check_dynamic_collision(&self, velocity: &glam::Vec3A, aabb: &BoundingBox) -> bool {
        self.check_dynamic_collision_details(velocity, aabb).is_some()
    }

    /// 구의 반지름만큼 aabb의 x, y, z를 확장해서 ray-cast  
    /// 
    /// 모서리부분에서는 정확하지 않습니다.  
    fn check_dynamic_collision_details(&self, velocity: &glam::Vec3A, aabb: &BoundingBox) -> Option<DynamicCollisionDetails> {
        let target = aabb.expanded(self.radius);
        let ray = Ray::build(self.center.into(), *velocity).unwrap();
        let info = ray.intersect(&target)?;

        Some(DynamicCollisionDetails {
            normal: info.normal,
            distance: info.distance,
        })
    }
}


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


impl DynamicCollision<OrientedBoundingBox> for Sphere {
    fn check_dynamic_collision(&self, velocity: &glam::Vec3A, obb: &OrientedBoundingBox) -> bool {
        self.check_dynamic_collision_details(velocity, obb).is_some()
    }

    fn check_dynamic_collision_details(&self, velocity: &glam::Vec3A, obb: &OrientedBoundingBox) -> Option<DynamicCollisionDetails> {
        let target = obb.expanded(self.radius);
        let ray = Ray::build(self.center.into(), *velocity).unwrap();
        let info = ray.intersect(&target)?;

        Some(DynamicCollisionDetails {
            normal: info.normal,
            distance: info.distance,
        })
    }
}


impl DynamicCollision<OrientedCapsule> for Sphere {
    fn check_dynamic_collision(&self, velocity: &glam::Vec3A, ocapsule: &OrientedCapsule) -> bool {
        self.check_dynamic_collision_details(velocity, ocapsule).is_some()
    }

    fn check_dynamic_collision_details(&self, velocity: &glam::Vec3A, ocapsule: &OrientedCapsule) -> Option<DynamicCollisionDetails> {
        let target = ocapsule.inflated(self.radius);
        let ray = Ray::build(self.center.into(), *velocity).unwrap();
        let info = ray.intersect(&target)?;

        Some(DynamicCollisionDetails {
            normal: info.normal,
            distance: info.distance,
        })
    }
}