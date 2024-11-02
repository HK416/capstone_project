use std::{mem, sync::{Arc, OnceLock}};

use mod_physics::BoundingBox;
use mod_world::render::{
    brush::MeshBrush, camera::CameraResource, material::{universal::{UniversalMaterialDescriptor, UniversalMaterialResource}, MaterialResource}, mesh::{Indices, Mesh, StaticMeshResource, Vertices}, DEPTH_STENCIL_FORMAT, SWAPCHAIN_FORMAT
};



/// [wgpu::ShaderModule]을 생성합니다.
#[inline]
#[must_use]
fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    #[cfg(feature = "enable-shader-validation")] {
        device.create_shader_module(
            wgpu::include_wgsl!(concat!(env!("CARGO_WORKSPACE_DIR"), "/assets/shaders/wireframe.wgsl"))
        )
    }
    #[cfg(not(feature = "enable-shader-validation"))] {
        unsafe {
            device.create_shader_module_unchecked(
                wgpu::include_wgsl!(concat!(env!("CARGO_WORKSPACE_DIR"), "/assets/shaders/wireframe.wgsl"))
            )
        }
    }
}



/// [wgpu::PipelineLayout]을 생성합니다.
#[inline]
#[must_use]
fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(BoundingBoxBrush)"), 
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device), 
                StaticMeshResource::bind_group_layout(device), 
                UniversalMaterialResource::bind_group_layout(device)
            ], 
            push_constant_ranges: &[] 
        }
    )
}



pub fn get_render_pipeline(device: &wgpu::Device) -> &'static wgpu::RenderPipeline {
    static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();
    PIPELINE.get_or_init(|| {
        let module = create_shader_module(device);
        let layout = create_pipeline_layout(device);
        device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(BoundingBoxBrush)"), 
                layout: Some(&layout), 
                vertex: wgpu::VertexState {
                    module: &module, 
                    entry_point: "vs_main", 
                    buffers: &[
                        // 0번 입력 속성: 위치
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 0, 
                                    format: wgpu::VertexFormat::Float32x3
                                }
                            ]
                        }
                    ], 
                    compilation_options: wgpu::PipelineCompilationOptions::default()
                }, 
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, 
                    polygon_mode: wgpu::PolygonMode::Line, 
                    ..Default::default()
                }, 
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_STENCIL_FORMAT, 
                    depth_write_enabled: true, 
                    depth_compare: wgpu::CompareFunction::Less, 
                    stencil: wgpu::StencilState::default(), 
                    bias: wgpu::DepthBiasState::default()
                }), 
                multisample: wgpu::MultisampleState::default(), 
                fragment: Some(
                    wgpu::FragmentState {
                        module: &module, 
                        entry_point: "fs_main", 
                        targets: &[
                            Some(wgpu::ColorTargetState {
                                format: SWAPCHAIN_FORMAT, 
                                blend: None, 
                                write_mask: wgpu::ColorWrites::ALL
                            })
                        ], 
                        compilation_options: wgpu::PipelineCompilationOptions::default()
                    }
                ), 
                multiview: None, 
                cache: None
            }
        )
    })
}



/// 충돌 경계 상자를 그리기는 브러쉬
#[derive(Debug)]
pub struct BoundingBoxBrush {
    /// 그리기 대상 메쉬입니다.
    mesh: Arc<Mesh>, 

    /// 메쉬의 쉐이더 리소스입니다.
    mesh_resource: Arc<StaticMeshResource>, 

    /// 재질의 쉐이더 리소스입니다.
    materials: Arc<UniversalMaterialResource>, 

    /// 그래픽스 파이프라인입니다.
    pipeline: &'static wgpu::RenderPipeline
}

impl BoundingBoxBrush {
    /// 경계 상자를 그리는 브러쉬를 생성합니다.
    pub fn new(
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bounds: BoundingBox, 
        color: (f32, f32, f32)
    ) -> Arc<Self> {
        Self {
            mesh: create_mesh(
                device, 
                queue, 
                bounds.extents.x, 
                bounds.extents.y, 
                bounds.extents.z
            ), 
            mesh_resource: Arc::new(StaticMeshResource::new(
                Some("Debug(BoundingBox)"), device
            )), 
            materials: create_material(
                device, 
                queue, 
                color.0, 
                color.1, 
                color.2
            ), 
            pipeline: get_render_pipeline(device)
        }.into()
    }

    /// 브러쉬의 메쉬 쉐이더 리소스를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn mesh_resource(&self) -> &Arc<StaticMeshResource> {
        &self.mesh_resource
    }
}

impl MeshBrush for BoundingBoxBrush {
    #[inline]
    #[must_use]
    fn mesh(&self) -> &Arc<Mesh> {
        &self.mesh
    }

    #[inline]
    #[must_use]
    fn pipeline(&self) -> &'static wgpu::RenderPipeline {
        self.pipeline
    }

    fn bind<'a>(&'a self, camera: &CameraResource, rpass: &mut wgpu::RenderPass<'a>) {
        // 렌더 패스에 그래픽스 파이프라인을 바인드합니다.
        rpass.set_pipeline(&self.pipeline);

        // 렌더 패스에 쉐이더 리소스를 바인드합니다.
        rpass.set_bind_group(0, camera.bind_group(), &[]);

        // 렌더 패스에 메쉬를 바인드합니다.
        rpass.set_vertex_buffer(0, self.mesh.vertex().slice(..));

        // 렌더 패스에 인덱스 버퍼를 바인드합니다.
        let index_buffer = unsafe { self.mesh.submeshes().first().unwrap_unchecked() };
        rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
    }

    fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        // 렌더 패스에 정적 메쉬 쉐이더 리소스를 바인드합니다.
        rpass.set_bind_group(1, self.mesh_resource.bind_group(), &[]);

        // 렌더 패스에 재질 쉐이더 리소스를 바인드합니다.
        rpass.set_bind_group(2, self.materials.bind_group(), &[]);

        // 인덱스 버퍼를 사용하여 경계 상자를 그립니다.
        let index = unsafe { self.mesh.submeshes().first().unwrap_unchecked() };
        rpass.draw_indexed(0..index.count(), 0, 0..1);
    }
}



/// 주어진 `w`(가로), `h`(세로), `d`(깊이)로 경계 상자 메쉬를 생성합니다.
#[must_use]
fn create_mesh(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    w: f32, 
    h: f32, 
    d: f32
) -> Arc<Mesh> {
    let hw = 0.5 * w;
    let hh = 0.5 * h;
    let hd = 0.5 * d;

    let mut vertices = Vec::with_capacity(8);
    vertices.push(gmm::Float3::new(-hw, hh, -hd));
    vertices.push(gmm::Float3::new(hw, hh, -hd));
    vertices.push(gmm::Float3::new(hw, hh, hd));
    vertices.push(gmm::Float3::new(-hw, hh, hd));
    vertices.push(gmm::Float3::new(-hw, -hh, -hd));
    vertices.push(gmm::Float3::new(hw, -hh, -hd));
    vertices.push(gmm::Float3::new(hw, -hh, hd));
    vertices.push(gmm::Float3::new(-hw, -hh, hd));

    let mut indices = Vec::with_capacity(36);
    indices.push(3); indices.push(1); indices.push(0);
    indices.push(2); indices.push(1); indices.push(3);
    indices.push(0); indices.push(5); indices.push(4);
    indices.push(1); indices.push(5); indices.push(0);
    indices.push(3); indices.push(4); indices.push(7);
    indices.push(0); indices.push(4); indices.push(3);
    indices.push(1); indices.push(6); indices.push(5);
    indices.push(2); indices.push(6); indices.push(1);
    indices.push(2); indices.push(7); indices.push(6);
    indices.push(3); indices.push(7); indices.push(2);
    indices.push(6); indices.push(4); indices.push(5);
    indices.push(7); indices.push(4); indices.push(6);

    let mut mesh = Mesh::new("Debug(BoundingBox)", device, queue, Vertices(vertices));
    mesh.insert_submesh(device, queue, Indices::U16(indices));

    Arc::new(mesh)
}



/// 주어진 `red`, `green`, `blue`로 재질을 생성합니다.
#[must_use]
fn create_material(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    red: f32, 
    green: f32, 
    blue: f32
) -> Arc<UniversalMaterialResource> {
    let mut desc = UniversalMaterialDescriptor::new(device, queue, "Debug(BoundingBox)");
    desc.albedo = [red, green, blue, 1.0];

    let material = UniversalMaterialResource::new(device, queue, &desc);
    Arc::new(material)
}
