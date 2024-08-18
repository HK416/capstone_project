/// 뼈의 최대 갯수입니다.
const MAX_BONES = 256;

/// 정점의 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>, 
    @location(1) texcoord: vec2<f32>, 
    @location(2) bone_index: vec4<u32>, 
    @location(3) bone_weight: vec4<f32>, 
};

/// vertex 쉐이더 출력 데이터입니다.
/// fragment 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, 
    @location(0) texcoord: vec2<f32>, 
};

/// fragment 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>, 
};

/// 카메라 오브젝트 데이터 레이아웃입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>, 
    position: vec3<f32>, 
    direction: vec3<f32>
};

/// 재질 데이터 레이아웃입니다.
struct MaterialDataLayout {
    diffuse: vec4<f32>, 
    specular: vec4<f32>, 
    emissive: vec4<f32>, 
};

/// 뼈 데이터 레이아웃입니다.
struct BoneDataLayout {
    bones_per_vertex: u32, 
};



@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_bone: BoneDataLayout;

@group(1) @binding(1)
var<uniform> u_bone_trans: array<mat4x4<f32>, MAX_BONES>;

@group(1) @binding(2)
var<uniform> u_bind_matrices: array<mat4x4<f32>, MAX_BONES>;

@group(2) @binding(0)
var<uniform> u_material: MaterialDataLayout;

@group(2) @binding(1)
var t_diffuse: texture_2d<f32>;

@group(2) @binding(2)
var s_diffuse: sampler;



/// 스키닝된 메쉬의 버텍스 쉐이더입니다.
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;
    
    var final_matrix = mat4x4<f32>(vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0));
    for (var i = 0u; i < u_bone.bones_per_vertex; i++) {
        let index = input.bone_index[i];
        let weight = input.bone_weight[i];
        let bone_transform = u_bone_trans[index] * u_bind_matrices[index];
        final_matrix += weight * bone_transform;
    }

    let position_w = (final_matrix * vec4<f32>(input.position, 1.0)).xyz;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.texcoord = input.texcoord;

    return out;
}

/// 프레그먼트(픽셀) 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    out.color = textureSample(t_diffuse, s_diffuse, input.texcoord);
    return out;
}
