use super::{BoundingBox, Capsule, OrientedBoundingBox, Plane, Sphere};

/// 절두체(Frustum)을 나타내는 구조체입니다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// 주어진 행렬(뷰 변환 행렬과 투영 변환 행렬이 곱해진)로부터 절두체를 생성합니다.
    pub fn from_mat4(m: glam::Mat4) -> Self {
        Self {
            planes: [
                Plane::from_vec4(m.row(3) + m.row(0)), // Left
                Plane::from_vec4(m.row(3) - m.row(0)), // Right
                Plane::from_vec4(m.row(3) + m.row(1)), // Bottom
                Plane::from_vec4(m.row(3) - m.row(1)), // Top
                Plane::from_vec4(m.row(3) + m.row(2)), // Near
                Plane::from_vec4(m.row(3) - m.row(2)), // Far
            ],
        }
    }

    /// 구체가 절두체와 충돌하는지 여부를 반환합니다.
    pub fn sphere_test(&self, sphere: &Sphere) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.distance(sphere.center) >= -sphere.radius)
    }

    /// 축에 정렬된 상자가 절두체와 충돌하는지 여부를 반환합니다.
    pub fn aabb_test(&self, aabb: &BoundingBox) -> bool {
        let corner = aabb.get_vertices();
        self.planes
            .iter()
            .all(|plane| corner.iter().any(|&c| plane.distance(c) >= 0.0))
    }

    /// 방향성 있는 상자가 절두체와 충돌하는지 여부를 반환합니다.
    pub fn obb_test(&self, obb: &OrientedBoundingBox) -> bool {
        let corner = obb.get_vertices();
        self.planes
            .iter()
            .all(|plane| corner.iter().any(|&c| plane.distance(c) >= 0.0))
    }

    /// 캡슐과 절두체가 충돌하는지 여부를 반환합니다.
    pub fn capsule_test(&self, capsule: &Capsule) -> bool {
        let radius = capsule.radius;
        let segment = capsule.get_seg();
        self.planes.iter().all(|plane| {
            let d1 = plane.distance(segment.start);
            let d2 = plane.distance(segment.end);
            d1 >= -radius || d2 >= -radius
        })
    }
}
