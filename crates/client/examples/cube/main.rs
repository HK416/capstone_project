//! 3차원 큐브를 렌더링하는 예제 애플리케이션입니다.
//! 
use std::fmt;
use std::mem;
use std::thread;
use std::sync::Arc;
use std::io::Cursor;
use std::borrow::Cow;
use std::include_bytes;
use client_framework::app::builder::AppBuilder;
use client_framework::app::dpi::Dpi;
use client_framework::app::Application;
use client_framework::error::AppError;
use client_framework::render;
use client_framework::hecs::World;
use client_framework::scene::GameScene;
use client_framework::wgpu;
use client_framework::wgpu::util::DeviceExt;

use framework::MAIN_THREAD_ID;
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



/// 쉐이더에 전달하는 글로벌 유니폼 블록의 구조체입니다.
#[repr(C, align(16))]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct GlobalUniformBlob {
    pub size: [f32; 2], 
    pub fov_y: f32, 
    pub _padding: [u8; mem::size_of::<f32>() * 1],
    pub position: [f32; 3], 
    pub radian: f32,
}



/// 예제 게임 장면 입니다.
pub struct ExampleScene {
    depth_stencil_view: Option<wgpu::TextureView>, 
}

impl ExampleScene {
    /// 텍스처 이미지 바인드 그룹을 생성합니다.
    #[must_use]
    fn create_texture_bind_group(
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        // 텍스처 이미지를 로드합니다.
        let img = ImageReader::new(Cursor::new(TEXTURE))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();

        // 텍스처 이미지 버퍼를 생성하고, 이미지 뷰를 생성합니다.
        let texture_view = device.create_texture_with_data(
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

        // 텍스처 샘플러를 생성합니다.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler2D(Test)"), 
            address_mode_u: wgpu::AddressMode::ClampToEdge, 
            address_mode_v: wgpu::AddressMode::ClampToEdge, 
            address_mode_w: wgpu::AddressMode::ClampToEdge, 
            mag_filter: wgpu::FilterMode::Linear, 
            min_filter: wgpu::FilterMode::Linear, 
            ..Default::default()
        });

        // 텍스처 바인드 그룹 레이아웃을 생성합니다.
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BindGroupLayout(Texture(Test))"), 
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), 
                    count: None,
                },
            ],
        });

        // 텍스처 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Texture(Test))"), 
            layout: &bind_group_layout, 
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, 
                    resource: wgpu::BindingResource::TextureView(&texture_view), 
                }, 
                wgpu::BindGroupEntry {
                    binding: 1, 
                    resource: wgpu::BindingResource::Sampler(&sampler), 
                },
            ],
        });

        (bind_group_layout, bind_group)
    }

    /// 그래픽스 파이프라인을 생성합니다.
    #[must_use]
    fn create_render_pipeline(
        device: &wgpu::Device, 
        bind_group_layouts: &[&wgpu::BindGroupLayout], 
    ) -> wgpu::RenderPipeline {
        // 쉐이더를 생성합니다.
        // ※ 이 함수는 런타임에 쉐이더 코드를 검사합니다.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader(Color)"), 
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&SHADER))
        });

        // 파이프라인 레이아웃을 생성합니다.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(Shader(Color))"), 
            bind_group_layouts,
            push_constant_ranges: &[]
        });

        // 그래픽스 파이프라인을 생성합니다.
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RenderPipeline(Shader(Color))"), 
            layout: Some(&pipeline_layout), 
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
        })
    }
}

impl Default for ExampleScene {
    #[inline]
    fn default() -> Self {
        Self { depth_stencil_view: None }
    }
}

impl GameScene for ExampleScene {
    fn on_enter(&mut self, world: &mut World, app: &dyn Application) -> Result<(), AppError> {
        let device = app.ref_render_device();
        let queue = app.ref_render_queue();

        // 깊이-스텐실 텍스처 뷰를 생성합니다.
        self.depth_stencil_view = Some(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture(Depth-Stencil)"), 
            size: wgpu::Extent3d {
                width: 1280, 
                height: 720, 
                depth_or_array_layers: 1, 
            }, 
            format: wgpu::TextureFormat::Depth32Float, 
            dimension: wgpu::TextureDimension::D2, 
            mip_level_count: 1, 
            sample_count: 1, 
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
            view_formats: &[]
        }).create_view(&wgpu::TextureViewDescriptor { ..Default::default() }));

        // 큐브 메쉬 버퍼를 생성합니다.
        let mesh = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer(Cube)"), 
            contents: bytemuck::cast_slice(&VERTICES), 
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // 글로벌 유니폼 버퍼를 생성합니다.
        let global_blob = GlobalUniformBlob {
            size: [1280.0, 720.0], 
            fov_y: 30.0f32.to_radians(), 
            position: [0.0, 0.0, -5.0], 
            ..Default::default()
        };
        let global_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer(Global)"), 
            contents: bytemuck::bytes_of(&global_blob), 
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 글로벌 유니폼 데이터 바인드 그룹 레이아웃을 생성합니다.
        let global_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor { 
                label: Some("BindGroupLayout(Buffer(Global))"), 
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, 
                        visibility: wgpu::ShaderStages::VERTEX, 
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None, 
                        }, 
                        count: None, 
                    },
                ], 
            }
        );

        // 글로벌 유니폼 데이터 바인드 그룹을 생성합니다.
        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { 
            label: Some("BindGroup(Buffer(Global))"), 
            layout: &global_bind_group_layout, 
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, 
                    resource: wgpu::BindingResource::Buffer(global_buffer.as_entire_buffer_binding()),
                }
            ] 
        });

        // 텍스처 이미지 바인드 그룹을 생성합니다.
        let (texture_bind_group_layout, texture_bind_group) = Self::create_texture_bind_group(device, queue);

        // 그래픽스 파이프라인을 생성합니다.
        let pipeline = Self::create_render_pipeline(device, &[&global_bind_group_layout, &texture_bind_group_layout]);
        
        // 큐브 오브젝트 엔티티를 생성합니다.
        world.spawn((
            global_blob, 
            Arc::new(global_buffer), 
            (
                Arc::new(mesh), 
                Arc::new(global_bind_group), 
                Arc::new(texture_bind_group), 
                Arc::new(pipeline)
            )
        ));

        Ok(())
    }

    fn on_exit(&mut self, world: &mut World, _app: &dyn Application) -> Result<(), AppError> {
        // 모든 엔티티를 삭제합니다.
        world.clear();
        
        Ok(())
    }

    fn on_draw(
        &self, 
        world: &World, 
        app: &dyn Application, 
        surface: &wgpu::Surface
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

        type QueryType<'a> = &'a (Arc<wgpu::Buffer>, Arc<wgpu::BindGroup>, Arc<wgpu::BindGroup>, Arc<wgpu::RenderPipeline>);
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
            for (_id, (mesh, group0, group1, pipeline)) in query.iter() {
                rpass.set_pipeline(&pipeline);
                rpass.set_vertex_buffer(0, mesh.slice(..));
                rpass.set_bind_group(0, &group0, &[]);
                rpass.set_bind_group(1, &group1, &[]);
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

/// 텍스처를 출력하는 쉐이더 코드
const SHADER: &'static str = r"
    // 버텍스 입력 구조체
    struct VertexInput {
        @location(0) position: vec3<f32>, 
        @location(1) texcoord: vec2<f32>, 
    };

    // 버텍스 출력 구조체
    struct VertexOutput {
        @location(0) texcoord: vec2<f32>,
        @builtin(position) position: vec4<f32>,
    };

    // 글로벌 정보 구조체
    struct Global {
        size: vec2<f32>, 
        fov_y: f32,
        position: vec3<f32>, 
        radian: f32,
    };

    // 글로벌 정보 유니폼 데이터
    @group(0)
    @binding(0)
    var<uniform> u_global: Global;

    // 텍스처
    @group(1)
    @binding(0)
    var t_diffuse: texture_2d<f32>;
    
    // 텍스처 샘플러
    @group(1)
    @binding(1)
    var s_diffuse: sampler;



    @vertex
    fn vs_main(input: VertexInput) -> VertexOutput {
        let s = sin(u_global.fov_y * 0.5);
        let c = cos(u_global.fov_y * 0.5);
        let h = c / s;
        let w = h / (u_global.size.x / u_global.size.y);
        let r = 1000.0 / (0.001 - 1000.0);
        let proj = mat4x4<f32>(
            w, 0.0, 0.0, 0.0, 
            0.0, h, 0.0, 0.0, 
            0.0, 0.0, r, -1.0, 
            0.0, 0.0, r * 0.001, 0.0
        );

        let view = mat4x4<f32>(
            1.0, 0.0, -0.0, 0.0, 
            0.0, 1.0, -0.0, 0.0, 
            0.0, 0.0, -1.0, 0.0, 
            -u_global.position.x, -u_global.position.y, u_global.position.z, 1.0
        );

        var output: VertexOutput;
        output.position = proj * view * vec4<f32>(input.position, 1.0);
        output.texcoord = input.texcoord;
        return output;
    }

    @fragment
    fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
        let diffuse_color = textureSample(t_diffuse, s_diffuse, input.texcoord);
        return diffuse_color;
    }
";

/// 테스트 텍스처 이미지 </br>
/// 경로: $CARGO_MANIFEST_DIR/examples/assets/test.png </br>
/// 크기: 1024x1024 </br>
/// 
const TEXTURE: &'static [u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/test.png"));
