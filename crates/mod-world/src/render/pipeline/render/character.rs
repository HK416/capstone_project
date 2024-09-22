use std::{mem, sync::{Arc, OnceLock}};

use crate::render::{
    material::Material, 
    mesh::SkinnedMesh, 
    DEPTH_STENCIL_FORMAT, 
    SWAPCHAIN_FORMAT
};



/// 캐릭터를 화면에 그릴 때 사용하는 그래픽스 파이프라인입니다.
pub struct CharacterShader {
    pipeline: wgpu::RenderPipeline, 
}

impl CharacterShader {
    /// 쉐이더 모듈을 생성합니다.
    #[must_use]
    fn shader_module(device: &Arc<wgpu::Device>) -> wgpu::ShaderModule {
        #[cfg(feature = "enable-shader-validation")] {
            device.create_shader_module(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/character.wgsl"))
            )
        }
        #[cfg(not(feature = "enable-shader-validation"))] {
            unsafe {
                device.create_shader_module_unchecked(
                    wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/character.wgsl"))
                )
            }
        }
    }

    /// 파이프라인 레이아웃을 생성합니다.
    #[must_use]
    fn pipeline_layout(device: &Arc<wgpu::Device>) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("PipelineLayout(CharacterShader)"), 
                bind_group_layouts: &[
                    // 0번 그룹: 카메라 데이터 & 조명 데이터
                    todo!(), 
                    // 1번 그룹: 스키닝된 메쉬 데이터
                    SkinnedMesh::bind_group_layout(device), 
                    // 2번 그룹: 재질 데이터
                    Material::bind_group_layout(device), 
                ], 
                push_constant_ranges: &[]
            }
        )
    }

    /// 그래픽스 파이프라인을 가져옵니다.
    #[must_use]
    pub fn get(device: &Arc<wgpu::Device>) -> &'static Self {
        static INSTANCE: OnceLock<CharacterShader> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let shader_module = Self::shader_module(device);
            let pipeline_layout = Self::pipeline_layout(device);
            Self {
                pipeline: device.create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: Some("RenderPipeline(CharacterShader)"), 
                        layout: Some(&pipeline_layout), 
                        vertex: wgpu::VertexState {
                            module: &shader_module, 
                            entry_point: "vs_main", 
                            compilation_options: wgpu::PipelineCompilationOptions::default(), 
                            buffers: &[
                                // 0번 입력 속성: 위치 데이터
                                wgpu::VertexBufferLayout {
                                    array_stride: mem::size_of::<gmm::Float3>() as wgpu::BufferAddress, 
                                    step_mode: wgpu::VertexStepMode::Vertex, 
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0, 
                                            shader_location: 0, 
                                            format: wgpu::VertexFormat::Float32x3, 
                                        }
                                    ]
                                }, 
                                // 1번 입력 속성: 노멀 데이터
                                wgpu::VertexBufferLayout {
                                    array_stride: mem::size_of::<gmm::Float3>() as wgpu::BufferAddress, 
                                    step_mode: wgpu::VertexStepMode::Vertex, 
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0, 
                                            shader_location: 1, 
                                            format: wgpu::VertexFormat::Float32x3, 
                                        }
                                    ]
                                },
                                // 2번 입력 속성: 탄젠트 공간 노멀 데이터
                                wgpu::VertexBufferLayout {
                                    array_stride: mem::size_of::<gmm::Float3>() as wgpu::BufferAddress, 
                                    step_mode: wgpu::VertexStepMode::Vertex, 
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0, 
                                            shader_location: 2, 
                                            format: wgpu::VertexFormat::Float32x3, 
                                        }
                                    ]
                                }, 
                                // 3번 입력 속성: 0번 텍스처 좌표
                                wgpu::VertexBufferLayout {
                                    array_stride: mem::size_of::<gmm::Float2>() as wgpu::BufferAddress, 
                                    step_mode: wgpu::VertexStepMode::Vertex, 
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0, 
                                            shader_location: 3, 
                                            format: wgpu::VertexFormat::Float32x2, 
                                        }
                                    ]
                                }, 
                                // 4번 입력 속성: 뼈 인덱스 데이터
                                wgpu::VertexBufferLayout {
                                    array_stride: mem::size_of::<gmm::UInteger4>() as wgpu::BufferAddress, 
                                    step_mode: wgpu::VertexStepMode::Vertex, 
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0, 
                                            shader_location: 4, 
                                            format: wgpu::VertexFormat::Uint32x4, 
                                        }
                                    ]
                                }, 
                                // 5번 입력 속성: 뼈 가중치 데이터
                                wgpu::VertexBufferLayout {
                                    array_stride: mem::size_of::<gmm::Float4>() as wgpu::BufferAddress, 
                                    step_mode: wgpu::VertexStepMode::Vertex, 
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0, 
                                            shader_location: 5, 
                                            format: wgpu::VertexFormat::Float32x4
                                        }
                                    ]
                                }, 
                            ]
                        }, 
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList, 
                            front_face: wgpu::FrontFace::Cw, 
                            cull_mode: Some(wgpu::Face::Back), 
                            polygon_mode: wgpu::PolygonMode::Fill, 
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
                        fragment: Some(wgpu::FragmentState {
                            module: &shader_module, 
                            entry_point: "fs_main", 
                            compilation_options: wgpu::PipelineCompilationOptions::default(), 
                            targets: &[
                                Some(wgpu::ColorTargetState {
                                    blend: None, 
                                    format: SWAPCHAIN_FORMAT, 
                                    write_mask: wgpu::ColorWrites::all()
                                }), 
                            ], 
                        }), 
                        multiview: None, 
                        cache: None
                    }
                )
            }
        })
    }

    /// 그래픽스 파이프라인을 렌더 패스에 바인드합니다.
    #[inline]
    pub fn bind<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&self.pipeline);
    }
}
