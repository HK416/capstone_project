/// 지역 조명의 최대 개수입니다.
const MAX_LOCAL_LIGHTS = 32;

/// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) texcoord: vec2<f32>,
};

/// 버텍스 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, 
    @location(0) position_w: vec3<f32>,
    @location(1) normal_w: vec3<f32>,
    @location(2) texcoord: vec2<f32>, 
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
var<uniform> u_trans: mat4x4<f32>;

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

@group(3) @binding(0)
var t_shadow: texture_depth_2d;

@group(3) @binding(1)
var s_shadow: sampler_comparison;

/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    let position_w = (u_trans * vec4<f32>(input.position, 1.0)).xyz;
    let normal_w = (u_trans * vec4<f32>(input.normal, 0.0)).xyz;

    var out: VertexOutput;
    out.position_w = position_w;
    out.normal_w = normal_w;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.texcoord = input.texcoord;
    return out;
}

/// 그림자를 생성할 때 사용되는 버텍스 쉐이더
@vertex
fn vs_bake(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    let position_w = (u_trans * vec4<f32>(position, 1.0)).xyz;
    return u_global_light.proj_view * vec4<f32>(position_w, 1.0);
}

/// 프래그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    // 텍스터 색상을 가져옵니다.
    var albedo = textureSample(t_albedo, s_albedo, input.texcoord);

    // 전역 조명 그림자를 계산합니다.
    var color = vec3<f32>(0.3);
    let shadow = calculate_shadow(u_global_light.proj_view * vec4<f32>(input.position_w, 1.0));
    let light_dir = -u_global_light.direction_w;
    color = min(color + shadow * u_global_light.color.xyz, vec3<f32>(1.0));

    var out: RenderTarget;
    out.color = albedo * vec4<f32>(color, 1.0);
    return out;
}

/// 그림자를 계산합니다.
fn calculate_shadow(light_space_position: vec4<f32>) -> f32 {
    if (light_space_position.w <= 0.0) {
        return 1.0;
    }
    
    let curr_depth = light_space_position.z / light_space_position.w;
    var proj_coords = light_space_position.xy / light_space_position.w;
    proj_coords = proj_coords * vec2<f32>(0.5, -0.5) + 0.5;

    return textureSampleCompareLevel(t_shadow, s_shadow, proj_coords, curr_depth);
}
