/// 뼈의 최대 개수입니다.
const MAX_BONES = 256;

/// 지역 조명의 최대 개수입니다.
const MAX_LOCAL_LIGHTS = 32;



/// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>, 
    @location(1) normal: vec3<f32>, 
    @location(2) tangent: vec3<f32>, 
    @location(3) texcoord: vec2<f32>, 
    @location(4) bone_index: vec4<u32>, 
    @location(5) bone_weight: vec4<f32>, 
};

/// 버텍스 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, 
    @location(0) position_w: vec3<f32>, 
    @location(1) texcoord: vec2<f32>, 
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>, 
};



/// 카메라 데이터 레이아웃입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>, 
    position_w: vec3<f32>, 
    direction_w: vec3<f32>, 
};

/// 전역 조명 데이터 레이아웃입니다.
struct GlobalLightDataLayout {
    proj_view: mat4x4<f32>, 
    direction_w: vec3<f32>,
    color: vec3<f32>, 
};

/// 지역 조명 데이터 레이아웃입니다.
struct LocalLightDataLayout {
    color: vec4<f32>, 
    position_w: vec3<f32>, 
    range_w: f32, 
    direction_w: vec3<f32>, 
    angle: f32, 
};

/// 지역 조명 집합 레이아웃입니다.
struct LocalLightSetLayout {
    lights: array<LocalLightDataLayout, MAX_LOCAL_LIGHTS>, 
    num_lights: u32, 
};

/// 스키닝된 메쉬의 데이터 레이아웃입니다.
struct SkinningDataLayout {
    quality: u32, 
    num_bones: u32, 
}

/// 재질 데이터 레이아웃입니다.
struct MaterialDataLayout {
    glossiness: f32, 
    smoothness: f32, 
    metallic: f32, 
    bump_scale: f32, 
    parallax: f32,
    strength: f32,
};



@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(0) @binding(1)
var<uniform> u_global_light: GlobalLightDataLayout;

// @group(0) @binding(2)
// var<uniform> u_local_lights: LocalLightSetLayout;


@group(1) @binding(0)
var<uniform> u_skinning: SkinningDataLayout;

@group(1) @binding(1)
var<uniform> u_bindposes: array<mat4x4<f32>, MAX_BONES>;

@group(1) @binding(2)
var<uniform> u_bone_trans: array<mat4x4<f32>, MAX_BONES>;


@group(2) @binding(0)
var<uniform> u_material: MaterialDataLayout;

@group(2) @binding(1)
var t_albedo: texture_2d<f32>;

@group(2) @binding(2)
var s_albedo: sampler;

// @group(2) @binding(3)
// var t_specular: texture_2d<f32>;

// @group(2) @binding(4)
// var s_specular: sampler;

// @group(2) @binding(5)
// var t_emissive: texture_2d<f32>;

// @group(2) @binding(6)
// var s_emissive: sampler;

// @group(2) @binding(7)
// var t_normal: texture_2d<f32>;

// @group(2) @binding(8)
// var s_normal: sampler;

// @group(2) @binding(9)
// var t_parallax: texture_2d<f32>;

// @group(2) @binding(10)
// var s_parallax: sampler;

// @group(2) @binding(11)
// var t_occlusion: texture_2d<f32>;

// @group(2) @binding(12)
// var s_occlusion: sampler;


/// 버텍스 쉐이더
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
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.position_w = position_w;
    out.texcoord = input.texcoord;

    return out;
}

/// 그림자를 생성할 때 사용되는 버텍스 쉐이더
@vertex
fn vs_bake(
    @location(0) position: vec3<f32>,
    @location(1) bone_index: vec4<u32>, 
    @location(2) bone_weight: vec4<f32>, 
) -> @builtin(position) vec4<f32> {
    let bone_transform_0 = u_bone_trans[bone_index[0]] * u_bindposes[bone_index[0]];
    let bone_transform_1 = u_bone_trans[bone_index[1]] * u_bindposes[bone_index[1]];
    let bone_transform_2 = u_bone_trans[bone_index[2]] * u_bindposes[bone_index[2]];
    let bone_transform_3 = u_bone_trans[bone_index[3]] * u_bindposes[bone_index[3]];

    let final_matrix = bone_weight[0] * bone_transform_0 +
                   bone_weight[1] * bone_transform_1 +
                   bone_weight[2] * bone_transform_2 +
                   bone_weight[3] * bone_transform_3;

    let position_w = (final_matrix * vec4<f32>(position, 1.0)).xyz;
    return u_global_light.proj_view * vec4<f32>(position_w, 1.0);
}



/// 프래그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    out.color = textureSample(t_albedo, s_albedo, input.texcoord);
    return out;
}
