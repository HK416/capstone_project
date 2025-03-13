use crate::{
    object3d::{BoundingBox, Capsule, Sphere}, 
    collision::{CollisionDetails, ConvexHull},
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
            Collider::Box(b) => self.check_box_collision(b),
            Collider::Capsule(c) => self.check_capsule_collision(c),
            Collider::Sphere(s) => self.check_sphere_collision(s),
        }
    }

    pub fn check_collision_details(&self, other: &Self) -> Option<CollisionDetails> {
        match other {
            Collider::Box(b) => self.check_box_collision_details(b),
            Collider::Capsule(c) => self.check_capsule_collision_details(c),
            Collider::Sphere(s) => self.check_sphere_collision_details(s),
        }
    }


    fn check_box_collision(&self, other: &BoundingBox) -> bool {
        match self {
            Collider::Box(b) => b.check_boundingbox_collision(other),
            Collider::Capsule(c) => c.gjk(other).is_some(),
            Collider::Sphere(s) => s.gjk(other).is_some(),
        }
    }
    
    fn check_capsule_collision(&self, other: &Capsule) -> bool {
        match self {
            Collider::Box(b) => b.gjk(other).is_some(),
            Collider::Capsule(c) => c.check_capsule_collision(other),
            Collider::Sphere(s) => s.check_capsule_collision(other),
        }
    }

    fn check_sphere_collision(&self, other: &Sphere) -> bool {
        match self {
            Collider::Box(b) => b.gjk(other).is_some(),
            Collider::Capsule(c) => c.check_sphere_collision(other),
            Collider::Sphere(s) => s.check_sphere_collision(other),
        }
    }

    fn check_box_collision_details(&self, other: &BoundingBox) -> Option<CollisionDetails> {
        match self {
            Collider::Box(b) => b.gjk_epa(other),
            Collider::Capsule(c) => c.gjk_epa(other),
            Collider::Sphere(s) => s.gjk_epa(other),
        }
    }

    fn check_capsule_collision_details(&self, other: &Capsule) -> Option<CollisionDetails> {
        match self {
            Collider::Box(b) => b.gjk_epa(other),
            Collider::Capsule(c) => c.check_capsule_collision_details(other),
            Collider::Sphere(s) => s.check_capsule_collision_details(other),
        }
    }

    fn check_sphere_collision_details(&self, other: &Sphere) -> Option<CollisionDetails> {
        match self {
            Collider::Box(b) => b.gjk_epa(other),
            Collider::Capsule(c) => c.check_sphere_collision_details(other),
            Collider::Sphere(s) => s.check_sphere_collision_details(other),
        }
    }
}


impl BoundingBox {
    pub fn check_boundingbox_collision(&self, other: &BoundingBox) -> bool {
        if self.rotation().is_some() || other.rotation().is_some() {
            self.obb_collision(other)
        } else {
            self.aabb_collision(other)
        }
    }
}

impl Sphere {
    pub fn check_sphere_collision(&self, other: &Sphere) -> bool {
        let center1 = glam::Vec3A::from(self.center);
        let center2 = glam::Vec3A::from(other.center);
        (center1 - center2).length_squared() <= (self.radius + other.radius).powi(2)
    }

    pub fn check_capsule_collision(&self, capsule: &Capsule) -> bool {
        capsule.check_sphere_collision(self)
    }

    pub fn check_sphere_collision_details(&self, other: &Sphere) -> Option<CollisionDetails> {
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

    pub fn check_capsule_collision_details(&self, capsule: &Capsule) -> Option<CollisionDetails> {
        let collision_details = capsule.check_sphere_collision_details(self)?;
        Some(CollisionDetails {
            normal: -collision_details.normal,
            penetration: collision_details.penetration,
        })
    }
}

impl Capsule {
    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
    /// 
    /// 두 선분 사이의 거리를 구하는 것과 같은가?  
    /// 캡슐은 한 선분에서 radius거리 이내의 모든 점의 집합  
    /// 따라서 두 선분의 최소 거리가 self.radius + other.radius보다 작거나 같으면 충돌한다.  
    pub fn check_capsule_collision(&self, other: &Capsule) -> bool {
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

    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.
    /// 
    /// Capsule의 크기를 sphere의 radius만큼 확장하고 sphere의 중심점과 충돌하는지 체크
    pub fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        let capsule = self.inflated(sphere.radius);

        capsule.check_point_collision(&sphere.center.into())
    }

    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.  
    pub fn check_capsule_collision_details(&self, other: &Capsule) -> Option<CollisionDetails> {
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

    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.  
    pub fn check_sphere_collision_details(&self, sphere: &Sphere) -> Option<CollisionDetails> {
        let center = glam::Vec3A::from(sphere.center);
        let nearest = self.get_seg().nearest_to_point(&center);
        let normal = nearest - center;
        let distance = normal.length();
        let penetration = self.radius + sphere.radius - distance;
        if penetration < 0.0 {
            return None;
        }

        Some(CollisionDetails {
            normal: normal.normalize_or_zero(),
            penetration,
        })
    }
}
