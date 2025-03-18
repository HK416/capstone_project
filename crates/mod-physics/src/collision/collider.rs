use crate::{
    object3d::{BoundingBox, OrientedBoundingBox, VertexBox, Capsule, Sphere}, 
    collision::{CollisionDetails, ConvexHull},
};


#[derive(Debug, Clone)]
pub enum Collider {
    Aabb(BoundingBox),
    Obb(OrientedBoundingBox),
    Capsule(Capsule),
    Sphere(Sphere),
}

impl Collider {
    pub fn check_collision(&self, other: &Self) -> bool {
        match other {
            Collider::Aabb(b) => self.check_aabb_collision(b),
            Collider::Obb(b) => self.check_obb_collision(b),
            Collider::Capsule(c) => self.check_capsule_collision(c),
            Collider::Sphere(s) => self.check_sphere_collision(s),
        }
    }

    pub fn check_collision_details(&self, other: &Self) -> Option<CollisionDetails> {
        match other {
            Collider::Aabb(b) => self.check_aabb_collision_details(b),
            Collider::Obb(b) => self.check_obb_collision_details(b),
            Collider::Capsule(c) => self.check_capsule_collision_details(c),
            Collider::Sphere(s) => self.check_sphere_collision_details(s),
        }
    }


    fn check_aabb_collision(&self, other: &BoundingBox) -> bool {
        match self {
            Collider::Aabb(b) => b.check_aabb_collision(other),
            Collider::Obb(b) => b.check_aabb_collision(other),
            Collider::Capsule(c) => c.gjk(other).is_some(),
            Collider::Sphere(s) => s.gjk(other).is_some(),
        }
    }

    fn check_obb_collision(&self, other: &OrientedBoundingBox) -> bool {
        match self {
            Collider::Aabb(b) => b.check_obb_collision(other),
            Collider::Obb(b) => b.check_obb_collision(other),
            Collider::Capsule(c) => c.gjk(other).is_some(),
            Collider::Sphere(s) => s.gjk(other).is_some(),
        }
    }
    
    fn check_capsule_collision(&self, other: &Capsule) -> bool {
        match self {
            Collider::Aabb(b) => b.gjk(other).is_some(),
            Collider::Obb(b) => b.gjk(other).is_some(),
            Collider::Capsule(c) => c.check_capsule_collision(other),
            Collider::Sphere(s) => s.check_capsule_collision(other),
        }
    }

    fn check_sphere_collision(&self, other: &Sphere) -> bool {
        match self {
            Collider::Aabb(b) => b.gjk(other).is_some(),
            Collider::Obb(b) => b.gjk(other).is_some(),
            Collider::Capsule(c) => c.check_sphere_collision(other),
            Collider::Sphere(s) => s.check_sphere_collision(other),
        }
    }

    fn check_aabb_collision_details(&self, other: &BoundingBox) -> Option<CollisionDetails> {
        match self {
            Collider::Aabb(b) => b.check_aabb_collision_details(other),
            Collider::Obb(b) => b.check_aabb_collision_details(other),
            Collider::Capsule(c) => c.gjk_epa(other),
            Collider::Sphere(s) => s.gjk_epa(other),
        }
    }

    fn check_obb_collision_details(&self, other: &OrientedBoundingBox) -> Option<CollisionDetails> {
        match self {
            Collider::Aabb(b) => b.check_obb_collision_details(other),
            Collider::Obb(b) => b.check_obb_collision_details(other),
            Collider::Capsule(c) => c.gjk_epa(other),
            Collider::Sphere(s) => s.gjk_epa(other),
        }
    }

    fn check_capsule_collision_details(&self, other: &Capsule) -> Option<CollisionDetails> {
        match self {
            Collider::Aabb(b) => b.gjk_epa(other),
            Collider::Obb(b) => b.gjk_epa(other),
            Collider::Capsule(c) => c.check_capsule_collision_details(other),
            Collider::Sphere(s) => s.check_capsule_collision_details(other),
        }
    }

    fn check_sphere_collision_details(&self, other: &Sphere) -> Option<CollisionDetails> {
        match self {
            Collider::Aabb(b) => b.gjk_epa(other),
            Collider::Obb(b) => b.gjk_epa(other),
            Collider::Capsule(c) => c.check_sphere_collision_details(other),
            Collider::Sphere(s) => s.check_sphere_collision_details(other),
        }
    }
}


impl BoundingBox {
    pub fn check_aabb_collision(&self, other: &BoundingBox) -> bool {
        let glam::Vec3 { x: ex1, y: ey1, z: ez1 } = self.extents();
        let glam::Vec3 { x: ex2, y: ey2, z: ez2 } = other.extents();
        let x_overlap = (self.center.x - other.center.x).abs() <= (ex1 + ex2);
        let y_overlap = (self.center.y - other.center.y).abs() <= (ey1 + ey2);
        let z_overlap = (self.center.z - other.center.z).abs() <= (ez1 + ez2);

        x_overlap && y_overlap && z_overlap
    }

    pub fn check_obb_collision(&self, obb: &OrientedBoundingBox) -> bool {
        let this = OrientedBoundingBox::new(
            glam::Vec3::ZERO, 
            self.extents(), 
            glam::Mat3::IDENTITY
        );
        this.check_obb_collision(&obb)
    }

    pub fn check_aabb_collision_details(&self, other: &BoundingBox) -> Option<CollisionDetails> {
        let min_a = self.center - self.extents();
        let max_a = self.center + self.extents();
        let min_b = other.center - other.extents();
        let max_b = other.center + other.extents();

        let overlap_min = min_a.max(min_b);
        let overlap_max = max_a.min(max_b);

        let mut min_penetration = f32::MAX;
        let mut min_element = 0;

        for i in 0..3 {
            let penetration = if overlap_min[i] <= overlap_max[i] {
                // 겹쳤을 경우, 겹침의 깊이를 계산
                let mid_a = (max_a[i] + min_a[i]) * 0.5;
                let mid_b = (max_b[i] + min_b[i]) * 0.5;
                if mid_a < mid_b {
                    // self가 other보다 왼쪽에 있을 때
                    overlap_min[i] - overlap_max[i]
                } else {
                    // self가 other보다 오른쪽에 있을 때
                    overlap_max[i] - overlap_min[i]
                }
            } else {
                // 겹치지 않는 경우
                return None;
            };

            if penetration.abs() < min_penetration.abs() {
                min_penetration = penetration;
                min_element = i;
            }
        }

        let mut collision_normal = match min_element {
            0 => glam::Vec3A::X,
            1 => glam::Vec3A::Y,
            2 => glam::Vec3A::Z,
            _ => glam::Vec3A::ZERO,
        };

        if min_penetration == 0.0 {
            collision_normal = glam::Vec3A::ZERO;
        }

        Some(CollisionDetails {
            normal: collision_normal,
            penetration: min_penetration,
        })
    }

    pub fn check_obb_collision_details(&self, obb: &OrientedBoundingBox) -> Option<CollisionDetails> {
        let this = OrientedBoundingBox::new(
            glam::Vec3::ZERO, 
            self.extents(), 
            glam::Mat3::IDENTITY
        );
        this.check_obb_collision_details(&obb)
    }
}

impl OrientedBoundingBox {
    /// SAT 를 이용한 OBB collision detection
    pub fn check_obb_collision(&self, other: &OrientedBoundingBox) -> bool {
        let self_axes = self.get_axes();
        let other_axes = other.get_axes();

        // cross products > vector
        let cross_products: [glam::Vec3A; 9] = [
            self_axes[0].cross(other_axes[0]),
            self_axes[0].cross(other_axes[1]),
            self_axes[0].cross(other_axes[2]),
            self_axes[1].cross(other_axes[0]),
            self_axes[1].cross(other_axes[1]),
            self_axes[1].cross(other_axes[2]),
            self_axes[2].cross(other_axes[0]),
            self_axes[2].cross(other_axes[1]),
            self_axes[2].cross(other_axes[2]),
        ];

        // 모든 축 확인
        let axes_to_test = self_axes.iter()
            .chain(other_axes.iter())       // 양 OBB의 지역 축
            .chain(cross_products.iter())   // Cross product 축
            .filter(|&axis| !axis.is_nan() && *axis != glam::Vec3A::ZERO); // NaN, Zero 제외

        let vbox1 = VertexBox::from(self);
        let vbox2 = VertexBox::from(other);

        for axis in axes_to_test {
            if !vbox1.overlaps_on_axis(&vbox2, axis) {
                return false; // if 분리된 축이 존재 = 충돌 없음
            }
        }

        true // 분리된 축 없음 = 충돌
    }

    pub fn check_aabb_collision(&self, aabb: &BoundingBox) -> bool {
        let aabb = OrientedBoundingBox::new(
            aabb.center, 
            aabb.extents(), 
            glam::Mat3::IDENTITY
        );
        self.check_obb_collision(&aabb)
    }

    /// SAT 를 이용한 OBB collision detection + 충돌 상세 정보 반환
    pub fn check_obb_collision_details(&self, other: &OrientedBoundingBox) -> Option<CollisionDetails> {
        let self_axes = self.get_axes();
        let other_axes = other.get_axes();

        // cross products > vector
        let cross_products: [glam::Vec3A; 9] = [
            self_axes[0].cross(other_axes[0]),
            self_axes[0].cross(other_axes[1]),
            self_axes[0].cross(other_axes[2]),
            self_axes[1].cross(other_axes[0]),
            self_axes[1].cross(other_axes[1]),
            self_axes[1].cross(other_axes[2]),
            self_axes[2].cross(other_axes[0]),
            self_axes[2].cross(other_axes[1]),
            self_axes[2].cross(other_axes[2]),
        ];

        // 모든 축 확인
        let axes_to_test = self_axes.iter()
            .chain(other_axes.iter())       // 양 OBB의 지역 축
            .chain(cross_products.iter())   // Cross product 축
            .filter(|&axis| !axis.is_nan() && *axis != glam::Vec3A::ZERO); // NaN, Zero 제외

        let vbox1 = VertexBox::from(self);
        let vbox2 = VertexBox::from(other);

        let mut min_penetration = f32::MAX;
        let mut collision_normal = glam::Vec3A::ZERO;

        for axis in axes_to_test {
            match vbox1.overlaps_length_on_axis(&vbox2, axis) {
                Some(penetration) => {
                    if penetration.abs() < min_penetration.abs() {
                        min_penetration = penetration;
                        collision_normal = axis.normalize();  // 최소 침투가 있는 축을 충돌 노말로 설정
                    }
                }
                None => return None, // 분리된 축이 존재 => 충돌 없음
            }
        }

        if min_penetration != f32::MAX {
            Some(CollisionDetails {
                normal: collision_normal,
                penetration: min_penetration,
            })
        } else {
            None // 침투가 없으면 충돌 없음
        }
    }

    pub fn check_aabb_collision_details(&self, aabb: &BoundingBox) -> Option<CollisionDetails> {
        let aabb = OrientedBoundingBox::new(
            aabb.center, 
            aabb.extents(), 
            glam::Mat3::IDENTITY
        );
        self.check_obb_collision_details(&aabb)
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
