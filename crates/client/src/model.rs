use std::{collections::HashMap, io::Cursor, sync::Arc};

use ddsfile::Dds;
use mod_app::asset::AssetBundle;
use mod_asset::model::{AnimationBlob, MaterialBlob, MeshBlob, ModelBlob, NodeBlob, TextureBlob};
use mod_world::{
    objects::{GameObjectDescriptor, GameWorld, ObjectId}, 
    render::{
        animation::{AnimationClip, KeyFrame, SkinnedMeshInfo, SkinningData}, 
        brush::{bullet::BulletBrush, student::StudentBrush}, 
        material::universal::{UniversalMaterialDescriptor, UniversalMaterialResource}, 
        mesh::{
            AttributeValues, BoneMatrixDataLayout, DynamicMeshDataLayout, DynamicMeshResource, Indices, Mesh, StaticMeshDataLayout, StaticMeshResource, Vertices, MAX_BONES
        }, 
        pool::{MeshPool, SamplerPool, TexturePool, TextureViewPool}
    }, task::DrawTask
};
use wgpu::util::DeviceExt;

/// `Aris_Original` 모델의 경로입니다.
const ARIS_ORIGINAL_PATH: &'static str = "characters/aris_original/Aris_Original_Mesh.ron";

/// `Sphere` 모양의 경로입니다.
const SPHERE_PATH: &'static str = "shape/sphere/Sphere.ron";



pub fn spawn_sphere_shape(
    world: &GameWorld,  
    bundle: &AssetBundle, 
    device: &wgpu::Device, 
    queue: &wgpu::Queue
) -> ObjectId {
    let cache = bundle.get_or_init(SPHERE_PATH).unwrap();
    let blob: ModelBlob = ron::de::from_bytes(cache.as_bytes()).unwrap();
    let root_id = spawn_shape_node(
        world, 
        bundle, 
        device, 
        queue, 
        ObjectId::NIL, 
        blob.root, 
        Vec::new()
    );

    root_id
}

/// 모양 노드를 생성합니다.
#[must_use]
fn spawn_shape_node(
    world: &GameWorld, 
    bundle: &AssetBundle,
    device: &wgpu::Device,
    queue: &wgpu::Queue, 
    parent: ObjectId, 
    mut blob: NodeBlob, 
    mut sibling: Vec<NodeBlob>, 
) -> ObjectId {
    // 로컬 변환 행렬과 월드 변환 행렬을 생성하고 설정합니다.
    let local_transform = gmm::Matrix::load_float4x4(blob.transform);
    let world_transform = gmm::Matrix::IDENTITY;

    // 새로운 게임 오브젝트를 생성합니다.
    let name = blob.name.clone();
    let desc = GameObjectDescriptor::new()
        .with_name(name)
        .with_parent(parent)
        .with_local_transform(local_transform)
        .with_world_transform(world_transform);
    let id = world.spawn(desc);


    // 자식 데이터가 있는 경우 자식 오브젝트를 생성합니다.
    if let Some(child_blob) = blob.children.pop() {
        let child_id = spawn_shape_node(
            world, 
            bundle, 
            device, 
            queue, 
            id, 
            child_blob, 
            blob.children
        );

        let mut object = unsafe { world.get_mut(&id).unwrap_unchecked() };
        object.child = child_id;
    }

    // 형제 데이터가 있는 경우 형제 오브젝트를 생성합니다.
    if let Some(sibling_blob) = sibling.pop() {
        let sibling_id = spawn_shape_node(
            world, 
            bundle, 
            device, 
            queue, 
            parent, 
            sibling_blob, 
            sibling
        );

        let mut object = unsafe { world.get_mut(&id).unwrap_unchecked() };
        object.sibling = sibling_id;
    }

    // 메쉬 데이터가 있는 경우 모델 렌더러를 생성합니다.
    if let Some(mesh_blob) = blob.mesh {
        let name = mesh_blob.name.clone();

        // 메쉬를 생성합니다.
        let mesh = create_mesh(device, queue, mesh_blob);

        // 메쉬 쉐이더 리소스를 생성합니다.
        let mesh_resource = Arc::new(StaticMeshResource::new(
            Some(&format!("StaticMeshResource({})", &name)), 
            device
        ));

        // 재질을 쉐이더 리소스를 생성합니다.
        let materials: Vec<_> = blob.materials.into_iter()
            .map(|material_blob| {
                create_material(device, queue, bundle, material_blob)
            })
            .collect();

        // 그리기 브러쉬를 생성합니다.
        let brush = BulletBrush::new(
            device, 
            mesh, 
            mesh_resource.clone(), 
            materials
        );

        // 그리기 작업을 생성합니다.
        let mesh_resource_cloned = mesh_resource.clone();
        let task = DrawTask::new(id)
            .with_on_pre_draw(Some(Box::new(move |device, queue, world, id| {
                let object = world.get(&id).unwrap();
                mesh_resource_cloned.mesh_uniform().write(device, queue, StaticMeshDataLayout {
                    trans: object.world_transform.into(), 
                    ..Default::default()
                });

                Ok(())
            })))
            .add_brush(brush);

        // 게임 세상에 그리기 작업을 등록합니다.
        world.regist_draw_task(task).unwrap();
    }

    id
}



pub fn spawn_aris_original_model(
    world: &GameWorld, 
    bundle: &AssetBundle,
    device: &wgpu::Device, 
    queue: &wgpu::Queue
) -> (ObjectId, Vec<AnimationClip>, HashMap<String, ObjectId>) {
    let cache = bundle.get_or_init(ARIS_ORIGINAL_PATH).unwrap();
    let blob: ModelBlob = ron::de::from_bytes(cache.as_bytes()).unwrap();

    let mut nodes = HashMap::new();
    let mut skinned_meshes = HashMap::new();
    let root_id = spawn_model_node(
        world, 
        bundle, 
        device, 
        queue, 
        &mut nodes, 
        &mut skinned_meshes, 
        ObjectId::NIL, 
        blob.root, 
        Vec::new()
    );

    let animations: Vec<_> = blob.animations.into_iter()
        .map(|animation_blob| create_animations(
            &nodes, 
            &skinned_meshes, 
            animation_blob
        ))
        .collect();

    (root_id, animations, nodes)
}


/// 모델 노드를 생성합니다.
#[must_use]
fn spawn_model_node(
    world: &GameWorld, 
    bundle: &AssetBundle,
    device: &wgpu::Device,
    queue: &wgpu::Queue, 
    nodes: &mut HashMap<String, ObjectId>, 
    skinned_meshes: &mut HashMap<String, Arc<SkinnedMeshInfo>>, 
    parent: ObjectId, 
    mut blob: NodeBlob, 
    mut sibling: Vec<NodeBlob>, 
) -> ObjectId {
    // 로컬 변환 행렬과 월드 변환 행렬을 생성하고 설정합니다.
    let local_transform = gmm::Matrix::load_float4x4(blob.transform);
    let world_transform = gmm::Matrix::IDENTITY;

    // 새로운 게임 오브젝트를 생성합니다.
    let name = blob.name.clone();
    let desc = GameObjectDescriptor::new()
        .with_name(name)
        .with_parent(parent)
        .with_local_transform(local_transform)
        .with_world_transform(world_transform);
    let id = world.spawn(desc);


    // 자식 데이터가 있는 경우 자식 오브젝트를 생성합니다.
    if let Some(child_blob) = blob.children.pop() {
        let child_id = spawn_model_node(
            world, 
            bundle, 
            device, 
            queue, 
            nodes, 
            skinned_meshes, 
            id, 
            child_blob, 
            blob.children
        );

        let mut object = unsafe { world.get_mut(&id).unwrap_unchecked() };
        object.child = child_id;
    }

    // 형제 데이터가 있는 경우 형제 오브젝트를 생성합니다.
    if let Some(sibling_blob) = sibling.pop() {
        let sibling_id = spawn_model_node(
            world, 
            bundle, 
            device, 
            queue, 
            nodes, 
            skinned_meshes, 
            parent, 
            sibling_blob, 
            sibling
        );

        let mut object = unsafe { world.get_mut(&id).unwrap_unchecked() };
        object.sibling = sibling_id;
    }

    // 메쉬 데이터가 있는 경우 모델 렌더러를 생성합니다.
    if let Some(mesh_blob) = blob.mesh {
        let name = mesh_blob.name.clone();
        let mesh = create_mesh(device, queue, mesh_blob);
        if let Some(skin_blob) = blob.skin {
            // 쉐이더 리소스를 생성합니다.
            let resource = Arc::new(DynamicMeshResource::new(
                Some(&format!("DynamicMeshResource({})", &name)), 
                device
            ));
            
            // 메쉬 데이터 유니폼 버퍼 갱신
            resource.mesh_uniform().write(device, queue, DynamicMeshDataLayout {
                quality: skin_blob.quality.min(4), 
                num_bones: skin_blob.bone_names.len().min(MAX_BONES) as u32, 
                ..Default::default()
            });

            // 뼈 바인드 포즈 데이터 유니폼 버퍼 갱신.
            let mut data = BoneMatrixDataLayout::new();
            for (index, &transform) in skin_blob.bindposes.iter().enumerate() {
                data[index] = transform.into();
            }
            resource.bindpose_uniform().write(device, queue, data);


            // 스키닝된 메쉬 데이터를 생성합니다.
            let mesh_info = Arc::new(SkinnedMeshInfo {
                root_bone: nodes.get(&skin_blob.root_bone).unwrap().clone(), 
                mesh_uniform: resource.mesh_uniform().clone(), 
                bindpose_uniform: resource.bindpose_uniform().clone(), 
                bone_transform_uniform: resource.bone_transform_uniform().clone(), 
                bones: skin_blob.bone_names.iter()
                .map(|name| {
                    nodes.get(name).unwrap().clone()
                })
                .collect()
            });

            skinned_meshes.insert(name.clone(), mesh_info.clone());
            
            // 재질을 생성합니다.
            let materials: Vec<_> = blob.materials.into_iter()
                .map(|material_blob| {
                    create_material(device, queue, bundle, material_blob)
                })
                .collect();

            // 그리기 브러쉬를 생성합니다.
            let brush = StudentBrush::new(
                device, 
                mesh, 
                resource, 
                materials, 
            );
            
            // 그리기 작업을 생성합니다.
            let task = DrawTask::new(id)
                .with_on_pre_draw(Some(Box::new(|device, queue, world, id| {
                    let object = world.get(&id).unwrap();
                    let mesh_info = object.get::<Arc<SkinnedMeshInfo>>().unwrap();

                    let mut data = BoneMatrixDataLayout::new();
                    for (index, id) in mesh_info.bones.iter().enumerate() {
                        let bone = world.get(id).unwrap();
                        data[index] = bone.world_transform.into();
                    }

                    mesh_info.bone_transform_uniform.write(device, queue, data);

                    Ok(())
                })))
                .add_brush(brush);

            // 게임 세상에 그리기 작업을 등록합니다.
            world.regist_draw_task(task).unwrap();

            let mut object = unsafe { world.get_mut(&id).unwrap_unchecked() };
            object.insert(mesh_info);
        };
    }

    nodes.insert(blob.name, id.clone());
    id
}

/// 메쉬를 생성합니다.
#[must_use]
fn create_mesh(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    blob: MeshBlob
) -> Arc<Mesh> {
    let name = blob.name.clone();
    MeshPool::get_or_init(name, move || {
        // 메쉬 빌더를 생성합니다.
        let mut mesh = Mesh::new(&blob.name, device, queue, Vertices(blob.vertices));
    
        if !blob.colors.is_empty() {
            mesh.insert_attribute(device, queue, AttributeValues::Color(blob.colors));
        }
    
        if !blob.normals.is_empty() {
            mesh.insert_attribute(device, queue, AttributeValues::Normal(blob.normals));
        }
    
        if !blob.tangents.is_empty() {
            mesh.insert_attribute(device, queue, AttributeValues::Tangent(blob.tangents));
        }
    
        if !blob.texcoords0.is_empty() {
            mesh.insert_attribute(device, queue, AttributeValues::Texcoord0(blob.texcoords0));
        }
    
        if !blob.texcoords1.is_empty() {
            mesh.insert_attribute(device, queue, AttributeValues::Texcoord1(blob.texcoords1));
        }
    
        if !blob.bone_indices.is_empty() {
            mesh.insert_attribute(device, queue, AttributeValues::BoneIndex(blob.bone_indices));
        }
    
        if !blob.bone_weights.is_empty() {
            mesh.insert_attribute(device, queue, AttributeValues::BoneWeight(blob.bone_weights));
        }
    
        for submesh in blob.submeshes {
            mesh.insert_submesh(device, queue, Indices::U32(submesh));
        }
    
        mesh.into()
    })
}

fn create_material(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    bundle: &AssetBundle, 
    blob: MaterialBlob
) -> Arc<UniversalMaterialResource> {
    // 재질 설명자를 생성합니다.
    let mut desc = UniversalMaterialDescriptor::new(device, queue, blob.name);

    if let Some(glossiness) = blob.glossiness {
        desc.glossiness = glossiness;
    }

    if let Some(smoothness) = blob.smoothness {
        desc.smoothness = smoothness;
    }

    if let Some(metallic) = blob.metallic {
        desc.metallic = metallic;
    }

    if let Some(diffuse) = blob.diffuse {
        desc.albedo = diffuse.into();
    }

    if let Some(specular) = blob.specular {
        desc.specular = specular.into();
    }

    if let Some(emissive) = blob.emissive {
        desc.emissive = emissive.into();
    }

    if let Some(texture_blob) = blob.diffuse_map {
        let (texture_view, sampler) = create_texture(device, queue, bundle, texture_blob);
        desc.albedo_map = texture_view;
        desc.albedo_sampler = sampler;
    }

    if let Some(texture_blob) = blob.specular_map {
        let (texture_view, sampler) = create_texture(device, queue, bundle, texture_blob);
        desc.specular_map = texture_view;
        desc.specular_sampler = sampler;
    }

    if let Some(texture_blob) = blob.normal_map {
        let (texture_view, sampler) = create_texture(device, queue, bundle, texture_blob);
        desc.normal_map = texture_view;
        desc.normal_sampler = sampler;
    }

    if let Some(texture_blob) = blob.emissive_map {
        let (texture_view, sampler) = create_texture(device, queue, bundle, texture_blob);
        desc.emissive_map = texture_view;
        desc.emissive_sampler = sampler;
    }

    Arc::new(UniversalMaterialResource::new(device, queue, &desc))
}

fn create_texture(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    bundle: &AssetBundle,
    blob: TextureBlob
) -> (Arc<wgpu::TextureView>, Arc<wgpu::Sampler>) {
    let name = blob.name.clone();
    let texture = TexturePool::get_or_init(&blob.name, move || {
        let path = format!("characters/aris_original/{}.dds", &name);
        let cache = bundle.get_or_init(&path).unwrap();
        let dds = Dds::read(Cursor::new(cache.as_bytes())).unwrap();
        let texture = Arc::new(device.create_texture_with_data(
            queue, 
            &wgpu::TextureDescriptor {
                label: Some(&format!("Texture({})", &name)), 
                size: wgpu::Extent3d {
                    width: dds.get_width(), 
                    height: dds.get_height(), 
                    depth_or_array_layers: dds.get_depth(), 
                }, 
                dimension: if dds.get_depth() > 1 { wgpu::TextureDimension::D3 } else { wgpu::TextureDimension::D2 }, 
                format: wgpu::TextureFormat::Bc7RgbaUnorm, 
                mip_level_count: dds.get_num_mipmap_levels(), 
                sample_count: 1, 
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, 
                view_formats: &[]
            }, 
            wgpu::util::TextureDataOrder::LayerMajor, 
            &dds.data
        ));

        bundle.remove(&path);

        texture
    });

    let texture_view = TextureViewPool::get_or_init(
        &texture, 
        &wgpu::TextureViewDescriptor {
            ..Default::default()
        }
    );

    let (_, sampler) = SamplerPool::get_or_init(
        device, 
        &wgpu::SamplerDescriptor {
            label: Some(&format!("Smapler({})", &blob.name)), 
            address_mode_u: blob.address_u.into(), 
            address_mode_v: blob.address_v.into(), 
            address_mode_w: blob.address_w.into(), 
            mag_filter: blob.filter_mode.into(), 
            min_filter: blob.filter_mode.into(), 
            mipmap_filter: blob.filter_mode.into(), 
            ..Default::default()
        }
    );

    (texture_view, sampler)
}

fn create_animations(
    nodes: &HashMap<String, ObjectId>,
    skinned_meshes: &HashMap<String, Arc<SkinnedMeshInfo>>,
    blob: AnimationBlob
) -> AnimationClip {
    AnimationClip::new(
        blob.name, 
        nodes.get(&blob.root_name).unwrap().clone(), 
        blob.length, 
        blob.frame_rate, 
        blob.keyframes.into_iter()
            .map(|blob| KeyFrame::new(
                blob.time_point, 
                blob.root.into(), 
                blob.meshes.into_iter()
                    .map(|blob| SkinningData {
                        mesh: skinned_meshes.get(&blob.name).unwrap().clone(), 
                        transforms: blob.transforms.into_iter()
                            .map(|transform| transform.into())
                            .collect()
                    })
            ))
    )
}
