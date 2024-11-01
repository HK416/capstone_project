use crate::render::mesh::{AttributeValues, Mesh, Vertices};



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

        let num_vertices = num_width * num_depth * 6;
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut texcoords0 = Vec::with_capacity(num_vertices);
        let mut texcoords1 = Vec::with_capacity(num_vertices);
        for z in 0..num_depth {
            for x in 0..num_width {
                let dx = (x + 0) as f32 * spacing;
                let dz = (z + 0) as f32 * spacing;
                let pos_x = x_start + dx;
                let pos_z = z_start + dz;
                positions.push([pos_x, 0.0, pos_z].into());
                normals.push([0.0, 1.0, 0.0].into());
                texcoords0.push([dx / width, dz / depth].into());
                texcoords1.push([0.0, 1.0].into());

                let dx = (x + 0) as f32 * spacing;
                let dz = (z + 1) as f32 * spacing;
                let pos_x = x_start + dx;
                let pos_z = z_start + dz;
                positions.push([pos_x, 0.0, pos_z].into());
                normals.push([0.0, 1.0, 0.0].into());
                texcoords0.push([dx / width, dz / depth].into());
                texcoords1.push([0.0, 0.0].into());

                let dx = (x + 1) as f32 * spacing;
                let dz = (z + 0) as f32 * spacing;
                let pos_x = x_start + dx;
                let pos_z = z_start + dz;
                positions.push([pos_x, 0.0, pos_z].into());
                normals.push([0.0, 1.0, 0.0].into());
                texcoords0.push([dx / width, dz / depth].into());
                texcoords1.push([1.0, 1.0].into());


                let dx = (x + 1) as f32 * spacing;
                let dz = (z + 0) as f32 * spacing;
                let pos_x = x_start + dx;
                let pos_z = z_start + dz;
                positions.push([pos_x, 0.0, pos_z].into());
                normals.push([0.0, 1.0, 0.0].into());
                texcoords0.push([dx / width, dz / depth].into());
                texcoords1.push([1.0, 1.0].into());

                let dx = (x + 0) as f32 * spacing;
                let dz = (z + 1) as f32 * spacing;
                let pos_x = x_start + dx;
                let pos_z = z_start + dz;
                positions.push([pos_x, 0.0, pos_z].into());
                normals.push([0.0, 1.0, 0.0].into());
                texcoords0.push([dx / width, dz / depth].into());
                texcoords1.push([0.0, 1.0].into());

                let dx = (x + 1) as f32 * spacing;
                let dz = (z + 1) as f32 * spacing;
                let pos_x = x_start + dx;
                let pos_z = z_start + dz;
                positions.push([pos_x, 0.0, pos_z].into());
                normals.push([0.0, 1.0, 0.0].into());
                texcoords0.push([dx / width, dz / depth].into());
                texcoords1.push([1.0, 0.0].into());
            }
        }

        let mut mesh = Mesh::new(name, device, queue, Vertices(positions));
        mesh.insert_attribute(device, queue, AttributeValues::Normal(normals));
        mesh.insert_attribute(device, queue, AttributeValues::Texcoord0(texcoords0));
        mesh.insert_attribute(device, queue, AttributeValues::Texcoord1(texcoords1));
        mesh
    }
}
