use crate::object3d::{
    BoundingBox, OrientedBoundingBox, VertexBox, 
    Capsule, OrientedCapsule,
    Sphere
};
use super::{CollisionDetails, ConvexHull, StaticCollision};


impl StaticCollision<BoundingBox> for OrientedBoundingBox {
    fn check_static_collision(&self, aabb: &BoundingBox) -> bool {
        let aabb = OrientedBoundingBox::new(
            aabb.center, 
            aabb.extents(), 
            glam::Mat3::IDENTITY
        );
        self.check_static_collision(&aabb)
    }

    fn check_static_collision_details(&self, aabb: &BoundingBox) -> Option<CollisionDetails> {
        let aabb = OrientedBoundingBox::new(
            aabb.center, 
            aabb.extents(), 
            glam::Mat3::IDENTITY
        );
        self.check_static_collision_details(&aabb)
    }
}

impl StaticCollision<OrientedBoundingBox> for OrientedBoundingBox {
    /// SAT 를 이용한 OBB collision detection
    fn check_static_collision(&self, other: &OrientedBoundingBox) -> bool {
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

    /// SAT 를 이용한 OBB collision detection + 충돌 상세 정보 반환
    fn check_static_collision_details(&self, other: &OrientedBoundingBox) -> Option<CollisionDetails> {
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
}

impl StaticCollision<Capsule> for OrientedBoundingBox {
    fn check_static_collision(&self, capsule: &Capsule) -> bool {
        self.gjk(capsule).is_some()
    }

    fn check_static_collision_details(&self, capsule: &Capsule) -> Option<CollisionDetails> {
        self.gjk_epa(capsule)
    }
}

impl StaticCollision<OrientedCapsule> for OrientedBoundingBox {
    fn check_static_collision(&self, capsule: &OrientedCapsule) -> bool {
        self.gjk(capsule).is_some()
    }

    fn check_static_collision_details(&self, capsule: &OrientedCapsule) -> Option<CollisionDetails> {
        self.gjk_epa(capsule)
    }
}

impl StaticCollision<Sphere> for OrientedBoundingBox {
    fn check_static_collision(&self, sphere: &Sphere) -> bool {
        // Sphere를 BoundingBox의 로컬 공간으로 변환
        let inv_rotation = self.rotation().transpose();    // 회전행렬의 전치행렬은 역행렬과 같다.
        let local_sphere_center = inv_rotation * (sphere.center - self.center);

        let aabb = BoundingBox::new(
            glam::Vec3::ZERO, 
            self.extents(), 
        );
        let sphere = Sphere {
            center: local_sphere_center, 
            radius: sphere.radius
        };
        
        aabb.check_static_collision(&sphere)
    }

    fn check_static_collision_details(&self, sphere: &Sphere) -> Option<CollisionDetails> {
        // Sphere를 BoundingBox의 로컬 공간으로 변환
        let rotation = self.rotation();
        let inv_rotation = rotation.transpose();    // 회전행렬의 전치행렬은 역행렬과 같다.
        let local_sphere_center = inv_rotation * (sphere.center - self.center);

        let aabb = BoundingBox::new(
            glam::Vec3::ZERO, 
            self.extents(), 
        );
        let sphere = Sphere {
            center: local_sphere_center, 
            radius: sphere.radius
        };
        
        let mut details = aabb.check_static_collision_details(&sphere)?;
        details.normal = rotation * details.normal;
        Some(details)
    }
}
