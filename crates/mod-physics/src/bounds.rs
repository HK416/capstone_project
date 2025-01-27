#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub center: glam::Vec3,
    pub extents: glam::Vec3,
    pub rotation: Option<[[f32; 3]; 3]>, // OBB를 위함
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            center: glam::Vec3::ZERO,
            extents: glam::Vec3::ZERO,
            rotation: None, // default None B/C AABB가 default 인 상태
        }
    }
}

impl BoundingBox {
    // AABB collision detection
    pub fn aabb_collision(&self, other: &BoundingBox) -> bool {
        let x_overlap = (self.center.x - other.center.x).abs() <= (self.extents.x + other.extents.x);
        let y_overlap = (self.center.y - other.center.y).abs() <= (self.extents.y + other.extents.y);
        let z_overlap = (self.center.z - other.center.z).abs() <= (self.extents.z + other.extents.z);

        x_overlap && y_overlap && z_overlap
    }

    // OBB를 축에 투영하고 투영 간격(최소, 최대)을 반환하는 메서드
    fn project_onto_axis(&self, axis: &glam::Vec3) -> (f32, f32) {
        let vertices = self.get_vertices();
        
        //각 정점을 축에 투영하는 dot product 계산
        let projections: Vec<f32> = vertices.iter().map(|v| {
            v.x * axis.x + v.y * axis.y + v.z * axis.z
        }).collect();

        let min_proj = *projections.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let max_proj = *projections.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        (min_proj, max_proj)
    }

    // 두 OBB가 주어진 축에서 겹치는지 확인
    fn overlaps_on_axis(&self, other: &BoundingBox, axis: &glam::Vec3) -> bool {
        let (min_a, max_a) = self.project_onto_axis(axis);
        let (min_b, max_b) = other.project_onto_axis(axis);

        // 축에서 투영이 겹치는지 확인
        max_a >= min_b && max_b >= min_a
    }

    // SAT 를 이용한 OBB collision detection
    pub fn obb_collision(&self, other: &BoundingBox) -> bool {
        if let (Some(self_rot), Some(other_rot)) = (self.rotation, other.rotation) {
            let self_axes = self.get_axes(self_rot);
            let other_axes = other.get_axes(other_rot);

            // cross products > vector
            let cross_products: Vec<glam::Vec3> = self_axes.iter().flat_map(|&a1| {
                other_axes.iter().map(move |&a2| a1.cross(a2)) // Cross products
            }).collect();

            // 모든 축 확인
            let axes_to_test = self_axes.iter()
                .chain(other_axes.iter()) // 양 OBB의 지역 축
                .chain(cross_products.iter()); // Cross product 축

            for axis in axes_to_test {
                if !self.overlaps_on_axis(other, axis) {
                    return false; // if 분리된 축이 존재 = 충돌 없음
                }
            }

            true // 분리된 축 없음 = 충돌
        } else {
            // 어느 박스든 회전이 없으면(AABB), AABB 충돌 검사로 대체
            self.aabb_collision(other)
        }
    }

    // OBB의 지역 축 가져오기 (회전 행렬의 열)
    fn get_axes(&self, rotation: [[f32; 3]; 3]) -> [glam::Vec3; 3] {
        [
            glam::Vec3::new(rotation[0][0], rotation[1][0], rotation[2][0]), // X
            glam::Vec3::new(rotation[0][1], rotation[1][1], rotation[2][1]), // Y
            glam::Vec3::new(rotation[0][2], rotation[1][2], rotation[2][2]), // Z
        ]
    }

    // 월드 공간에서 OBB의 정점 가져오기
    fn get_vertices(&self) -> [glam::Vec3; 8] {
        let extents = glam::Vec3::new(self.extents.x, self.extents.y, self.extents.z);

        if let Some(rotation) = self.rotation {
            // OBB 정점
            [
                self.center + self.rotation_mul(glam::Vec3::new(1.0 * extents.x, 1.0 * extents.y, 1.0 * extents.z), rotation),
                self.center + self.rotation_mul(glam::Vec3::new(-1.0 * extents.x, 1.0 * extents.y, 1.0 * extents.z), rotation),
                self.center + self.rotation_mul(glam::Vec3::new(1.0 * extents.x, -1.0 * extents.y, 1.0 * extents.z), rotation),
                self.center + self.rotation_mul(glam::Vec3::new(-1.0 * extents.x, -1.0 * extents.y, 1.0 * extents.z), rotation),
                self.center + self.rotation_mul(glam::Vec3::new(1.0 * extents.x, 1.0 * extents.y, -1.0 * extents.z), rotation),
                self.center + self.rotation_mul(glam::Vec3::new(-1.0 * extents.x, 1.0 * extents.y, -1.0 * extents.z), rotation),
                self.center + self.rotation_mul(glam::Vec3::new(1.0 * extents.x, -1.0 * extents.y, -1.0 * extents.z), rotation),
                self.center + self.rotation_mul(glam::Vec3::new(-1.0 * extents.x, -1.0 * extents.y, -1.0 * extents.z), rotation),
            ]
        } else {
            // AABB 정점 (회전 없음)
            [
                self.center + glam::Vec3::new(1.0 * extents.x, 1.0 * extents.y, 1.0 * extents.z),
                self.center + glam::Vec3::new(-1.0 * extents.x, 1.0 * extents.y, 1.0 * extents.z),
                self.center + glam::Vec3::new(1.0 * extents.x, -1.0 * extents.y, 1.0 * extents.z),
                self.center + glam::Vec3::new(-1.0 * extents.x, -1.0 * extents.y, 1.0 * extents.z),
                self.center + glam::Vec3::new(1.0 * extents.x, 1.0 * extents.y, -1.0 * extents.z),
                self.center + glam::Vec3::new(-1.0 * extents.x, 1.0 * extents.y, -1.0 * extents.z),
                self.center + glam::Vec3::new(1.0 * extents.x, -1.0 * extents.y, -1.0 * extents.z),
                self.center + glam::Vec3::new(-1.0 * extents.x, -1.0 * extents.y, -1.0 * extents.z),
            ]
        }
    }

    // 매트릭스-벡터 곱셈
    fn rotation_mul(&self, vec: glam::Vec3, rotation: [[f32; 3]; 3]) -> glam::Vec3 {
        glam::Vec3::new(
            rotation[0][0] * vec.x + rotation[0][1] * vec.y + rotation[0][2] * vec.z,
            rotation[1][0] * vec.x + rotation[1][1] * vec.y + rotation[1][2] * vec.z,
            rotation[2][0] * vec.x + rotation[2][1] * vec.y + rotation[2][2] * vec.z,
        )
    }
}
