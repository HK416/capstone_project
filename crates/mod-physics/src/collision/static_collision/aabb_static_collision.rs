use crate::object3d::{
    BoundingBox, OrientedBoundingBox, 
    Capsule, OrientedCapsule,
    Sphere
};
use super::{ConvexHull, StaticCollision, StaticCollisionDetails};


impl StaticCollision<BoundingBox> for BoundingBox {
    fn check_static_collision(&self, other: &BoundingBox) -> bool {
        let glam::Vec3 { x: ex1, y: ey1, z: ez1 } = self.extents();
        let glam::Vec3 { x: ex2, y: ey2, z: ez2 } = other.extents();
        let x_overlap = (self.center.x - other.center.x).abs() <= (ex1 + ex2);
        let y_overlap = (self.center.y - other.center.y).abs() <= (ey1 + ey2);
        let z_overlap = (self.center.z - other.center.z).abs() <= (ez1 + ez2);

        x_overlap && y_overlap && z_overlap
    }

    fn check_static_collision_details(&self, other: &BoundingBox) -> Option<StaticCollisionDetails> {
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

        Some(StaticCollisionDetails {
            normal: collision_normal,
            penetration: min_penetration,
        })
    }
}

impl StaticCollision<OrientedBoundingBox> for BoundingBox {
    fn check_static_collision(&self, obb: &OrientedBoundingBox) -> bool {
        let this = OrientedBoundingBox::new(
            glam::Vec3::ZERO, 
            self.extents(), 
            glam::Mat3::IDENTITY
        );
        this.check_static_collision(obb)
    }

    fn check_static_collision_details(&self, obb: &OrientedBoundingBox) -> Option<StaticCollisionDetails> {
        let this = OrientedBoundingBox::new(
            glam::Vec3::ZERO, 
            self.extents(), 
            glam::Mat3::IDENTITY
        );
        this.check_static_collision_details(obb)
    }
}

impl StaticCollision<Capsule> for BoundingBox {
    fn check_static_collision(&self, capsule: &Capsule) -> bool {
        // capsule segment y min, max
        let cs_y_min = capsule.center.y + capsule.radius;
        let cs_y_max = capsule.center.y + capsule.height - capsule.radius;

        let nearest_y = self.center.y.clamp(cs_y_min, cs_y_max);

        let sphere = Sphere {
            center: glam::Vec3::new(capsule.center.x, nearest_y, capsule.center.z),
            radius: capsule.radius,
        };

        self.check_static_collision(&sphere)
    }

    fn check_static_collision_details(&self, capsule: &Capsule) -> Option<StaticCollisionDetails> {
        // capsule segment y min, max
        let cs_y_min = capsule.center.y + capsule.radius;
        let cs_y_max = capsule.center.y + capsule.height - capsule.radius;

        let nearest_y = self.center.y.clamp(cs_y_min, cs_y_max);

        let sphere = Sphere {
            center: glam::Vec3::new(capsule.center.x, nearest_y, capsule.center.z),
            radius: capsule.radius,
        };

        self.check_static_collision_details(&sphere)
    }
}

impl StaticCollision<OrientedCapsule> for BoundingBox {
    fn check_static_collision(&self, capsule: &OrientedCapsule) -> bool {
        self.gjk(capsule).is_some()
    }

    fn check_static_collision_details(&self, capsule: &OrientedCapsule) -> Option<StaticCollisionDetails> {
        self.gjk_epa(capsule)
    }
}

impl StaticCollision<Sphere> for BoundingBox {
    fn check_static_collision(&self, sphere: &Sphere) -> bool {
        let local_sphere_center = sphere.center - self.center;
        let aabb_extents = self.extents();
        let mut distance_sq = 0.0;

        for i in 0..3 {
            // 각 축에 대해 Box에서 벗어난 거리 측정
            let dist = local_sphere_center[i].abs() - aabb_extents[i];
            if dist > 0.0 {
                // dist <= 0.0인 경우는 거리 0으로 처리
                distance_sq += dist.powi(2);
            }
        }

        distance_sq <= sphere.radius.powi(2)
    }

    fn check_static_collision_details(&self, sphere: &Sphere) -> Option<StaticCollisionDetails> {
        let local_sphere_center = sphere.center - self.center;
        let aabb_extents = self.extents();
        let mut to_center = glam::Vec3::ZERO;

        for i in 0..3 {
            // 각 축에 대해 Box에서 벗어난 거리 측정
            let dist = local_sphere_center[i].abs() - aabb_extents[i];
            if dist >= 0.0 {
                to_center[i] = local_sphere_center[i].signum() * dist;
            }
        }

        let to_center = glam::Vec3A::from(to_center);
        let penetration = sphere.radius - to_center.length();

        if penetration < 0.0 {
            return None;
        }
        
        let normal = -to_center.normalize_or_zero();
        
        Some(StaticCollisionDetails {
            normal,
            penetration,
        })
    }
}
