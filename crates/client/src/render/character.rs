use std::sync::Arc;

use mod_render::{CameraResource, MaterialResource, MeshResource};

pub const CHARACTER_PIPELINE_ID: &'static str = "Character";
pub const CHARACTER_HALO_PIPELINE_ID: &'static str = "CharacterHalo";
pub const CHARACTER_SHADOW_PIPELINE_ID: &'static str = "Shadow(Character)";

/// 캐릭터 쉐이더 모듈을 생성합니다.
fn create_character_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/character.wgsl"
    ));

    if cfg!(feature = "enable-shader-validation") {
        device.create_shader_module(desc)
    } else {
        unsafe { device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked()) }
    }
}

/// 캐릭터 헤일로 쉐이더 모듈을 생성합니다.
fn create_character_halo_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/character_halo.wgsl"
    ));

    if cfg!(feature = "enable-shader-validation") {
        device.create_shader_module(desc)
    } else {
        unsafe { device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked()) }
    }
}

/// 캐릭터 그림자 쉐이더 모듈을 생성합니다.
fn create_character_shadow_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/character_shadow.wgsl"
    ));

    if cfg!(feature = "enable-shader-validation") {
        device.create_shader_module(desc)
    } else {
        unsafe { device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked()) }
    }
}

/// 파이프라인 레이아웃을 생성합니다.
fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("PipelineLayout(Character)"),
        bind_group_layouts: &[
            CameraResource::bind_group_layout(device),
            MeshResource::bind_group_layout(device),
            MaterialResource::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    })
}

/// 그림자 파이프라인 레이아웃을 생성합니다.
fn create_shadow_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("PipelineLayout(Character(Shadow))"),
        bind_group_layouts: &[
            CameraResource::bind_group_layout(device),
            MeshResource::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    })
}

/// 캐릭터 모델 렌더링 파이프라인을 생성합니다.
pub fn create_character_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_character_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(Character)"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[
                // 0번 입력 속성: 위치
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 1번 입력 속성: 노멀
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 2번 입력 속성: 탄젠트 공간 노멀
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 3번 입력 속성: 0번 텍스처 좌표
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 4번 입력 속성: 뼈 번호
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[u32; 4]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 4,
                        format: wgpu::VertexFormat::Uint32x4,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 5번 입력 속성: 뼈 가중치
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 5,
                        format: wgpu::VertexFormat::Float32x4,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Cw,
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            depth_compare: wgpu::CompareFunction::Less,
            depth_write_enabled: true,
            format: depth_stencil_format,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                blend: None,
                format: render_target_format,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    Arc::new(pipeline)
}

/// 캐릭터 헤일로 렌더링 파이프라인을 생성합니다.
pub fn create_character_halo_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_character_halo_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(CharacterHalo)"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[
                // 0번 입력 속성: 위치
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 1번 입력 속성: 0번 텍스처 좌표
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            front_face: wgpu::FrontFace::Cw,
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            depth_compare: wgpu::CompareFunction::Less,
            depth_write_enabled: true,
            format: depth_stencil_format,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                blend: None,
                format: render_target_format,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    Arc::new(pipeline)
}

/// 그림자를 생성하는 그래픽스 파이프라인을 생성합니다.
pub fn create_character_shadow_render_pipeline(device: &wgpu::Device) -> Arc<wgpu::RenderPipeline> {
    let module = create_character_shadow_shader_module(device);
    let layout = create_shadow_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(Shadow)"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[
                // 0번 입력 속성: 위치
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 1번 입력 속성: 뼈 번호
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[u32; 4]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Uint32x4,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 2번 입력 속성: 뼈 가중치
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x4,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Front),
            front_face: wgpu::FrontFace::Cw,
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: device
                .features()
                .contains(wgpu::Features::DEPTH_CLIP_CONTROL),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            depth_compare: wgpu::CompareFunction::LessEqual,
            depth_write_enabled: true,
            format: wgpu::TextureFormat::Depth32Float,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview: None,
        cache: None,
    });

    Arc::new(pipeline)
}
