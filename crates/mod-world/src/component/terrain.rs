use crate::render::mesh::{AttributeValues, Indices, Mesh, Vertices};



/// Height Map을 사용하는 지형 오브젝트를 생성합니다.
pub struct TerrainFactory;

impl TerrainFactory {
    /// 주어진 `width`, `depth`, `spacing`으로 Height Map 지형 메쉬를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 `width`, `depth`가 `spacing`보다 작은 경우 [`panic!`]을 호출합니다.
    /// 
    pub fn mesh(
        name: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        width: f32, 
        depth: f32, 
        spacing: f32, 
    ) -> Mesh {
        assert!(spacing <= width && spacing <= depth, "The given `width` and `depth` must be greater than or equal to `spacing`.");
        let name = name.unwrap_or("Unknown");
        let num_width = (width / spacing) as usize; // `width`의 개수
        let num_depth = (depth / spacing) as usize; // `depth`의 개수

        let x_start = -0.5 * width; // `x` 좌표의 시작 위치
        let z_start = -0.5 * depth; // `z` 좌표의 시작 위치

        let num_vertices = (num_width + 1) * (num_depth + 1);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut texcoords0 = Vec::with_capacity(num_vertices);
        let mut texcoords1 = Vec::with_capacity(num_vertices);
        for z in 0..(num_depth + 1) {
            for x in 0..(num_width + 1) {
                let delta_x = x as f32 * spacing;
                let delta_z = z as f32 * spacing;
                positions.push(gmm::Float3::new(x_start + delta_x, 0.0, z_start + delta_z));
                texcoords0.push(gmm::Float2::new(delta_x / width, delta_z / depth));
                texcoords1.push(gmm::Float2::new(
                    if x % 2 == 0 { 0.0 } else { 1.0 }, 
                    if z % 2 == 0 { 0.0 } else { 1.0 }
                ));
            }
        }

        let num_indices = (num_width + 1) * num_depth * 2;
        let mut indices = Vec::with_capacity(num_indices);

        // TriangleStrip 인덱스 생성
        for z in 0..num_depth {
            if z % 2 == 0 {
                for x in 0..(num_width + 1) {
                    indices.push((z * (num_width + 1) + x) as u32);
                    indices.push(((z + 1) * (num_width + 1) + x) as u32);
                }
            } else {
                for x in (0..(num_width + 1)).rev() {
                    indices.push(((z + 1) * (num_width + 1) + x) as u32);
                    indices.push((z * (num_width + 1) + x) as u32);
                }
            }
        }

        let mut mesh = Mesh::new(name, device, queue, Vertices(positions));
        mesh.insert_attribute(device, queue, AttributeValues::Texcoord0(texcoords0));
        mesh.insert_attribute(device, queue, AttributeValues::Texcoord1(texcoords1));
        mesh.insert_submesh(device, queue, Indices::U32(indices));
        mesh
    }
}
