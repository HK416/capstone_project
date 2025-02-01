use std::{
    io::Cursor,
    num::NonZeroU64,
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};
use ddsfile::Dds;
use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_render::{CameraResource, GraphicsPipelinePool, TexturePool};
use wgpu::util::DeviceExt;

use crate::{
    asset::ModelAssetError,
    component::{Parent, ToParentTrans, WorldTransform},
};

use super::{LifeTime, ParticleKind};

/// 데미지 파티클의 렌더링 파이프라인 이름입니다.
pub const FX_DAMAGE_PIPELINE_NAME: &'static str = "Fx(Damage)";

#[derive(Debug, Clone, Copy)]
pub struct Damage {
    pub width: f32,
    pub height: f32,
    pub number: u32,
    pub position_v: [f32; 3],
}

/// ## Damage Particle Data Layout
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FxDamageDataLayout {
    /// 월드 변환 행렬
    pub trans: [f32; 16],
    /// 카메라 좌표계에서 사각형의 상대 위치
    pub position_v: [f32; 3],
    pub number: u32,
    /// 사각형의 가로 길이
    pub width: f32,
    /// 사각형의 세로 길이
    pub height: f32,
    pub _padding1: [u8; 8],
}

impl Default for FxDamageDataLayout {
    fn default() -> Self {
        Self {
            trans: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            position_v: [0.0, 0.0, 0.0],
            number: 0,
            width: 1.0,
            height: 1.0,
            _padding1: [0; 8],
        }
    }
}

/// ## Damage Particle Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxDamageUniform(Arc<wgpu::Buffer>);

impl FxDamageUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<FxDamageDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

#[allow(dead_code)]
impl FxDamageUniform {
    /// 초기화 되지 않은 새로운 데미지 파티클 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 데미지 파티클 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(&self, _device: &wgpu::Device, _queue: &wgpu::Queue, data: FxDamageDataLayout) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut FxDamageDataLayout = bytemuck::from_bytes_mut(&mut view);

                    *layout = data;

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });

        // let index = queue.submit([]);
        // device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 카메라 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub unsafe fn update_from_bytes(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        data: Vec<u8>,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    view.copy_from_slice(&data);

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });

        // let index = queue.submit([]);
        // device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 범위에 해당하는 슬라이스된 유니폼 버퍼를 반환합니다.
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.0.slice(bounds)
    }

    /// 유니폼 버퍼의 [`wgpu::BindingResource`]를 반환합니다.
    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        self.0.as_entire_binding()
    }

    /// 유니폼 버퍼의 [`wgpu::BufferBinding`]을 반환합니다.
    pub fn as_entire_buffer_binding(&self) -> wgpu::BufferBinding<'_> {
        self.0.as_entire_buffer_binding()
    }
}

/// ## Damage Particle Shader Resource
#[derive(Debug)]
pub struct FxDamageResource {
    pub uniform_buffer: FxDamageUniform,
    pub bind_group: wgpu::BindGroup,
}

impl FxDamageResource {
    /// 데미지 파티클 쉐이더 리소스의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(Fx(Damage))"),
                entries: &[
                    // 0번 바인딩: 데미지 파티클 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(FxDamageUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                    // 1번 바인딩: 데미지 폰트 텍스처 뷰
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 2번 바인딩: 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
        })
    }
}

impl FxDamageResource {
    /// 초기화 되지 않은 새로운 데미지 파티클 쉐이더 리소스를 생성합니다.
    pub fn uninit(
        label: Option<&str>,
        device: &wgpu::Device,
        t_font: &wgpu::TextureView,
        s_font: &wgpu::Sampler,
    ) -> Self {
        let tag = format!("Uniform(Fx({}))", label.unwrap_or("Unknonw"));
        let uniform_buffer = FxDamageUniform::uninit(Some(&tag), device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(t_font),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(s_font),
                },
            ],
        });

        Self {
            uniform_buffer,
            bind_group,
        }
    }
}

/// 쉐이더 모듈을 생성합니다.
fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/fx_damage.wgsl"
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
        label: Some("PipelineLayout(Bullet)"),
        bind_group_layouts: &[
            CameraResource::bind_group_layout(device),
            FxDamageResource::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    })
}

/// 지형 모델 렌더링 파이프라인을 생성합니다.
pub fn create_fx_damage_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(Fx(Damage))"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Cw,
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            polygon_mode: wgpu::PolygonMode::Fill,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            depth_compare: wgpu::CompareFunction::Always,
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                format: render_target_format,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    Arc::new(pipeline)
}

/// 데미지를 표시하는 파티클 엔터티를 생성합니다.
///
/// 생성되는 엔터티는 아래 컴포넌트를 갖습니다.
/// - 부모 엔터티(`Parent`) 주의: 부모에서 자식 엔터티로 연결되지는 않음.
/// - 라이프타임(`LifeTime`)
/// - 파티클 종류(`ParticleKind`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
/// - 데미지 파티클 쉐이더 리소스(`Arc<FxDamageResource>`)
///
pub fn spawn_damage_fx(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    t_font: &wgpu::TextureView,
    s_font: &wgpu::Sampler,
    world: &World,
    parent: Entity,
    life_time: f32,
    width: f32,
    height: f32,
    position_v: [f32; 3],
    number: u32,
) -> (Entity, EntityBuilder) {
    // 엔터티를 하나 할당 받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트를 준비합니다.
    let kind = ParticleKind::Damage;
    let life_time = LifeTime(life_time);
    let local_transform = ToParentTrans::default();
    let world_transform = WorldTransform::default();

    // 쉐이더 리소스를 준비합니다.
    let resource = Arc::new(FxDamageResource::uninit(None, device, t_font, s_font));
    resource.uniform_buffer.update(
        device,
        queue,
        FxDamageDataLayout {
            number,
            width,
            height,
            position_v,
            ..Default::default()
        },
    );

    // 컴포넌트를 추가합니다.
    builder.add(kind);
    builder.add(life_time);
    builder.add(Parent(parent));
    builder.add(local_transform);
    builder.add(world_transform);
    builder.add(resource.clone());
    builder.add(Damage {
        width,
        height,
        number,
        position_v,
    });

    (entity, builder)
}

/// 데미지 파티클을 렌더링합니다.
pub fn draw_damage_particle<'a>(
    world: &'a World,
    camera_resource: &'a CameraResource,
    device: &wgpu::Device,
    render_target_format: wgpu::TextureFormat,
    depth_stencil_format: wgpu::TextureFormat,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    let mut query = world.query::<&Arc<FxDamageResource>>();
    for (_, fx_resource) in query.iter() {
        // Skybox 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
        let pipeline = GraphicsPipelinePool::get_or_init(FX_DAMAGE_PIPELINE_NAME, || {
            create_fx_damage_render_pipeline(device, depth_stencil_format, render_target_format)
        });
        rpass.set_pipeline(&pipeline);

        // 카메라 쉐이더 리소스를 랜더 패스에 바인드합니다.
        rpass.set_bind_group(0, &camera_resource.bind_group, &[]);

        // 파티클 쉐이더 리소스를  랜더 패스에 바인드합니다.
        rpass.set_bind_group(1, &fx_resource.bind_group, &[]);

        // 파티클을 그립니다.
        rpass.draw(0..4, 0..1);
    }
}

/// 데미지 폰트 텍스처를 가져옵니다.
pub fn get_damage_font(
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Arc<wgpu::Texture>, ModelAssetError> {
    TexturePool::get_or_init(
        "Fx(D_Font_Normal)",
        move || -> Result<Arc<wgpu::Texture>, ModelAssetError> {
            let path = "font/D_Font_Normal.dds";
            let cached_asset = asset_manager
                .get_or_init(&path)
                .map_err(|e| ModelAssetError::from(e))?;

            let dds = Dds::read(Cursor::new(cached_asset.as_bytes()))
                .map_err(|e| ModelAssetError::from(e))?;

            let texture = device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(&"Texture(D_Font_Normal)"),
                    size: wgpu::Extent3d {
                        width: dds.get_width(),
                        height: dds.get_height(),
                        depth_or_array_layers: 1,
                    },
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bc7RgbaUnorm,
                    mip_level_count: dds.get_num_mipmap_levels(),
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &dds.data,
            );

            asset_manager.remove(path);
            Ok(Arc::new(texture))
        },
    )
}
