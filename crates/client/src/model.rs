use std::{collections::HashMap, io::Cursor, sync::Arc};

use mod_app::asset::AssetBundle;
use mod_asset::model::{AnimationBlob, MaterialBlob, MeshBlob, ModelBlob, NodeBlob, TextureBlob};
use mod_parallelism::collections::{Queue, SkipMap};
use mod_world::{
    component::{GameObject, IdGenerator, WorldID}, 
    render::{
        animation::{AnimationClip, KeyFrame, Skinning}, 
        material::{Material, MaterialBuilder}, 
        mesh::{Indices, Mesh, MeshBuilder, SkinnedMesh, SkinningData, VertexAttributeValues}, 
        pipeline::mesh::{model::ModelRenderer, shape::ShapeRenderer, MeshRenderer}, 
        pool::{SamplerPool, TexturePool, TextureViewPool}
    }
};

/// `Aris_Original` 모델의 경로입니다.
const ARIS_ORIGINAL_PATH: &'static str = "characters/aris_original/Aris_Original_Mesh.ron";

/// `Sphere` 모양의 경로입니다.
const SPHERE_PATH: &'static str = "shape/sphere/Sphere.ron";



pub fn spawn_sphere_shape(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    id_generator: &Arc<IdGenerator>, 
    renderer: &Arc<Queue<Arc<dyn MeshRenderer>>>,
    bundle: &AssetBundle, 
    device: &wgpu::Device, 
    queue: &wgpu::Queue
) -> WorldID {
    let cache = bundle.get_or_init(SPHERE_PATH).unwrap();
    let blob: ModelBlob = ron::de::from_bytes(cache.as_bytes()).unwrap();
    let root_id = spawn_shape_node(
        world, 
        id_generator, 
        renderer, 
        bundle, 
        device, 
        queue, 
        None, 
        blob.root, 
        Vec::new()
    );

    root_id
}

/// 모양 노드를 생성합니다.
#[must_use]
fn spawn_shape_node(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    id_generator: &Arc<IdGenerator>, 
    renderer: &Arc<Queue<Arc<dyn MeshRenderer>>>, 
    bundle: &AssetBundle,
    device: &wgpu::Device,
    queue: &wgpu::Queue, 
    parent: Option<WorldID>, 
    mut blob: NodeBlob, 
    mut sibling: Vec<NodeBlob>, 
) -> WorldID {
    // 새로운 게임 오브젝트를 생성합니다.
    let name = blob.name.clone();
    let mut object = GameObject::new(id_generator, name, parent.clone());


    // 로컬 변환 행렬과 월드 변환 행렬을 생성하고 설정합니다.
    let local_transform = gmm::Matrix::load_float4x4(blob.transform);
    let world_transform = gmm::Matrix::IDENTITY;
    object.set_local_transform(local_transform);
    object.set_world_transform(world_transform);


    // 자식 데이터가 있는 경우 자식 오브젝트를 생성합니다.
    if let Some(child_blob) = blob.children.pop() {
        let child_id = spawn_shape_node(
            world, 
            id_generator, 
            renderer, 
            bundle, 
            device, 
            queue, 
            Some(object.id().clone()), 
            child_blob, 
            blob.children
        );
        object.set_child(Some(child_id));
    }

    // 형제 데이터가 있는 경우 형제 오브젝트를 생성합니다.
    if let Some(sibling_blob) = sibling.pop() {
        let sibling_id = spawn_shape_node(
            world, 
            id_generator, 
            renderer, 
            bundle, 
            device, 
            queue, 
            parent, 
            sibling_blob, 
            sibling
        );
        object.set_sibling(Some(sibling_id));
    }

    // 메쉬 데이터가 있는 경우 모델 렌더러를 생성합니다.
    if let Some(mesh_blob) = blob.mesh {
        let builder = create_mesh_builder(mesh_blob);
        let mesh = builder.build(device, queue, None);

        let materials: Vec<_> = blob.materials.into_iter()
            .map(|material_blob| {
                create_material(device, queue, bundle, material_blob)
            })
            .collect();

        let mesh_renderer = Arc::new(ShapeRenderer::new(
            object.id().clone(), 
            mesh, 
            materials, 
            device
        ));
        object.insert(mesh_renderer.clone());
        renderer.push(mesh_renderer);
    }

    let id = object.id().clone();
    world.insert(id.clone(), object);
    id
}



pub fn spawn_aris_original_model(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    id_generator: &Arc<IdGenerator>, 
    renderer: &Arc<Queue<Arc<dyn MeshRenderer>>>, 
    bundle: &AssetBundle,
    device: &wgpu::Device, 
    queue: &wgpu::Queue
) -> (WorldID, Vec<AnimationClip>, HashMap<String, WorldID>) {
    let cache = bundle.get_or_init(ARIS_ORIGINAL_PATH).unwrap();
    let blob: ModelBlob = ron::de::from_bytes(cache.as_bytes()).unwrap();

    let mut nodes = HashMap::new();
    let mut skinned_meshes = HashMap::new();
    let root_id = spawn_model_node(
        world, 
        id_generator, 
        renderer, 
        bundle, 
        device, 
        queue, 
        &mut nodes, 
        &mut skinned_meshes, 
        None, 
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
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    id_generator: &Arc<IdGenerator>, 
    renderer: &Arc<Queue<Arc<dyn MeshRenderer>>>, 
    bundle: &AssetBundle,
    device: &wgpu::Device,
    queue: &wgpu::Queue, 
    nodes: &mut HashMap<String, WorldID>, 
    skinned_meshes: &mut HashMap<String, Arc<SkinnedMesh>>, 
    parent: Option<WorldID>, 
    mut blob: NodeBlob, 
    mut sibling: Vec<NodeBlob>, 
) -> WorldID {
    // 새로운 게임 오브젝트를 생성합니다.
    let name = blob.name.clone();
    let mut object = GameObject::new(id_generator, name, parent.clone());


    // 로컬 변환 행렬과 월드 변환 행렬을 생성하고 설정합니다.
    let local_transform = gmm::Matrix::load_float4x4(blob.transform);
    let world_transform = gmm::Matrix::IDENTITY;
    object.set_local_transform(local_transform);
    object.set_world_transform(world_transform);


    // 자식 데이터가 있는 경우 자식 오브젝트를 생성합니다.
    if let Some(child_blob) = blob.children.pop() {
        let child_id = spawn_model_node(
            world, 
            id_generator, 
            renderer, 
            bundle, 
            device, 
            queue, 
            nodes, 
            skinned_meshes, 
            Some(object.id().clone()), 
            child_blob, 
            blob.children
        );
        object.set_child(Some(child_id));
    }

    // 형제 데이터가 있는 경우 형제 오브젝트를 생성합니다.
    if let Some(sibling_blob) = sibling.pop() {
        let sibling_id = spawn_model_node(
            world, 
            id_generator, 
            renderer, 
            bundle, 
            device, 
            queue, 
            nodes, 
            skinned_meshes, 
            parent, 
            sibling_blob, 
            sibling
        );
        object.set_sibling(Some(sibling_id));
    }

    // 메쉬 데이터가 있는 경우 모델 렌더러를 생성합니다.
    if let Some(mesh_blob) = blob.mesh {
        let mesh_name = mesh_blob.name.clone();
        let builder = create_mesh_builder(mesh_blob);
        let skinning = blob.skin.map(|skin_blob| {
            SkinningData {
                quality: skin_blob.quality.min(4), 
                root_bone: nodes.get(&skin_blob.root_bone).unwrap().clone(), 
                bones: skin_blob.bone_names.iter()
                    .map(|name| {
                        nodes.get(name).unwrap().clone()
                    })
                    .collect(), 
                bindpose: skin_blob.bindposes
            }
        });

        let mesh = builder.build(device, queue, skinning);
        if let Mesh::SkinnedMesh(mesh) = mesh.clone() {
            skinned_meshes.insert(mesh_name.clone(), mesh);
        }

        let materials: Vec<_> = blob.materials.into_iter()
            .map(|material_blob| {
                create_material(device, queue, bundle, material_blob)
            })
            .collect();

        let mesh_renderer = Arc::new(ModelRenderer::new(
            object.id().clone(), 
            mesh, 
            materials, 
            device
        ));
        object.insert(mesh_renderer.clone());
        renderer.push(mesh_renderer);
    }

    let id = object.id().clone();
    nodes.insert(blob.name, id.clone());
    world.insert(id.clone(), object);
    id
}

/// 메쉬 빌더를 생성합니다.
#[must_use]
fn create_mesh_builder(blob: MeshBlob) -> MeshBuilder {
    // 메쉬 빌더를 생성합니다.
    let mut builder = MeshBuilder::new(blob.name, blob.vertices);

    if !blob.colors.is_empty() {
        builder = builder.with_attribute(VertexAttributeValues::Colors(blob.colors));
    }

    if !blob.normals.is_empty() {
        builder = builder.with_attribute(VertexAttributeValues::Normals(blob.normals));
    }

    if !blob.tangents.is_empty() {
        builder = builder.with_attribute(VertexAttributeValues::Tangents(blob.tangents));
    }

    if !blob.texcoords0.is_empty() {
        builder = builder.with_attribute(VertexAttributeValues::Texcoords0(blob.texcoords0));
    }

    if !blob.texcoords1.is_empty() {
        builder = builder.with_attribute(VertexAttributeValues::Texcoords1(blob.texcoords1));
    }

    if !blob.bone_indices.is_empty() {
        builder = builder.with_attribute(VertexAttributeValues::BoneIndices(blob.bone_indices));
    }

    if !blob.bone_weights.is_empty() {
        builder = builder.with_attribute(VertexAttributeValues::BoneWeights(blob.bone_weights));
    }

    for submesh in blob.submeshes {
        builder = builder.with_submesh(Indices(submesh));
    }

    builder
}

fn create_material(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    bundle: &AssetBundle, 
    blob: MaterialBlob
) -> Arc<Material> {
    // 재질 빌더를 생성합니다.
    let mut builder = MaterialBuilder::new(blob.name, device, queue);

    if let Some(glossiness) = blob.glossiness {
        builder.glossiness = glossiness;
    }

    if let Some(smoothness) = blob.smoothness {
        builder.smoothness = smoothness;
    }

    if let Some(metallic) = blob.metallic {
        builder.metallic = metallic;
    }

    if let Some(diffuse) = blob.diffuse {
        builder.diffuse = diffuse;
    }

    if let Some(specular) = blob.specular {
        builder.specular = specular;
    }

    if let Some(emissive) = blob.emissive {
        builder.emissive = emissive;
    }

    if let Some(texture_blob) = blob.diffuse_map {
        let (texture_view, sampler) = create_texture(device, queue, bundle, texture_blob);
        builder.diffuse_map = texture_view;
        builder.diffuse_sampler = sampler;
    }

    if let Some(texture_blob) = blob.specular_map {
        let (texture_view, sampler) = create_texture(device, queue, bundle, texture_blob);
        builder.specular_map = texture_view;
        builder.specular_sampler = sampler;
    }

    if let Some(texture_blob) = blob.normal_map {
        let (texture_view, sampler) = create_texture(device, queue, bundle, texture_blob);
        builder.normal_map = texture_view;
        builder.normal_sampler = sampler;
    }

    if let Some(texture_blob) = blob.emissive_map {
        let (texture_view, sampler) = create_texture(device, queue, bundle, texture_blob);
        builder.emissive_map = texture_view;
        builder.emissive_sampler = sampler;
    }

    builder.build(device, queue)
}

fn create_texture(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    bundle: &AssetBundle,
    blob: TextureBlob
) -> (Arc<wgpu::TextureView>, Arc<wgpu::Sampler>) {
    let texture = match TexturePool::get(&blob.name) {
        Some(texture) => texture, 
        None => {
            let path = format!("characters/aris_original/{}.dds", &blob.name);
            let cache = bundle.get_or_init(path).unwrap();
            let dds = ddsfile::Dds::read(Cursor::new(cache.as_bytes())).unwrap();
            TexturePool::get_or_init(
                device, 
                queue, 
                blob.name.clone(),&wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", &blob.name)), 
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
                &dds.data
            )
        }
    };

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
    nodes: &HashMap<String, WorldID>,
    skinned_meshes: &HashMap<String, Arc<SkinnedMesh>>,
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
                    .map(|blob| Skinning {
                        skinned_mesh: skinned_meshes.get(&blob.name).unwrap().clone(), 
                        transforms: blob.transforms.into_iter()
                            .map(|transform| transform.into())
                            .collect()
                    })
            ))
    )
}
