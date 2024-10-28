/// 뼈의 최대 개수입니다.
const MAX_BONES = 256;

/// 지역 조명의 최대 개수입니다.
const MAX_LIGHTS = 16;



/// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>, 
    @location(1) normal: vec3<f32>, 
    @location(2) tangent: vec3<f32>, 
    @location(3) texcoord: vec2<f32>, 
    @location(4) bone_index: vec4<u32>, 
    @location(5) bone_weight: vec4<f32> 
};

/// 버텍스 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, 
    @location(0) position_w: vec3<f32>, 
    @location(1) texcoord: vec2<f32> 
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32> 
};



/// 카메라 데이터 레이아웃입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>, 
    position_w: vec3<f32>, 
    direction_w: vec3<f32> 
};

/// 전역 조명 데이터 레이아웃입니다.
struct GlobalLightDataLayout {
    color: vec4<f32>, 
    direction_w: vec3<f32> 
};

/// 지역 조명 데이터 레이아웃입니다.
struct LocalLightDataLayout {
    color: vec4<f32>, 
    position_w: vec3<f32>, 
    range_w: f32, 
    direction_w: vec3<f32>, 
    angle: f32 
};

/// 지역 조명 집합 레이아웃입니다.
struct LocalLightSetLayout {
    lights: array<LocalLightDataLayout, MAX_LIGHTS>, 
    num_lights: u32 
};

/// 동적 메쉬 데이터 레이아웃입니다.
struct DynamicMeshDataLayout {
    quality: u32, 
    num_bones: u32 
};

/// `Universal` 재질 데이터 레이아웃입니다.
struct UniversalMaterialDataLayout {
    glossiness: f32, 
    smoothness: f32, 
    metallic: f32, 
    height: f32, 
    albedo: vec4<f32>, 
    specular: vec4<f32>, 
    emissive: vec4<f32> 
};



@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

// @group(0) @binding(1)
// var<uniform> u_global_light: GlobalLightDataLayout;

// @group(0) @binding(2)
// var<uniform> u_local_lights: LocalLightSetLayout;


@group(1) @binding(0)
var<uniform> u_model: DynamicMeshDataLayout;

@group(1) @binding(1)
var<uniform> u_bindposes: array<mat4x4<f32>, MAX_BONES>;

@group(1) @binding(2)
var<uniform> u_bone_trans: array<mat4x4<f32>, MAX_BONES>;


@group(2) @binding(0)
var<uniform> u_material: UniversalMaterialDataLayout;

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
// var t_height: texture_2d<f32>;

// @group(2) @binding(10)
// var s_height: sampler;


/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;
    var final_matrix = mat4x4<f32>(
        0.0, 0.0, 0.0, 0.0, 
        0.0, 0.0, 0.0, 0.0, 
        0.0, 0.0, 0.0, 0.0, 
        0.0, 0.0, 0.0, 0.0
    );

    for (var i = 0u; i < u_model.quality; i++) {
        let index = input.bone_index[i];
        let weight = input.bone_weight[i];
        let bone_transform = u_bone_trans[index] * u_bindposes[index];
        final_matrix += weight * bone_transform;
    }

    let position_w = (final_matrix * vec4<f32>(input.position, 1.0)).xyz;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.position_w = position_w;
    out.texcoord = input.texcoord;

    return out;
}



/// 프래그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;

    out.color = u_material.albedo * textureSample(t_albedo, s_albedo, input.texcoord);
    
    return out;
}
