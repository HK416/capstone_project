use crate::object3d::{
    BoundingBox, OrientedBoundingBox, 
    Capsule, OrientedCapsule, 
    Sphere
};
use super::{StaticCollision, StaticCollisionDetails};


impl StaticCollision<BoundingBox> for OrientedCapsule {
    fn check_static_collision(&self, other: &BoundingBox) -> bool {
        other.check_static_collision(self)
    }

    fn check_static_collision_details(&self, other: &BoundingBox) -> Option<StaticCollisionDetails> {
        let mut details = other.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<OrientedBoundingBox> for OrientedCapsule {
    fn check_static_collision(&self, other: &OrientedBoundingBox) -> bool {
        other.check_static_collision(self)
    }

    fn check_static_collision_details(&self, other: &OrientedBoundingBox) -> Option<StaticCollisionDetails> {
        let mut details = other.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<Capsule> for OrientedCapsule {
    fn check_static_collision(&self, other: &Capsule) -> bool {
        let other = OrientedCapsule::new(
            other.center.into(),
            glam::Vec3::Y,
            other.height,
            other.radius,
        ).unwrap();
        self.check_static_collision(&other)
    }

    fn check_static_collision_details(&self, other: &Capsule) -> Option<StaticCollisionDetails> {
        let other = OrientedCapsule::new(
            other.center.into(),
            glam::Vec3::Y,
            other.height,
            other.radius,
        ).unwrap();
        self.check_static_collision_details(&other)
    }
}

impl StaticCollision<OrientedCapsule> for OrientedCapsule {
    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
    fn check_static_collision(&self, other: &OrientedCapsule) -> bool {
        use mod_math::Segment;
        
        // 두 캡슐위의 점 사이 거리가 두 캡슐의 높이 합보다 크면 충돌하지 않음
        let c_to_c = glam::Vec3A::from(other.center - self.center);
        if c_to_c.length_squared() > (self.height + other.height).powi(2) {
            return false;
        }

        // 테스트 필요
        let distance = Segment::distance_between_segments(&self.get_seg(), &other.get_seg());

        distance <= self.radius + other.radius
    }

    fn check_static_collision_details(&self, other: &OrientedCapsule) -> Option<StaticCollisionDetails> {
        use mod_math::Segment;
        
        let (nearest1, nearest2) = Segment::each_nearest(&self.get_seg(), &other.get_seg());
        let normal = nearest1 - nearest2;
        let distance = normal.length();
        let penetration = self.radius + other.radius - distance;
        if penetration < 0.0 {
            return None;
        }

        Some(StaticCollisionDetails {
            normal: normal.normalize_or_zero(),
            penetration,
        })
    }
}

impl StaticCollision<Sphere> for OrientedCapsule {
    fn check_static_collision(&self, sphere: &Sphere) -> bool {
        let segment = self.get_seg();
        let center = glam::Vec3A::from(sphere.center);

        segment.distance_to_point(&center) <= sphere.radius + self.radius
    }

    fn check_static_collision_details(&self, sphere: &Sphere) -> Option<StaticCollisionDetails> {
        let center = sphere.center - self.center;
        let sphere = Sphere {
            center,
            radius: sphere.radius,
        };
        let capsule = Capsule {
            center: glam::Vec3::ZERO,
            height: self.height,
            radius: self.radius,
        };

        capsule.check_static_collision_details(&sphere)
    }
}
