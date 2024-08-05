use crate::render::mesh::IndexValues;
use crate::render::mesh::VertexAttributeValues;
use crate::render::mesh::ModelMesh3D;



/// 주어진 크기로 3차원 큐브 메쉬를 생성합니다.
/// 
/// # Panics
/// 주어진 크기가 음수일 경우 `panic!`을 호출합니다.
/// 
#[must_use]
pub fn create_cube_mesh(
    sx: f32, sy: f32, sz: f32, 
    device: &wgpu::Device, queue: &wgpu::Queue
) -> ModelMesh3D {
    assert!(sx >= 0.0 && sy >= 0.0 && sz >= 0.0, "The given size must be greater than zero!");

    let hx = 0.5 * sx;
    let hy = 0.5 * sy;
    let hz = 0.5 * sz;

    let indices = IndexValues::Uint16([
        0, 1, 2, 2, 3, 0,      // Back face
        4, 5, 6, 6, 7, 4,      // Front face
        8, 9, 10, 10, 11, 8,   // Left face
        12, 13, 14, 14, 15, 12, // Right face
        16, 17, 18, 18, 19, 16, // Bottom face
        20, 21, 22, 22, 23, 20  // Top face
    ].into());

    let positions = VertexAttributeValues::Float32x3([
        [-hx, -hy, -hz], [ hx, -hy, -hz], [ hx,  hy, -hz], 
        [-hx,  hy, -hz], [-hx, -hy,  hz], [ hx, -hy,  hz], 
        [ hx,  hy,  hz], [-hx,  hy,  hz], [-hx, -hy, -hz], 
        [-hx,  hy, -hz], [-hx,  hy,  hz], [-hx, -hy,  hz], 
        [ hx, -hy, -hz], [ hx,  hy, -hz], [ hx,  hy,  hz], 
        [ hx, -hy,  hz], [-hx, -hy, -hz], [ hx, -hy, -hz], 
        [ hx, -hy,  hz], [-hx, -hy,  hz], [-hx,  hy, -hz], 
        [ hx,  hy, -hz], [ hx,  hy,  hz], [-hx,  hy,  hz], 
    ].into());

    let normals = VertexAttributeValues::Float32x3([
        [ 0.0,  0.0, -1.0], [ 0.0,  0.0, -1.0], [ 0.0,  0.0, -1.0],
        [ 0.0,  0.0, -1.0], [ 0.0,  0.0,  1.0], [ 0.0,  0.0,  1.0],
        [ 0.0,  0.0,  1.0], [ 0.0,  0.0,  1.0], [-1.0,  0.0,  0.0],
        [-1.0,  0.0,  0.0], [-1.0,  0.0,  0.0], [-1.0,  0.0,  0.0],
        [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0],
        [ 1.0,  0.0,  0.0], [ 0.0, -1.0,  0.0], [ 0.0, -1.0,  0.0],
        [ 0.0, -1.0,  0.0], [ 0.0, -1.0,  0.0], [ 0.0,  1.0,  0.0],
        [ 0.0,  1.0,  0.0], [ 0.0,  1.0,  0.0], [ 0.0,  1.0,  0.0],
    ].into());

    let tangents = VertexAttributeValues::Float32x3([
        [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0], 
        [ 1.0,  0.0,  0.0], [-1.0,  0.0,  0.0], [-1.0,  0.0,  0.0], 
        [-1.0,  0.0,  0.0], [-1.0,  0.0,  0.0], [ 0.0,  0.0, -1.0], 
        [ 0.0,  0.0, -1.0], [ 0.0,  0.0, -1.0], [ 0.0,  0.0, -1.0], 
        [ 0.0,  0.0,  1.0], [ 0.0,  0.0,  1.0], [ 0.0,  0.0,  1.0], 
        [ 0.0,  0.0,  1.0], [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0], 
        [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0], 
        [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0], [ 1.0,  0.0,  0.0], 
    ].into());

    let texcoords = VertexAttributeValues::Float32x2([
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], 
        [0.0, 1.0], [0.0, 0.0], [1.0, 0.0], 
        [1.0, 1.0], [0.0, 1.0], [0.0, 0.0], 
        [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], 
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], 
        [0.0, 1.0], [0.0, 1.0], [1.0, 1.0], 
        [1.0, 0.0], [0.0, 0.0], [0.0, 1.0], 
        [1.0, 1.0], [1.0, 0.0], [0.0, 0.0], 
    ].into());

    let mut mesh = ModelMesh3D::new(&format!("Cube({}x{}x{})", sx, sy, sz));
    mesh.insert_indices(device, queue, indices);
    mesh.insert_position(device, queue, positions);
    mesh.insert_normal(device, queue, normals);
    mesh.insert_tangent(device, queue, tangents);
    mesh.insert_texcoord0(device, queue, texcoords);
    return mesh;
}
