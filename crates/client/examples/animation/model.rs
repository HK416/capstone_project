use std::io::Cursor;
use std::sync::Arc;

use client_framework::physics::BoundingBox;
use client_framework::render::material::Material;
use client_framework::render::material::MaterialBuilder;
use client_framework::render::material::SamplerPool;
use client_framework::render::material::TexturePool;
use client_framework::render::mesh::Indices;
use client_framework::render::mesh::MeshBuilder;
use client_framework::render::mesh::VertexAttributeValues;
use client_framework::render::object::GameObject;
use client_framework::render::object::Transform;
use client_framework::render::object::WorldTransform;
use hecs::Entity;
use hecs::World;
use rust_embed::Embed;
use serde::Deserialize;
use serde::Serialize;



/// 사용할 에셋 목록입니다.
#[derive(Embed)]
#[folder = "examples/assets/Aris_Original"]
struct AssetBundle;



/// 모델 데이터 노드입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelNode {
    name: String, 
    transform: gmm::Float4x4, 
    mesh: Option<MeshNode>, 
    skinning: Option<SkinningNode>, 
    materials: Vec<MaterialNode>, 
    children: Vec<ModelNode>, 
}

/// 메쉬 데이터 노드입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeshNode {
    name: String, 
    num_vertices: u32, 
    vertices: Vec<gmm::Float3>, 
    colors: Vec<gmm::Float4>, 
    normals: Vec<gmm::Float3>, 
    tangents: Vec<gmm::Float3>, 
    texcoords0: Vec<gmm::Float2>, 
    texcoords1: Vec<gmm::Float2>, 
    submeshes: Vec<Vec<u32>>, 
    bounds: BoundingBox, 
}

/// 스키닝 데이터 노드입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkinningNode {
    bones_per_vertex: u32, 
    bone_names: Vec<String>, 
    bone_offsets: Vec<gmm::Float4x4>, 
    bone_indices: Vec<gmm::UInteger4>, 
    bone_weights: Vec<gmm::Float4>, 
}

/// 재질 데이터 노드입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MaterialNode {
    name: String, 
    glossiness: Option<f32>, 
    smoothness: Option<f32>, 
    metallic: Option<f32>, 
    diffuse: Option<gmm::Float4>, 
    specular: Option<gmm::Float4>, 
    emissive: Option<gmm::Float4>, 
    diffuse_map: Option<TextureNode>, 
    specular_map: Option<TextureNode>, 
    normal_map: Option<TextureNode>, 
    emissive_map: Option<TextureNode>, 
}

/// 텍스처 데이터 노드입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TextureNode {
    name: String, 
    view_dimension: TextureDimension, 
    filter_mode: FilterMode, 
    address_u: AddressMode, 
    address_v: AddressMode, 
    address_w: AddressMode, 
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum TextureDimension {
    Auto, 
    D1, 
    D2, 
    D2Array, 
    Cube, 
    CubeArray, 
    D3, 
}

impl Into<Option<wgpu::TextureViewDimension>> for TextureDimension {
    #[inline]
    fn into(self) -> Option<wgpu::TextureViewDimension> {
        match self {
            Self::Auto => None, 
            Self::D1 => Some(wgpu::TextureViewDimension::D1), 
            Self::D2 => Some(wgpu::TextureViewDimension::D2), 
            Self::D2Array => Some(wgpu::TextureViewDimension::D2Array), 
            Self::Cube => Some(wgpu::TextureViewDimension::Cube), 
            Self::CubeArray => Some(wgpu::TextureViewDimension::CubeArray), 
            Self::D3 => Some(wgpu::TextureViewDimension::D3),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum FilterMode {
    Nearest, 
    Linear, 
}

impl Into<wgpu::FilterMode> for FilterMode {
    #[inline]
    fn into(self) -> wgpu::FilterMode {
        match self {
            FilterMode::Linear => wgpu::FilterMode::Linear, 
            FilterMode::Nearest => wgpu::FilterMode::Nearest, 
        }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum AddressMode {
    ClampToEdge, 
    MirrorRepeat, 
    Repeat, 
}

impl Into<wgpu::AddressMode> for AddressMode {
    #[inline]
    fn into(self) -> wgpu::AddressMode {
        match self {
            AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge, 
            AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat, 
            AddressMode::Repeat => wgpu::AddressMode::Repeat, 
        }
    }
}



/// 모델 에셋의 루트 모델 노드를 반환합니다.
#[must_use]
fn decode_model<P: AsRef<str>>(file_path: P) -> ModelNode {
    let embeded_file = AssetBundle::get(file_path.as_ref()).unwrap();
    ron::de::from_bytes(&embeded_file.data).unwrap()
}

/// 3차원 메쉬를 생성합니다.
#[must_use]
fn create_mesh_builder(node: MeshNode) -> MeshBuilder {
    // 3차원 메쉬 빌더를 생성합니다.
    let mut builder = MeshBuilder::new(node.name, node.vertices);

    // 색상 속성을 추가합니다.
    if !node.colors.is_empty() {
        builder = builder.insert_attribute(VertexAttributeValues::Colors(node.colors));
    }

    // 노멀 속성을 추가합니다.
    if !node.normals.is_empty() {
        builder = builder.insert_attribute(VertexAttributeValues::Normals(node.normals));
    }

    // 탄젠트 속성을 추가합니다.
    if !node.tangents.is_empty() {
        builder = builder.insert_attribute(VertexAttributeValues::Tangents(node.tangents));
    }

    // 0번 텍스처 좌표 속성을 추가합니다.
    if !node.texcoords0.is_empty() {
        builder = builder.insert_attribute(VertexAttributeValues::Texcoords0(node.texcoords0));
    }

    // 1번 텍스처 좌표 속성을 추가합니다.
    if !node.texcoords1.is_empty() {
        builder = builder.insert_attribute(VertexAttributeValues::Texcoords1(node.texcoords1));
    }

    // 하위 메쉬들을 추가합니다.
    for values in node.submeshes {
        builder = builder.add_submesh(Indices(values));
    }

    return builder;
}

/// 스키닝 데이터를 추가합니다.
#[allow(unused_variables)]
fn add_skinning(node: SkinningNode, mut builder: MeshBuilder) -> MeshBuilder {
    if !node.bone_indices.is_empty() {
        builder = builder.insert_attribute(VertexAttributeValues::BoneIndices(node.bone_indices));
    }

    if !node.bone_weights.is_empty() {
        builder = builder.insert_attribute(VertexAttributeValues::BoneWeights(node.bone_weights));
    }

    return builder;
}

/// 재질 데이터를 추가합니다.
fn create_material(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    node: MaterialNode
) -> Material {
    // 재질 빌더를 생성합니다.
    let mut builder = MaterialBuilder::new(
        Some(&node.name), 
        device, 
        queue
    );

    // `Diffuse` 색상을 추가합니다.
    if let Some(diffuse) = node.diffuse {
        builder.diffuse = diffuse;
    }

    // `Specular` 색상을 추가합니다. 
    if let Some(specular) = node.specular {
        builder.specular = specular;
    }

    // `Emissive` 색상을 추가합니다.
    if let Some(emissive) = node.emissive {
        builder.emissive = emissive;
    }

    // `Diffuse` 텍스처를 추가합니다.
    if let Some(texture_node) = node.diffuse_map {
        let (texture_view, sampler) = create_texture(device, queue, texture_node);
        builder.diffuse_map = texture_view;
        builder.diffuse_sampler = sampler;
    }

    // `Normal` 텍스처를 추가합니다.
    if let Some(texture_node) = node.normal_map {
        let (texture_view, sampler) = create_texture(device, queue, texture_node);
        builder.normal_map = texture_view;
        builder.normal_sampler = sampler;
    }

    // `Specular` 텍스처를 추가합니다.
    if let Some(texture_node) = node.specular_map {
        let (texture_view, sampler) = create_texture(device, queue, texture_node);
        builder.specular_map = texture_view;
        builder.specular_sampler = sampler;
    }

    // `Emissive` 텍스처를 추가합니다.
    if let Some(texture_node) = node.emissive_map {
        let (texture_view, sampler) = create_texture(device, queue, texture_node);
        builder.emissive_map = texture_view;
        builder.emissive_sampler = sampler;
    }

    return builder.build(device);
}

/// 텍스처를 생성합니다.
fn create_texture(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    node: TextureNode
) -> (Arc<wgpu::TextureView>, Arc<wgpu::Sampler>) {
    let file_path = node.name.clone() + ".dds";
    let texture_view = match TexturePool::get(file_path.as_str()) {
        Some(texture) => texture, 
        None => {
            // 텍스처 에셋 파일을 가져옵니다.
            let embeded_file = AssetBundle::get(&file_path).unwrap();
            let bytes = Cursor::new(embeded_file.data);
            let dds = ddsfile::Dds::read(bytes).unwrap();

            TexturePool::spawn_with_data(
                device, 
                queue, 
                file_path.as_str(), 
                &wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", &node.name)), 
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
            )
        }
    }.get_view_or_init(&wgpu::TextureViewDescriptor {
        dimension: node.view_dimension.into(),
        ..Default::default()
    });

    let sampler = SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor {
        label: None, 
        address_mode_u: node.address_u.into(), 
        address_mode_v: node.address_v.into(), 
        address_mode_w: node.address_w.into(), 
        mag_filter: node.filter_mode.into(), 
        min_filter: node.filter_mode.into(), 
        mipmap_filter: node.filter_mode.into(), 
        ..Default::default()
    });

    return (texture_view, sampler);
}


/// 게임 오브젝트를 생성합니다.
fn spawn_game_object(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    world: &mut World, 
    parent: Option<Entity>, 
    parent_transform: gmm::Matrix, 
    node: ModelNode, 
) -> Entity {
    // 비어있는 엔티티를 생성합니다.
    let entity = world.spawn(());


    // 월드 변환 행렬과 로컬 변환 행렬을 생성하고 추가합니다.
    let mut trans = Transform::new();
    (*trans) = node.transform.into();
    let mut world_trans = WorldTransform::new();
    (*world_trans) = parent_transform * (*trans);
    world.insert(entity, (world_trans, trans)).unwrap();


    // 모델에 연결된 3차원 메쉬를 생성하고 추가합니다.
    if let Some(mesh_node) = node.mesh {
        let mut builder = create_mesh_builder(mesh_node);

        // 모델에 연결된 스키닝 데이터를 생성하고 추가합니다.
        if let Some(skinning_node) = node.skinning {
            builder = add_skinning(skinning_node, builder);
            world.insert(entity, (builder.build(device, queue), )).unwrap();
        } else {
            world.insert_one(entity, builder.build(device, queue)).unwrap();
        }
    }


    // 모델에 연결된 재질을 생성하고 추가합니다.
    let mut materials = Vec::with_capacity(node.materials.len());
    for material_node in node.materials {
        materials.push(create_material(device, queue, material_node));
    }
    world.insert_one(entity, materials).unwrap();


    // 계층 구조를 추가합니다.
    let mut object = GameObject::new(Some(&node.name), device);
    object.parent = parent.unwrap_or(Entity::DANGLING);
    object.children.reserve(node.children.len());
    for child in node.children {
        let entity = spawn_game_object(device, queue, world, Some(entity), *world_trans, child);
        object.children.push(entity);
    }
    world.insert_one(entity, object).unwrap();


    return entity;
}

/// 모델을 생성합니다.
pub fn spawn_model<P: AsRef<str>>(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    world: &mut World, 
    file_path: P, 
) -> Entity {
    let node = decode_model(file_path);
    spawn_game_object(device, queue, world, None, gmm::Float4x4::IDENTITY.into(), node)
}
