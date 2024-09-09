use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use hecs::Component;
use hecs::Entity; 
use hecs::World;
use mod_asset::node::AnimationNode;
use mod_asset::node::MaterialNode;
use mod_asset::node::MeshNode;
use mod_asset::node::ModelNode;
use mod_asset::node::RootModelNode;
use mod_asset::node::SkinNode;
use mod_asset::node::TextureNode;
use mod_render::anim::Animation;
use mod_render::anim::Bone;
use mod_render::anim::BoneTransform;
use mod_render::anim::KeyFrame;
use mod_render::material::MaterialBuilder;
use mod_render::material::MaterialComponent;
use mod_render::material::SamplerPool;
use mod_render::material::TexturePool;
use mod_render::mesh::Indices;
use mod_render::mesh::Mesh;
use mod_render::mesh::MeshBuilder;
use mod_render::mesh::MeshComponent;
use mod_render::mesh::VertexAttributeValues;
use mod_render::object::GameObject;
use mod_render::object::GameObjectDataLayout;
use mod_render::object::Transform;
use mod_render::object::WorldTransform;
use mod_render::skin::BoneDataLayout;
use mod_render::skin::Skin;
use mod_render::skin::SkinComponent;
use rust_embed::Embed;



/// 사용할 임베딩된 에셋 데이터입니다.
#[derive(Embed)]
#[folder = "examples/assets/Aris_Original"]
struct AssetBundle;

/// 모델 에셋을 디코드합니다.
#[must_use]
fn decode_model<T: AsRef<str>>(filepath: T) -> RootModelNode {
    let embeded_file = AssetBundle::get(filepath.as_ref()).unwrap();
    ron::de::from_bytes(&embeded_file.data).unwrap()
}

/// 에셋으로부터 모델을 생성합니다.
#[must_use]
pub fn spawn_model_from_asset<T: AsRef<str>>(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    world: &mut World, 
    shader: impl Component + Copy, 
    filepath: T
) -> (Entity, Vec<Animation>) {
    let root_model = decode_model(filepath);
    let mut objects = HashMap::new();
    let mut skinned_meshes = HashMap::new();
    let entity = spawn_game_object(
        device, 
        queue, 
        world, 
        shader, 
        &mut objects, 
        &mut skinned_meshes, 
        Entity::DANGLING, 
        WorldTransform::default(), 
        Some(root_model.root), 
        Vec::new()
    );

    let animations = root_model.animations.into_iter()
        .map(|node| {
            create_animation(node, &skinned_meshes)
        })
        .collect();

    return (entity, animations);
}

/// 게임 오브젝트를 생성합니다.
#[must_use]
fn spawn_game_object(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    world: &mut World, 
    shader: impl Component + Copy,
    objects: &mut HashMap<String, Entity>, 
    skinned_meshes: &mut HashMap<String, Arc<Skin>>, 
    parent: Entity, 
    parent_transform: WorldTransform, 
    current: Option<ModelNode>, 
    mut sibling: Vec<ModelNode>
) -> Entity {
    // 현재 모델 노드를 가져옵니다.
    let mut current = match current {
        Some(model_node) => model_node, 
        None => return Entity::DANGLING, 
    };

    // 비어있는 엔티티를 생성합니다.
    let entity = world.spawn(());
    objects.insert(current.name.clone(), entity);
    
    // 변환 행렬을 생성하고 컴포넌트를 추가합니다.
    let transform: Transform = current.transform.into();
    let world_transform: WorldTransform = ((*parent_transform) * (*transform)).into();
    world.insert(entity, (transform, world_transform)).unwrap();

    // 게임 오브젝트 컴포넌트를 생성합니다.
    let game_object = GameObject::new(Some(&format!("GameObject({})", &current.name)), device);

    // 게임 오브젝트를 갱신합니다.
    game_object.update(queue, GameObjectDataLayout {transform: world_transform.into() });
    
    // 부모 엔티티를 설정합니다.
    game_object.set_parent(parent);

    // 자식 엔티티를 설정합니다.
    game_object.set_child(spawn_game_object(
        device, 
        queue, 
        world, 
        shader, 
        objects, 
        skinned_meshes, 
        entity, 
        world_transform, 
        current.children.pop(), 
        current.children
    ));

    // 형제 엔티티를 설정합니다.
    game_object.set_sibling(spawn_game_object(
        device, 
        queue, 
        world, 
        shader, 
        objects, 
        skinned_meshes, 
        parent, 
        parent_transform, 
        sibling.pop(), 
        sibling
    ));


    // 게임 오브젝트 컴포넌트를 추가합니다.
    world.insert_one(entity, game_object).unwrap();

    // 모델 노드에 연결된 3차원 메쉬를 생성하고 추가합니다.
    if let Some(mesh_node) = current.mesh {
        let mesh_name = mesh_node.name.clone();
        let mesh = create_mesh(device, queue, mesh_node);
        world.insert(entity, (mesh.clone(), shader)).unwrap();

        // 스키닝된 메쉬의 경우 스키닝 데이터를 추가합니다.
        if let Some(skin_node) = current.skin {
            let skin = create_skin(device, queue, mesh, skin_node, objects);
            world.insert_one(entity, skin.clone()).unwrap();
            
            // 스키닝 메쉬 목록에 추가합니다.
            skinned_meshes.insert(mesh_name, skin);
        }
    }

    // 모델 노드에 연결된 재질을 생성하고 추가합니다.
    let mut materials = Vec::with_capacity(current.materials.len());
    for material_node in current.materials {
        materials.push(create_material(device, queue, material_node));
    }
    world.insert_one(entity, materials).unwrap();

    // 엔티티를 반환합니다.
    return entity;
}

/// 3차원 메쉬를 생성합니다.
#[must_use]
fn create_mesh(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    node: MeshNode
) -> MeshComponent {
    // 3차원 메쉬 빌더를 생성합니다.
    let mut builder = MeshBuilder::new(
        node.name, 
        node.vertices
    );

    // 색상 속성을 추가합니다.
    if !node.colors.is_empty() {
        builder = builder.insert_attribute(
            VertexAttributeValues::Colors(node.colors)
        );
    }

    // 노멀 속성을 추가합니다.
    if !node.normals.is_empty() {
        builder = builder.insert_attribute(
            VertexAttributeValues::Normals(node.normals)
        );
    }

    // 탄젠트 속성을 추가합니다.
    if !node.tangents.is_empty() {
        builder = builder.insert_attribute(
            VertexAttributeValues::Tangents(node.tangents)
        );
    }

    // 0번 텍스처 좌표 속성을 추가합니다.
    if !node.texcoords0.is_empty() {
        builder = builder.insert_attribute(
            VertexAttributeValues::Texcoords0(node.texcoords0)
        );
    }

    // 1번 텍스처 좌표 속성을 추가합니다.
    if !node.texcoords1.is_empty() {
        builder = builder.insert_attribute(
            VertexAttributeValues::Texcoords1(node.texcoords1)
        );
    }

    // 뼈 인덱스 속성을 추가합니다.
    if !node.bone_indices.is_empty() {
        builder = builder.insert_attribute(
            VertexAttributeValues::BoneIndices(node.bone_indices)
        );
    }

    // 뼈 가중치 속성을 추가합니다.
    if !node.bone_weights.is_empty() {
        builder = builder.insert_attribute(
            VertexAttributeValues::BoneWeights(node.bone_weights)
        );
    }

    // 뼈의 초기 상태 위치 변환 행렬을 추가합니다.
    if !node.bindposes.is_empty() {
        builder = builder.set_bindposes(node.bindposes);
    }

    // 하위 메쉬들을 추가합니다.
    for values in node.submeshes {
        builder = builder.add_submesh(Indices(values));
    }

    builder.build(device, queue)
}

/// 스키닝 컴포넌트를 생성합니다.
#[must_use]
fn create_skin(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    mesh: Arc<Mesh>, 
    node: SkinNode, 
    objects: &mut HashMap<String, Entity>
) -> SkinComponent {
    let root_bone = objects.get(&node.root_bone).expect("Could not find root bone object!").clone();
    let bones = node.bone_names.into_iter().map(|name| {
        objects.get(&name).expect("Could not find bone object!").clone()
    });

    Skin::new(
        mesh, 
        root_bone, 
        bones, 
        device, 
        queue, 
        BoneDataLayout { 
            quality: node.quality, 
            ..Default::default() 
        }
    )
}

/// 재질 컴포넌트를 생성합니다.
#[must_use]
fn create_material(
    device: &wgpu::Device, 
    queue: &wgpu::Queue, 
    node: MaterialNode
) -> MaterialComponent {
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

    builder.build(device, queue)
}

/// 텍스처를 생성합니다.
#[must_use]
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

/// 애니메이션을 생성합니다.
#[must_use]
fn create_animation(
    node: AnimationNode, 
    skinned_meshes: &HashMap<String, Arc<Skin>>
) -> Animation {
    Animation::new(
        node.name, 
        node.length, 
        node.frame_rate, 
        node.keyframes.into_iter()
            .map(|node| KeyFrame::new(
                node.time_point, 
                node.meshes.into_iter()
                    .map(|node| {
                        Bone::new(
                            skinned_meshes.get(&node.mesh_name).unwrap().clone(), 
                            node.bone_transforms.into_iter()
                                .map(|node| BoneTransform {
                                    scale: node.scale, 
                                    rotation: node.rotation, 
                                    translation: node.translation
                                })
                        )
                    })
            ))
    )
}
