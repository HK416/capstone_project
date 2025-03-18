use crate::object3d::{
    BoundingBox, OrientedBoundingBox, 
    Capsule, OrientedCapsule, 
    Sphere
};
use super::{CollisionDetails, StaticCollision};
use mod_math::Segment;


impl StaticCollision<BoundingBox> for Capsule {
    fn check_static_collision(&self, other: &BoundingBox) -> bool {
        other.check_static_collision(self)
    }

    fn check_static_collision_details(&self, other: &BoundingBox) -> Option<CollisionDetails> {
        let mut details = other.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<OrientedBoundingBox> for Capsule {
    fn check_static_collision(&self, other: &OrientedBoundingBox) -> bool {
        other.check_static_collision(self)
    }

    fn check_static_collision_details(&self, other: &OrientedBoundingBox) -> Option<CollisionDetails> {
        let mut details = other.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<Capsule> for Capsule {
    fn check_static_collision(&self, other: &Capsule) -> bool {
        let a = self.inflated(other.radius);

        match a.get_y_range_at(other.center.x, other.center.z) {
            Some((bot_y, top_y)) => {
                let Segment { start, end } = other.get_seg();

                if end.y < bot_y || top_y < start.y {
                    false
                } else {
                    true
                }
            },
            None => false,
        }
    }

    fn check_static_collision_details(&self, other: &Capsule) -> Option<CollisionDetails> {
        use mod_math::Segment;
        
        let (nearest1, nearest2) = Segment::each_nearest(&self.get_seg(), &other.get_seg());
        let normal = nearest1 - nearest2;
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

impl StaticCollision<OrientedCapsule> for Capsule {
    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
    fn check_static_collision(&self, other: &OrientedCapsule) -> bool {
        other.check_static_collision(self)
    }

    fn check_static_collision_details(&self, other: &OrientedCapsule) -> Option<CollisionDetails> {
        let mut details = other.check_static_collision_details(self)?;
        details.normal = -details.normal;
        Some(details)
    }
}

impl StaticCollision<Sphere> for Capsule {
    fn check_static_collision(&self, sphere: &Sphere) -> bool {
        let capsule = self.inflated(sphere.radius);

        // self.center가 확장된 캡슐의 y축 범위에 포함되는지 확인
        match capsule.get_y_range_at(sphere.center.x, sphere.center.z) {
            Some((bot_y, top_y)) => bot_y <= sphere.center.y && sphere.center.y <= top_y,
            None =>                 false,
        }
    }

    fn check_static_collision_details(&self, sphere: &Sphere) -> Option<CollisionDetails> {
        let center = glam::Vec3A::from(sphere.center);
        let nearest = self.get_seg().nearest_to_point(&center);
        let from_center = nearest - center;
        let distance = from_center.length();
        let penetration = sphere.radius + self.radius - distance;
        if penetration < 0.0 {
            return None;
        }

        let normal = from_center.normalize_or_zero();
        Some(CollisionDetails {
            normal,
            penetration,
        })
    }
}
