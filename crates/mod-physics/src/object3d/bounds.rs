#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub center: glam::Vec3,
    /// center로부터 x, y, z 방향으로 확장되는 길이  
    /// extents: (0.5, 0.5, 0.5) 인 경우 박스의 크기는 (1, 1, 1)  
    /// 음수는 허용하지 않음  
    extents: glam::Vec3,
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            center: glam::Vec3::ZERO,
            extents: glam::Vec3::ZERO,
        }
    }
}

impl BoundingBox {
    /// Axis-Aligned Bounding Box 생성
    pub fn new(center: glam::Vec3, extents: glam::Vec3) -> Self {
        Self {
            center,
            extents: extents.abs(), // extents는 음수가 될 수 없음
        }
    }

    pub fn extents(&self) -> glam::Vec3 {
        self.extents
    }

    // 월드 공간에서 OBB의 정점 가져오기
    pub fn get_vertices(&self) -> [glam::Vec3A; 8] {
        let center = glam::Vec3A::from(self.center);
        let extents = glam::Vec3A::from(self.extents);
        let vertices = [
            glam::Vec3A::new(1.0, 1.0, 1.0) * extents,
            glam::Vec3A::new(-1.0, 1.0, 1.0) * extents,
            glam::Vec3A::new(1.0, -1.0, 1.0) * extents,
            glam::Vec3A::new(-1.0, -1.0, 1.0) * extents,
            glam::Vec3A::new(1.0, 1.0, -1.0) * extents,
            glam::Vec3A::new(-1.0, 1.0, -1.0) * extents,
            glam::Vec3A::new(1.0, -1.0, -1.0) * extents,
            glam::Vec3A::new(-1.0, -1.0, -1.0) * extents,
        ];

        vertices.map(|v| center + v)
    }
}


#[derive(Debug, Clone, Copy)]
pub struct OrientedBoundingBox {
    pub center: glam::Vec3,
    /// center로부터 x, y, z 방향으로 확장되는 길이  
    /// extents: (0.5, 0.5, 0.5) 인 경우 박스의 크기는 (1, 1, 1)  
    /// 음수는 허용하지 않음  
    extents: glam::Vec3,
    rotation: glam::Mat3, // OBB를 위함
}

impl OrientedBoundingBox {
    /// Oriented Bounding Box 생성
    pub fn new(center: glam::Vec3, extents: glam::Vec3, rotation: glam::Mat3) -> Self {
        Self {
            center,
            extents: extents.abs(), // extents는 음수가 될 수 없음
            rotation: rotation,
        }
    }

    pub fn extents(&self) -> glam::Vec3 {
        self.extents
    }

    pub fn set_rotation(&mut self, rotation: glam::Mat3) {
        self.rotation = rotation;
    }

    pub fn rotation(&self) -> glam::Mat3 {
        self.rotation
    }


    // OBB의 지역 축 가져오기 (회전 행렬의 열)
    pub fn get_axes(&self) -> [glam::Vec3A; 3] {
        [
            glam::Vec3A::from(self.rotation.x_axis),
            glam::Vec3A::from(self.rotation.y_axis),
            glam::Vec3A::from(self.rotation.z_axis),
        ]
    }

    // 월드 공간에서 OBB의 정점 가져오기
    pub fn get_vertices(&self) -> [glam::Vec3A; 8] {
        let center = glam::Vec3A::from(self.center);
        let extents = glam::Vec3A::from(self.extents);
        let vertices = [
            glam::Vec3A::new(1.0, 1.0, 1.0) * extents,
            glam::Vec3A::new(-1.0, 1.0, 1.0) * extents,
            glam::Vec3A::new(1.0, -1.0, 1.0) * extents,
            glam::Vec3A::new(-1.0, -1.0, 1.0) * extents,
            glam::Vec3A::new(1.0, 1.0, -1.0) * extents,
            glam::Vec3A::new(-1.0, 1.0, -1.0) * extents,
            glam::Vec3A::new(1.0, -1.0, -1.0) * extents,
            glam::Vec3A::new(-1.0, -1.0, -1.0) * extents,
        ];

        vertices.map(|v| center + self.rotation * v)
    }
}


pub struct VertexBox {
    vertices: [glam::Vec3A; 8],
}

impl From<&BoundingBox> for VertexBox {
    fn from(boundingbox: &BoundingBox) -> Self {
        Self {
            vertices: boundingbox.get_vertices(),
        }
    }
}

impl From<&OrientedBoundingBox> for VertexBox {
    fn from(obb: &OrientedBoundingBox) -> Self {
        Self {
            vertices: obb.get_vertices(),
        }
    }
}

impl VertexBox {
    /// OBB를 축에 투영하고 투영 간격(최소, 최대)을 반환하는 메서드
    pub fn project_onto_axis(&self, axis: &glam::Vec3A) -> (f32, f32) {
        //각 정점을 축에 투영하는 dot product 계산
        let projections: [f32; 8] = [
            axis.dot(self.vertices[0]),
            axis.dot(self.vertices[1]),
            axis.dot(self.vertices[2]),
            axis.dot(self.vertices[3]),
            axis.dot(self.vertices[4]),
            axis.dot(self.vertices[5]),
            axis.dot(self.vertices[6]),
            axis.dot(self.vertices[7]),
        ];

        let min_proj = *projections.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let max_proj = *projections.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        (min_proj, max_proj)
    }

    /// 두 OBB가 주어진 축에서 겹치는지 확인
    pub fn overlaps_on_axis(&self, other: &VertexBox, axis: &glam::Vec3A) -> bool {
        let (min_a, max_a) = self.project_onto_axis(axis);
        let (min_b, max_b) = other.project_onto_axis(axis);

        // 축에서 투영이 겹치는지 확인
        max_a >= min_b && max_b >= min_a
    }

    /// 두 OBB가 주어진 축에서 겹치는지 확인, 겹쳐지는 길이 반환
    pub fn overlaps_length_on_axis(&self, other: &VertexBox, axis: &glam::Vec3A) -> Option<f32> {
        let (min_a, max_a) = self.project_onto_axis(axis);
        let (min_b, max_b) = other.project_onto_axis(axis);

        let overlap_min = min_a.max(min_b);
        let overlap_max = max_a.min(max_b);
        
        if overlap_min <= overlap_max {
            // 겹쳤을 경우, 겹침의 깊이를 반환
            let mid_a = (max_a + min_a) * 0.5;
            let mid_b = (max_b + min_b) * 0.5;
            if mid_a < mid_b {
                // self가 other보다 왼쪽에 있을 때
                Some(overlap_min - overlap_max)
            } else {
                // self가 other보다 오른쪽에 있을 때
                Some(overlap_max - overlap_min)
            }
        } else {
            // 겹치지 않는 경우
            None
        }
    }

    pub fn get_vertices(&self) -> &[glam::Vec3A; 8] {
        &self.vertices
    }
}