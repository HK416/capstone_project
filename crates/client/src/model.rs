use std::{collections::HashMap, io::Cursor, sync::{Arc, Weak}};

use mod_asset::model::{
    AnimationBlob, 
    MaterialBlob, 
    MeshBlob, 
    ModelBlob, 
    NodeBlob, 
    TextureBlob
};
use mod_world::{
    object::GameObject, 
    render::{
        animation::{AnimationClip, KeyFrame, Skinning}, 
        material::{Material, MaterialBuilder}, 
        mesh::{Indices, MeshBuilder, MeshRenderer, SkinnedMesh, SkinningData, VertexAttributeValues}, 
        pool::{SamplerPool, TexturePool, TextureViewPool}
    }
};
use rust_embed::Embed;

/// `Aris_Original` 모델의 경로입니다.
const PATH: &'static str = "characters/aris_original/Aris_Original_Mesh.ron";

/// 임베딩된 에셋 파일 관리자입니다.
#[derive(Embed)]
#[folder = "assets/"]
struct EmbededAssets;



/// `Aris_Original` 모델을 생성합니다.
/// 
/// ※ 임시로 사용하는 함수입니다.
/// 
pub fn spawn_aris_original(
    device: &Arc<wgpu::Device>, 
    queue: &Arc<wgpu::Queue>
) -> Arc<GameObject> {
    let embeded_file = EmbededAssets::get(&PATH).unwrap();
    let blob: ModelBlob = ron::de::from_bytes(&embeded_file.data).unwrap();
    let mut nodes = HashMap::new();
    let mut skinned_meshes = HashMap::new();
    let object = spawn_node(
        device, 
        queue, 
        &mut nodes, 
        &mut skinned_meshes, 
        None, 
        blob.root, 
        Vec::new()
    );

    if !blob.animations.is_empty() {
        let mut animations = Vec::with_capacity(blob.animations.len());
        for animation_blob in blob.animations {
            animations.push(build_animations(&skinned_meshes, animation_blob));
        }

        object.add_element(animations);
    }

    object
}

pub fn spawn_node(
    device: &Arc<wgpu::Device>, 
    queue: &Arc<wgpu::Queue>, 
    nodes: &mut HashMap<String, Arc<GameObject>>, 
    skinned_meshes: &mut HashMap<String, Arc<SkinnedMesh>>, 
    parent: Option<Weak<GameObject>>, 
    mut blob: NodeBlob, 
    mut sibling: Vec<NodeBlob>
) -> Arc<GameObject> {
    // 게임 오브젝트를 생성합니다.
    let object = GameObject::new(parent.clone(), blob.name.clone());

    // 부모로 부터 변환 행렬을 설정합니다.
    object.set_to_parent_trans(|result| {
        let mut lock_guard = result.unwrap();
        *lock_guard = blob.transform.into()
    });

    // 자식 노드가 존재하는 경우 자식 노드를 설정합니다.
    if let Some(child_blob) = blob.children.pop() {
        let child = spawn_node(
            device, 
            queue, 
            nodes, 
            skinned_meshes, 
            Arc::downgrade(&object).into(), 
            child_blob, 
            blob.children
        );
        object.set_child(child.into());
    }

    // 형제 노드가 존재하는 경우 형제 노드를 설정합니다.
    if let Some(sibling_blob) = sibling.pop() {
        let sibling = spawn_node(
            device, 
            queue, 
            nodes, 
            skinned_meshes, 
            parent.clone(), 
            sibling_blob, 
            sibling
        );
        object.set_sibling(sibling.into());
    }

    // 노드에 메쉬 데이터가 존재하는 경우 메쉬 요소를 추가합니다.
    if let Some(mesh_blob) = blob.mesh {
        let mesh_name = mesh_blob.name.clone();
        let builder = get_mesh_builder(mesh_blob);
        
        // 노드에 스키닝 데이터가 존재하는 경우 스키닝 데이터를 가져옵니다.
        let skinning = blob.skin.map(|skin_blob| {
            SkinningData {
                quality: skin_blob.quality, 
                root_bone: nodes.get(&skin_blob.root_bone).unwrap().clone(), 
                bones: skin_blob.bone_names.into_iter()
                    .map(|bone_name| nodes.get(&bone_name).unwrap().clone())
                    .collect(), 
                bindpose: skin_blob.bindposes, 
            }
        });

        let mesh_renderer = builder.build(device, queue, skinning);
        if let MeshRenderer::SkinnedMesh(skinned_mesh) = &mesh_renderer {
            skinned_meshes.insert(mesh_name, skinned_mesh.clone());
        }

        object.add_element(mesh_renderer);
    }

    // 노드에 재질 데이터가 존재하는 경우 재질 요소를 추가합니다.
    if !blob.materials.is_empty() {
        let mut materials = Vec::with_capacity(blob.materials.len());
        for material_blob in blob.materials {
            materials.push(build_material(device, queue, material_blob));
        }

        object.add_element(materials);
    }

    nodes.insert(blob.name.clone(), object.clone());
    object
}

fn get_mesh_builder(blob: MeshBlob) -> MeshBuilder {
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
        builder = builder.with_submesh(Indices(submesh))
    }

    builder
}

fn build_material(
    device: &Arc<wgpu::Device>,
    queue: &Arc<wgpu::Queue>,
    blob: MaterialBlob, 
) -> Material {
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

    if let Some(blob) = blob.diffuse_map {
        let (texture_view, sampler) = build_texture_and_sampler(device, queue, blob);
        builder.diffuse_map = texture_view;
        builder.diffuse_sampler = sampler;
    }

    if let Some(blob) = blob.specular_map {
        let (texture_view, sampler) = build_texture_and_sampler(device, queue, blob);
        builder.specular_map = texture_view;
        builder.specular_sampler = sampler;
    }

    if let Some(blob) = blob.normal_map {
        let (texture_view, sampler) = build_texture_and_sampler(device, queue, blob);
        builder.normal_map = texture_view;
        builder.normal_sampler = sampler;
    }

    if let Some(blob) = blob.emissive_map {
        let (texture_view, sampler) = build_texture_and_sampler(device, queue, blob);
        builder.emissive_map = texture_view;
        builder.emissive_sampler = sampler;
    }

    builder.build(device, queue)
}

fn build_texture_and_sampler(
    device: &Arc<wgpu::Device>, 
    queue: &Arc<wgpu::Queue>,
    blob: TextureBlob
) -> (Arc<wgpu::TextureView>, Arc<wgpu::Sampler>) {
    let texture = match TexturePool::get(&blob.name) {
        Some(texture) => texture, 
        None => {
            let path = format!("characters/aris_original/{}.dds", &blob.name);
            let embeded_file = EmbededAssets::get(&path).unwrap();
            let dds = ddsfile::Dds::read(Cursor::new(embeded_file.data)).unwrap();
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

fn build_animations(
    skinned_meshes: &HashMap<String, Arc<SkinnedMesh>>,
    blob: AnimationBlob
) -> AnimationClip {
    AnimationClip::new(
        blob.name, 
        blob.length, 
        blob.frame_rate, 
        blob.keyframes.into_iter()
            .map(|blob| KeyFrame::new(
                blob.time_point, 
                blob.meshes.into_iter()
                    .map(|blob| Skinning {
                        skinned_mesh: skinned_meshes.get(&blob.mesh_name).unwrap().clone(), 
                        transforms: blob.bone_transforms.into_iter()
                            .map(|transform| gmm::Matrix::from_scale_rotation_translation(
                                transform.scale.into(), 
                                transform.rotation.into(), 
                                transform.translation.into()
                            ).into())
                            .collect()
                    })
            ))
    )
}
