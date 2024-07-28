//! 3차원 큐브를 렌더링하는 예제 애플리케이션입니다.
//! 
use core::fmt;
use core::mem;
use core::ops::Deref;
use std::thread;
use std::sync::Arc;
use std::io::Cursor;
use std::include_bytes;
use hecs::World;
use wgpu::util::DeviceExt;
use winit::window::Window;
use gmm::{Float3, Float4x4, Quaternion, Matrix};
use client_framework::app::builder::AppBuilder;
use client_framework::app::dpi::Dpi;
use client_framework::app::Application;
use client_framework::components::Transform;
use client_framework::components::Projection;
use client_framework::components::PerspectiveBuilder;
use client_framework::error::AppError;
use client_framework::render;
use client_framework::scene::GameScene;

use image::io::Reader as ImageReader;



/// 64bit `Windows`, `macOS` 플랫폼의
/// 애플리케이션 진입점 입니다.
/// 
/// 게임 화면은 16 : 9 비율의 scaled 크기를 가집니다.
/// 
/// `Windows`, `macOS` 플랫폼의 경우 최초 실행시 전체 화면으로 실행됩니다.
/// 
#[cfg(target_pointer_width = "64")]
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn main() {
    use framework::concurrency::MAIN_THREAD_ID;
    assert_eq!(thread::current().id(), *MAIN_THREAD_ID, "Invalid main thread id!");

    // 로그 시스템을 초기화 합니다.
    env_logger::init();
    log::info!("클라이언트 애플리케이션 실행...");

    AppBuilder::new(Box::new(ExampleScene::default()))
        .set_title("Example: Cube")
        .set_fullscreen(false)
        .set_dpi(Dpi::W1280H720)
        .build_and_run()
}



/// 쉐이더에 전달되는 엔티티 유니폼 블록의 구조체 입니다.
#[repr(C, align(16))]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct EntityBlob {
    pub trans: Float4x4, 
}

/// 쉐이더에 전달되는 카메라 유니폼 블록의 구조체 입니다.
#[repr(C, align(16))]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct CameraBlob {
    pub view_proj: Float4x4, 
    pub position: Float3, 
    pub _padding0: [u8; mem::size_of::<f32>() * 1], 
}



/// 예제 게임 장면 입니다.
pub struct ExampleScene {
    depth_stencil_view: Option<wgpu::TextureView>, 
}

impl ExampleScene {
    /// 깊이-스텐실 텍스처를 생성합니다.
    fn create_depth_stencil_view(width: u32, height: u32, device: &wgpu::Device) -> wgpu::TextureView {
        // 깊이-스텐실 텍스처 뷰를 생성합니다.
        device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Texture2D(Depth-Stencil)"), 
                size: wgpu::Extent3d {width, height, depth_or_array_layers: 1 }, 
                format: wgpu::TextureFormat::Depth32Float, 
                dimension: wgpu::TextureDimension::D2, 
                mip_level_count: 1, 
                sample_count: 1, 
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
                view_formats: &[]
            }
        ).create_view(&wgpu::TextureViewDescriptor { ..Default::default() })
    }

    /// 큐브 메쉬 버텍스 버퍼를 생성합니다.
    fn create_cube_mesh(device: &wgpu::Device) -> Arc<wgpu::Buffer> {
        device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Buffer(Cube)"), 
                contents: bytemuck::cast_slice(&VERTICES), 
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        ).into()
    }

    /// 카메라 쉐이더 리소스를 생성합니다.
    fn create_camera_resources(device: &wgpu::Device) -> (
        CameraBlob,
        Arc<wgpu::Buffer>, 
        Arc<wgpu::BindGroupLayout>, 
        Arc<wgpu::BindGroup>
    ) {
        // 카메라 블롭을 생성합니다.
        let blob = CameraBlob::default();

        // 유니폼 버퍼를 생성합니다.
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Buffer(Camera)"), 
                contents: bytemuck::bytes_of(&blob), 
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
            }
        );

        // 바인드 그룹 레이아웃을 생성합니다.
        let layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(Buffer(CameraBlob))"), 
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, 
                        visibility: wgpu::ShaderStages::VERTEX, 
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        },
                        count: None, 
                    }
                ]
            }
        );

        // 바인드 그룹을 생성합니다.
        let group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("BindGroup(Buffer(EntityBlob))"), 
                layout: &layout, 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: wgpu::BindingResource::Buffer(
                            buffer.as_entire_buffer_binding()
                        )
                    },
                ]
            }
        );

        return (blob, buffer.into(), layout.into(), group.into());
    }

    /// 엔티티 쉐이더 리소스를 생성합니다.
    fn create_entity_resources(device: &wgpu::Device) -> (
        EntityBlob,
        Arc<wgpu::Buffer>, 
        Arc<wgpu::BindGroupLayout>, 
        Arc<wgpu::BindGroup>
    ) {
        // 엔티티 블롭을 생성합니다.
        let blob = EntityBlob::default();

        // 유니폼 버퍼를 생성합니다.
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Buffer(Entity)"), 
                contents: bytemuck::bytes_of(&blob), 
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
            }
        );

        // 바인드 그룹 레이아웃을 생성합니다.
        let layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(Buffer(EntityBlob))"), 
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, 
                        visibility: wgpu::ShaderStages::VERTEX, 
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        },
                        count: None, 
                    }
                ]
            }
        );

        // 바인드 그룹을 생성합니다.
        let group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("BindGroup(Buffer(EntityBlob))"), 
                layout: &layout, 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: wgpu::BindingResource::Buffer(
                            buffer.as_entire_buffer_binding()
                        )
                    },
                ]
            }
        );

        return (blob, buffer.into(), layout.into(), group.into());
    }

    /// 텍스처를 쉐이더 리소스를 생성합니다.
    fn create_texture_resource(device: &wgpu::Device, queue: &wgpu::Queue) -> (
        Arc<wgpu::TextureView>,
        Arc<wgpu::BindGroupLayout>, 
        Arc<wgpu::BindGroup>
    ) {
        // 텍스처 이미지를 로드합니다.
        let img = ImageReader::new(Cursor::new(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/test.png")))
        )
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

        // 텍스처 이미지 뷰를 생성합니다.
        let view = device.create_texture_with_data(
            queue, 
            &wgpu::TextureDescriptor {
                label: Some("Texture2D(Test)"), 
                size: wgpu::Extent3d {
                    width: 1024, 
                    height: 1024, 
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Rgba8Unorm,
                dimension: wgpu::TextureDimension::D2, 
                mip_level_count: 1, 
                sample_count: 1, 
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }, 
            wgpu::util::TextureDataOrder::default(), 
            &img.to_rgba8()
        ).create_view(&wgpu::TextureViewDescriptor { ..Default::default() });

        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                label: Some("Sampler"), 
                address_mode_u: wgpu::AddressMode::ClampToEdge, 
                address_mode_v: wgpu::AddressMode::ClampToEdge, 
                address_mode_w: wgpu::AddressMode::ClampToEdge, 
                mag_filter: wgpu::FilterMode::Linear, 
                min_filter: wgpu::FilterMode::Linear, 
                mipmap_filter: wgpu::FilterMode::Linear, 
                ..Default::default()
            }
        );

        // 바인드 그룹 레이아웃을 생성합니다.
        let layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(Texture2D(Test))"), 
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, 
                        visibility: wgpu::ShaderStages::FRAGMENT, 
                        ty: wgpu::BindingType::Texture { 
                            sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                            view_dimension: wgpu::TextureViewDimension::D2, 
                            multisampled: false 
                        }, 
                        count: None,
                    }, 
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, 
                        visibility: wgpu::ShaderStages::FRAGMENT, 
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering
                        ), 
                        count: None,
                    },
                ]
            }
        );

        // 바인드 그룹을 생성합니다.
        let group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("BindGroup(Texture2D(Test))"), 
                layout: &layout, 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: wgpu::BindingResource::TextureView(
                            &view
                        )
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: wgpu::BindingResource::Sampler(
                            &sampler
                        )
                    }
                ]
            }
        );

        return (view.into(), layout.into(), group.into());
    }

    /// 그래픽스 파이프라인을 생성합니다.
    fn create_render_pipeline(
        device: &wgpu::Device, 
        bind_group_layouts: &[&wgpu::BindGroupLayout], 
    ) -> Arc<wgpu::RenderPipeline> {
        // 쉐이더를 생성합니다.
        // ※ 이 함수는 런타임에 쉐이더 코드를 검사합니다.
        let shader = device.create_shader_module(
            wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/shader.wgsl"))
        );

        // 파이프라인 레이아웃을 생성합니다.
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("PipelineLayout(Shader(Color))"), 
                bind_group_layouts,
                push_constant_ranges: &[]
            }
        );

        // 그래픽스 파이프라인을 생성합니다.
        device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(Shader(Color))"), 
                layout: Some(&pipeline_layout), 
                cache: None, 
                vertex: wgpu::VertexState {
                    module: &shader, 
                    entry_point: "vs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: (mem::size_of::<f32>() * 5) as u64, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    shader_location: 0, 
                                    format: wgpu::VertexFormat::Float32x3, 
                                    offset: (mem::size_of::<f32>() * 0) as u64,
                                },
                                wgpu::VertexAttribute {
                                    shader_location: 1, 
                                    format: wgpu::VertexFormat::Float32x2, 
                                    offset: (mem::size_of::<f32>() * 3) as u64,
                                }, 
                            ], 
                        }, 
                    ], 
                }, 
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, 
                    strip_index_format: None, 
                    // front_face: wgpu::FrontFace::Ccw, 
                    // cull_mode: Some(wgpu::Face::Back), 
                    polygon_mode: wgpu::PolygonMode::Fill, 
                    ..Default::default()
                }, 
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float, 
                    depth_write_enabled: true, 
                    depth_compare: wgpu::CompareFunction::Less, 
                    stencil: wgpu::StencilState::default(), 
                    bias: wgpu::DepthBiasState::default(), 
                }), 
                multisample: wgpu::MultisampleState::default(), 
                fragment: Some(wgpu::FragmentState {
                    module: &shader, 
                    entry_point: "fs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            blend: None, 
                            format: render::get_swapchain_format(), 
                            write_mask: wgpu::ColorWrites::all(),
                        }),
                    ],
                }),
                multiview: None,
            }
        ).into()
    }
}

impl Default for ExampleScene {
    #[inline]
    fn default() -> Self {
        Self { depth_stencil_view: None }
    }
}

impl GameScene for ExampleScene {
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        let device = app.ref_render_device();
        let queue = app.ref_render_queue();

        let (width, height): (u32, u32) = window.inner_size().into();
        self.depth_stencil_view = Some(Self::create_depth_stencil_view(width, height, device));

        let cube_mesh = Self::create_cube_mesh(device);

        let (
            entity_blob, 
            entity_buffer, 
            entity_layout, 
            entity_group
        ) = Self::create_entity_resources(device);

        let (
            camera_blob, 
            camera_buffer, 
            camera_layout, 
            camera_group
        ) = Self::create_camera_resources(device);
        
        let (
            _texture_view, 
            texture_layout, 
            texture_group
        ) = Self::create_texture_resource(device, queue);

        let bind_group_layouts = &[entity_layout.deref(), &camera_layout.deref(), &texture_layout.deref()];
        let render_pipeline = Self::create_render_pipeline(
            device, 
            bind_group_layouts
        );



        // 엔티티를 생성합니다.
        world.spawn((
            Transform::new(), 
            cube_mesh, 
            (entity_blob, entity_buffer),
            (entity_group.clone(), camera_group.clone(), texture_group.clone()), 
            render_pipeline, 
        ));

        // 카메라를 생성합니다.
        world.spawn((
            Transform::from_rotation_translation(
                Quaternion::from_rotation_x(30_f32.to_radians()), 
                (0.0, 1.0, -2.0)
            ), 
            PerspectiveBuilder::new()
                .with_fov_y(60f32.to_radians())
                .with_aspect(width as f32 / height as f32)
                .with_z_near(0.001)
                .with_z_far(100.0)
                .build(), 
            (camera_blob, camera_buffer), 
        ));

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_exit(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        // 모든 엔티티를 삭제합니다.
        world.clear();
        
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Application 
    ) -> Result<(), AppError> {
        let queue = app.ref_render_queue();

        // 카메라 갱신
        type QueryCamera<'a> = (&'a Transform, &'a Projection, &'a mut (CameraBlob, Arc<wgpu::Buffer>));
        for (_id, (transform, projection, (blob, buffer))) in world.query_mut::<QueryCamera>() {
            let view = Matrix::look_to_rh(
                transform.get_translation().into(), 
                transform.get_forward_vector().into(), 
                transform.get_up_vector().into()
            );

            let projection: Matrix = (**projection).into();
            blob.view_proj = (projection * view).into();
            blob.position = transform.get_translation();
            queue.write_buffer(&buffer, 0, bytemuck::bytes_of(blob));
        }


        // 엔티티 갱신
        type QueryEntity<'a> = (&'a mut Transform, &'a mut (EntityBlob, Arc<wgpu::Buffer>));
        for (_id, (transform, (blob, buffer))) in world.query_mut::<QueryEntity>() {
            transform.rotate_y_axis(15_f32.to_radians() * elapsed_time_sec);
            blob.trans = **transform;
            queue.write_buffer(&buffer, 0, bytemuck::bytes_of(blob));
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        let device = app.ref_render_device();
        let queue = app.ref_render_queue();

        // 이전 작업이 끝날 때 까지 기다립니다.
        device.poll(wgpu::Maintain::Wait);

        // 현재 스왑체인 이미지를 가져옵니다.
        let frame = surface.get_current_texture()
            .map_err(|e| AppError::from(e))?;
        
        // 렌더 타겟 뷰를 가져옵니다.
        let render_target_view = frame.texture.create_view(&wgpu::TextureViewDescriptor { ..Default::default() });

        // 엔티티의 반복자를 빌려옵니다.
        type QueryType<'a> = (
            &'a Arc<wgpu::Buffer>, 
            &'a (Arc<wgpu::BindGroup>, Arc<wgpu::BindGroup>, Arc<wgpu::BindGroup>), 
            &'a Arc<wgpu::RenderPipeline>
        );
        let mut query = world.query::<QueryType>();

        // 커맨드 버퍼를 생성합니다.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { ..Default::default() });
        {
            let mut rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("RenderPass(RenderPipeline(Shader(Color)))"), 
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &render_target_view, 
                            resolve_target: None, 
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), 
                                store: wgpu::StoreOp::Store, 
                            }, 
                        }),
                    ],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: self.depth_stencil_view.as_ref().unwrap(), 
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0), 
                            store: wgpu::StoreOp::Store, 
                        }), 
                        stencil_ops: None,
                    }), 
                    timestamp_writes: None, 
                    occlusion_query_set: None, 
                },
            );

            // 렌더 대상 엔티티들을 수집합니다.
            for (_id, (mesh, group, pipeline)) in query.iter() {
                rpass.set_pipeline(&pipeline);
                rpass.set_vertex_buffer(0, mesh.slice(..));
                rpass.set_bind_group(0, &group.0, &[]);
                rpass.set_bind_group(1, &group.1, &[]);
                rpass.set_bind_group(2, &group.2, &[]);
                rpass.draw(0..36, 0..1);
            };
        }

        queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}

impl fmt::Debug for ExampleScene {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(ExampleScene))
    }
}



/// Triangle Strip 큐브 메쉬
/// 
/// 속성
/// - Position
/// - Texcoord 
/// 
const VERTICES: [f32; 180] = [
    -0.5,  0.5, -0.5, 0.0, 0.0, 
     0.5,  0.5, -0.5, 1.0, 0.0, 
     0.5, -0.5, -0.5, 1.0, 1.0, 

    -0.5,  0.5, -0.5, 0.0, 0.0, 
     0.5, -0.5, -0.5, 1.0, 1.0, 
    -0.5, -0.5, -0.5, 0.0, 1.0, 

    -0.5,  0.5,  0.5, 0.0, 0.0, 
     0.5,  0.5,  0.5, 1.0, 0.0, 
     0.5,  0.5, -0.5, 1.0, 1.0, 

    -0.5,  0.5,  0.5, 0.0, 0.0, 
     0.5,  0.5, -0.5, 1.0, 1.0, 
    -0.5,  0.5, -0.5, 0.0, 1.0, 

    -0.5, -0.5,  0.5, 0.0, 1.0, 
     0.5, -0.5,  0.5, 1.0, 1.0, 
     0.5,  0.5,  0.5, 1.0, 0.0, 

    -0.5, -0.5,  0.5, 0.0, 1.0, 
     0.5,  0.5,  0.5, 1.0, 0.0, 
    -0.5,  0.5,  0.5, 0.0, 0.0, 

    -0.5, -0.5, -0.5, 0.0, 1.0, 
     0.5, -0.5, -0.5, 1.0, 1.0, 
     0.5, -0.5,  0.5, 1.0, 0.0, 

    -0.5, -0.5, -0.5, 0.0, 1.0, 
     0.5, -0.5,  0.5, 1.0, 0.0, 
    -0.5, -0.5,  0.5, 0.0, 0.0, 

    -0.5,  0.5,  0.5, 1.0, 0.0, 
    -0.5,  0.5, -0.5, 1.0, 1.0, 
    -0.5, -0.5, -0.5, 0.0, 1.0, 

    -0.5,  0.5,  0.5, 1.0, 0.0, 
    -0.5, -0.5, -0.5, 0.0, 1.0, 
    -0.5, -0.5,  0.5, 0.0, 0.0, 

     0.5,  0.5, -0.5, 1.0, 1.0, 
     0.5,  0.5,  0.5, 1.0, 0.0, 
     0.5, -0.5,  0.5, 0.0, 0.0, 

     0.5,  0.5, -0.5, 1.0, 1.0, 
     0.5, -0.5,  0.5, 0.0, 0.0, 
     0.5, -0.5, -0.5, 0.0, 1.0, 
];
