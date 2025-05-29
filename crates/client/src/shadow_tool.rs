//! 그림자 맵을 생성하는 도구 프로그램 입니다.
//!
use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufWriter, Read},
    path::{Path, PathBuf},
    process::exit,
    sync::mpsc,
};

use ahash::HashMap;
use asset::{
    MeshPool, ModelPool, SamplerPool, StageBoundingVolumn, StageBoundingVolumnHierarchy,
    TextureDataPool, TexturePool, TextureViewPool, STAGE_URI, STAGE_WORKSPACES,
};
use component::{
    bake_stage, build_stage, update_entity_hierarchy, update_stage_resource, Child,
    LightTransformDataLayout, MaterialDataPool, MaterialKind, MeshRenderer, ShadowResource,
    Sibling, SkinnedMeshRenderer, StageBakePipeline, WorldTransform,
};
use hecs::{Entity, World};
use image::{GrayImage, Luma};
use mod_network::components::{StageKind, StageLayoutData};
use mod_render::init_wgpu;

mod asset;
mod component;
mod config;

/// 그림자 맵의 설정 데이터입니다.
pub struct ShadowMapConfig {
    current_dir: PathBuf,
    kind: StageKind,
    texture_size: u32,
    view_size: u32,
    view_depth: u32,
}

fn main() {
    // 명령 줄 인자로 부터 현재 실행 경로와 지형 종류를 가져옵니다.
    let config = parse_command_line_args();
    println!(
        "정보\n\t- 현재 경로:{}\n\t- 지형 종류:{:?}\n\t- 텍스처 크기:{}x{}\n\t- 캡쳐 크기:{}x{}x{}\n",
        config.current_dir.display(),
        &config.kind,
        config.texture_size,
        config.texture_size,
        config.view_size,
        config.view_size,
        config.view_depth,
    );

    println!("wgpu 렌더러를 초기화합니다...");
    let result = pollster::block_on(init_wgpu());
    let (_instance, _adapter, device, queue) = match result {
        Ok(it) => it,
        Err(e) => {
            eprintln!("wgpu 렌더러 초기화에 실패했습니다. (사유:{e})");
            exit(-1);
        }
    };

    println!("렌더링 파이프라인을 초기화합니다...");
    let pipeline = StageBakePipeline::get_or_init(&device, wgpu::TextureFormat::Depth32Float);
    let depth_target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Target"),
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        mip_level_count: 1,
        sample_count: 1,
        size: wgpu::Extent3d {
            width: config.texture_size,
            height: config.texture_size,
            depth_or_array_layers: 1,
        },
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth_target_view = depth_target.create_view(&wgpu::TextureViewDescriptor::default());

    println!("지형 데이터를 불러옵니다...");
    let i = config.kind as usize;
    let mut workspace = config.current_dir.clone();
    workspace.push(&format!("assets/{}", STAGE_WORKSPACES[i]));
    let layout = load_stage_layout_from_file(&workspace, STAGE_URI);

    println!("지형을 구성하는 모델 데이터를 불러옵니다...");
    let mesh_pool = MeshPool::new();
    let model_pool = ModelPool::new();
    let material_data_pool = MaterialDataPool::new();
    let texture_data_pool = TextureDataPool::new();
    let texture_pool = TexturePool::new();
    let texture_view_pool = TextureViewPool::new();
    let sampler_pool = SamplerPool::new();

    {
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        for uri in layout.models.iter() {
            let result = model_pool.get_or_init(
                &mesh_pool,
                &material_data_pool,
                &texture_data_pool,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &workspace,
                &uri,
            );

            if let Err(e) = result {
                eprintln!(
                    "지형을 구성하는 모델을 불러오는데 실패했습니다. (모델 Uri:{}, 사유:{})",
                    uri, e
                );
                exit(-1);
            }
        }

        // 렌더링 작업을 제출합니다.
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
    }

    println!("지형을 생성합니다...");
    let mut world = World::new();
    let bvh = {
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let (bvh, batch_commands) = build_stage(
            &world,
            &model_pool,
            &texture_data_pool,
            &layout,
            &device,
            &mut encoder,
            &mut staging_buffers,
        );

        // 게임 월드에 엔터티를 생성합니다.
        for (entity, mut builder) in batch_commands {
            let result = world.insert(entity, builder.build());
            if let Err(_) = result {
                eprintln!("엔터티를 찾을 수 없습니다!");
                exit(-1);
            }
        }

        // 렌더링 작업을 제출합니다.
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::Wait);

        bvh
    };

    // 지형의 변환 행렬을 갱신합니다.
    println!("지형을 배치합니다...");
    let mut shadow_map = HashMap::default();
    let mut opaque_map = HashMap::default();
    let mut transparent_map = HashMap::default();

    let entities = collect_stage_entity(&bvh);
    for entity in entities.iter().cloned() {
        update_entity_hierarchy(&mut world, entity, glam::Mat4::IDENTITY);
    }

    {
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let child_view = &world.view::<&Child>();
        let sibling_view = &world.view::<&Sibling>();
        let transform_view = &world.view::<&WorldTransform>();
        let mesh_filter_view = &mut world.view::<MeshRenderer>();
        let skinned_mesh_filter_view = &mut world.view::<SkinnedMeshRenderer>();

        for entity in entities {
            update_stage_resource(
                entity,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &mut shadow_map,
                &mut opaque_map,
                &mut transparent_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        // 렌더링 작업을 제출합니다.
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
    }

    println!("조명을 준비합니다...");
    // 가장 맨 처음 Directional Light를 찾습니다.
    let light_dir: glam::Vec3A = match layout.global_light {
        Some(light) => light.direction_w.into(),
        None => {
            eprintln!("전역 조명이 게임 월드에 존재하지 않습니다!\n프로그램을 종료합니다.");
            exit(-1);
        }
    };

    let eye = -light_dir * 50.0;
    let light_view = glam::Mat4::look_at_lh(eye.into(), glam::Vec3::ZERO, glam::Vec3::Y);
    let light_proj = glam::Mat4::orthographic_lh(
        -(config.view_size as f32),
        config.view_size as f32,
        -(config.view_size as f32),
        config.view_size as f32,
        0.01,
        config.view_depth as f32,
    );
    let light_proj_view = light_proj * light_view;
    println!("{}", light_proj_view);

    let light_resource = ShadowResource::new(Some("Main"), &device, depth_target_view);
    {
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        light_resource.uniform.update(
            &device,
            &mut encoder,
            &mut staging_buffers,
            LightTransformDataLayout {
                proj_view: light_proj_view.to_cols_array(),
            },
        );

        // 렌더링 작업을 제출합니다.
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
    }

    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(ShadowMap)"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &light_resource.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for ((mesh, kind), resource) in shadow_map.iter() {
                if *kind == MaterialKind::Stage {
                    bake_stage(mesh, pipeline, &light_resource, resource, &mut rpass);
                }
            }
        }

        // 렌더링 작업을 제출합니다.
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
    }

    // 버퍼에 텍스처 데이터를 복사합니다.
    let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("OutputStagingBuffer"),
        size: (config.texture_size * config.texture_size * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let bytes_per_pixel = std::mem::size_of::<f32>() as u32;
        let bytes_per_row = config.texture_size * bytes_per_pixel;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &depth_target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(config.texture_size),
                },
            },
            wgpu::Extent3d {
                width: config.texture_size,
                height: config.texture_size,
                depth_or_array_layers: 1,
            },
        );

        // 렌더링 작업을 제출합니다.
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
    }

    // 버퍼로부터 텍스처 데이터를 가져옵니다.
    let mut data = vec![0u8; (config.texture_size * config.texture_size * 4) as usize];
    let buffer_slice = output_staging_buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    let _ = device.poll(wgpu::PollType::Wait);

    let result = receiver.recv().unwrap();
    match result {
        Ok(_) => {
            let view = buffer_slice.get_mapped_range();
            data.copy_from_slice(&view[..]);
        }
        Err(e) => {
            eprintln!("버퍼를 읽을 수 없습니다! (사유:{})", e);
            exit(-1);
        }
    };

    // 1. f32로 변환
    let float_data: &[f32] = bytemuck::cast_slice(&data);

    // 2. 정규화 및 u8 변환
    let mut image = GrayImage::new(config.texture_size, config.texture_size);

    for y in 0..config.texture_size {
        for x in 0..config.texture_size {
            let idx = (y * config.texture_size + x) as usize;
            let depth = float_data[idx];

            // NaN 방지 및 클램핑
            let clamped = depth.clamp(0.0, 1.0);
            let scaled = (clamped * 255.0) as u8;

            image.put_pixel(x, y, Luma([scaled]));
        }
    }

    // 3. 저장
    let mut path = config.current_dir.clone();
    path.push("shadow_map.tga");

    let file = File::create(path).expect("파일 생성 실패");
    let writer = BufWriter::new(file);
    image::codecs::tga::TgaEncoder::new(writer)
        .encode(
            &image,
            config.texture_size,
            config.texture_size,
            image::ColorType::L8.into(),
        )
        .expect("이미지 저장 실패");

    println!("완료!");
}

fn load_stage_layout_from_file<Dir, Uri>(workspace: Dir, uri: Uri) -> StageLayoutData
where
    Dir: AsRef<Path>,
    Uri: AsRef<str>,
{
    let mut path = workspace.as_ref().to_path_buf();
    path.push(format!("{}.json", uri.as_ref()));

    // 파일을 읽습니다.
    let result = OpenOptions::new().read(true).write(false).open(&path);
    let mut file = match result {
        Ok(file) => file,
        Err(e) => {
            eprintln!(
                "파일 열기에 실패했습니다. (파일:{}, 사유:{})",
                path.display(),
                &e
            );
            exit(-1);
        }
    };

    // 파일 데이터를 버퍼에 저장합니다.
    let mut buf = Vec::new();
    let result = file.read_to_end(&mut buf);
    if let Err(e) = result {
        eprintln!(
            "파일 읽기에 실패했습니다. (파일:{}, 사유:{})",
            path.display(),
            &e
        );
        exit(-1);
    };
    drop(file);

    // 파일 데이터를 구문 분석합니다.
    let result = serde_json::from_slice(&buf);
    match result {
        Ok(layout) => layout,
        Err(e) => {
            eprintln!(
                "파일 구문 분석에 실패했습니다. (파일:{}, 사유:{})",
                path.display(),
                &e
            );
            exit(-1);
        }
    }
}

/// 스테이지를 구성하는 엔터티를 수집합니다.
fn collect_stage_entity(bvh: &StageBoundingVolumnHierarchy) -> Vec<Entity> {
    let mut entities = bvh.area.clone();
    if let Some(node) = bvh.root.as_ref() {
        collect_stage_entity_recursive(&node, &mut entities);
    }
    entities
}

/// 스테이지를 구성하는 엔터티를 재귀적으로 순회하며 수집합니다.
fn collect_stage_entity_recursive(node: &StageBoundingVolumn, entities: &mut Vec<Entity>) {
    entities.push(node.entity);
    if let Some(l_node) = node.left.as_ref() {
        collect_stage_entity_recursive(&l_node, entities);
    }
    if let Some(r_node) = node.right.as_ref() {
        collect_stage_entity_recursive(&r_node, entities);
    }
}

/// 명령 줄 인자로부터 지형 종류를 반환합니다.
fn parse_command_line_args() -> ShadowMapConfig {
    // 현재 애플리에키션 실행 디렉토리 경로를 가져옵니다.
    let mut args = env::args();
    let argument = match args.next() {
        Some(argument) => argument,
        None => {
            eprintln!("명령 줄 인수가 비어있습니다!");
            exit(-1);
        }
    };
    let current_exe = Path::new(&argument);
    let current_dir = current_exe
        .parent()
        .map(|p| p.canonicalize().ok())
        .flatten();
    let current_dir = match current_dir {
        Some(path) => path,
        None => {
            eprintln!("프로그램 실행 경로를 찾을 수 없습니다!");
            exit(-1);
        }
    };

    // 입력된 명령 줄 인수를 구문 분석합니다.
    let mut config = ShadowMapConfig {
        current_dir,
        kind: StageKind::default(),
        texture_size: 4096,
        view_size: 70,
        view_depth: 100,
    };
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--kind" => {
                // 지형 종류를 가져옵니다.
                let arg = match args.next() {
                    Some(arg) => arg,
                    None => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };
                let val: u8 = match arg.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };

                config.kind = match StageKind::new(val) {
                    Some(kind) => kind,
                    None => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };
            }
            "--texture-size" => {
                // 텍스처 크기를 가져옵니다.
                let arg = match args.next() {
                    Some(arg) => arg,
                    None => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };
                let size: u32 = match arg.parse() {
                    Ok(n) => {
                        if n == 0 {
                            eprintln!("주어진 텍스처 크기는 0보다 커야합니다!");
                            exit(-1)
                        }

                        if n % 256 != 0 {
                            eprintln!("주어진 텍스처의 크기는 256의 배수여야 합니다!");
                            exit(-1);
                        } else {
                            n
                        }
                    }
                    Err(_) => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };

                config.texture_size = size;
            }
            "--view-size" => {
                // 정사영 투영 행렬의 크기를 가져옵니다.
                let arg = match args.next() {
                    Some(arg) => arg,
                    None => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };
                let size: u32 = match arg.parse() {
                    Ok(n) => {
                        if n == 0 {
                            eprintln!("주어진 뷰포트 크기는 0보다 커야합니다!");
                            exit(-1)
                        } else {
                            n
                        }
                    }
                    Err(_) => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };

                config.view_size = size;
            }
            "--view-depth" => {
                // 정사영 투영 행렬의 깊이를 가져옵니다.
                let arg = match args.next() {
                    Some(arg) => arg,
                    None => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };
                let size: u32 = match arg.parse() {
                    Ok(n) => {
                        if n == 0 {
                            eprintln!("주어진 뷰포트 깊이는 0보다 커야합니다!");
                            exit(-1)
                        } else {
                            n
                        }
                    }
                    Err(_) => {
                        print_pragram_usages();
                        exit(-1);
                    }
                };

                config.view_depth = size;
            }
            _ => {
                print_pragram_usages();
                exit(-1);
            }
        };
    }

    config
}

/// 프로그램 사용 방법을 콘솔에 출력합니다.
#[rustfmt::skip]
fn print_pragram_usages() {
    eprintln!(
        "사용 방법: shadow_tool <OPTIONS>\n
        \tOptions\n
        \t\t--kind <Number> (0: City)\n
        \t\t--texture-size <Number> (Number > 0)\n
        \t\t--view-size <Number> (Number > 0)\n
        \t\t--view-depth <Number> (Number > 0)\n
        "
    );
}
