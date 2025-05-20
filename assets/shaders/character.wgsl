//! 일반 캐릭터를 그리는 쉐이더 코드를 관리합니다.
//! 

// 최대 조명의 개수입니다.
const max_lights: u32 = 32u;

// 최대 뼈 노드의 개수입니다.
const max_bones: u32 = 256u;

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) bone_index: vec4<u32>, 
    @location(4) bone_weight: vec4<f32>,
};

// 버텍스 쉐이더 출력 데이터입니다.
// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position_w: vec3<f32>,
    @location(1) normal_w: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
};

// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>,
};

// 카메라 데이터 유니폼 버퍼입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
};

// 캐릭터 재질 데이터 유니폼 버퍼입니다.
struct CharacterMaterialDataLayout {
    glossiness: f32,
    smoothness: f32,
    metallic: f32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_bindposes: array<mat4x4<f32>, max_bones>;

@group(1) @binding(1)
var<uniform> u_bone_trans: array<mat4x4<f32>, max_bones>;

@group(2) @binding(0)
var<uniform> u_material: CharacterMaterialDataLayout;

@group(2) @binding(1)
var t_main_color: texture_2d<f32>;

@group(2) @binding(2)
var s_main_color: sampler; 

// 캐릭터를 그리는 버텍스 쉐이더입니다.
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;

    let bone_transform_0 = u_bone_trans[input.bone_index[0]] * u_bindposes[input.bone_index[0]];
    let bone_transform_1 = u_bone_trans[input.bone_index[1]] * u_bindposes[input.bone_index[1]];
    let bone_transform_2 = u_bone_trans[input.bone_index[2]] * u_bindposes[input.bone_index[2]];
    let bone_transform_3 = u_bone_trans[input.bone_index[3]] * u_bindposes[input.bone_index[3]];

    let final_matrix = input.bone_weight[0] * bone_transform_0 +
                   input.bone_weight[1] * bone_transform_1 +
                   input.bone_weight[2] * bone_transform_2 +
                   input.bone_weight[3] * bone_transform_3;
    
    let position_w = (final_matrix * vec4<f32>(input.position, 1.0)).xyz;
    let normal_w = input.bone_weight[0] * (bone_transform_0 * vec4<f32>(input.normal, 0.0)).xyz +
                    input.bone_weight[1] * (bone_transform_1 * vec4<f32>(input.normal, 0.0)).xyz +
                    input.bone_weight[2] * (bone_transform_2 * vec4<f32>(input.normal, 0.0)).xyz +
                    input.bone_weight[3] * (bone_transform_3 * vec4<f32>(input.normal, 0.0)).xyz;

    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.position_w = position_w;
    out.normal_w = normalize(normal_w);
    out.texcoord = input.texcoord;

    return out;
}

// 캐릭터를 그리는 프래그먼트 쉐이더입니다.
@fragment 
fn fs_main(input: VertexOutput) -> RenderTarget {
    let color = textureSample(t_main_color, s_main_color, input.texcoord);
    var out: RenderTarget;
    out.color = vec4(pow(color.rgb, vec3(1.0 / 2.2)), color.a); // 감마 보정
    return out;
}
