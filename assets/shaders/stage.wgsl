//! 지형을 그리는 쉐이더 코드를 관리합니다.
//!

// 분할하는 Cascade의 개수입니다.
const num_cascades: u32 = 4u;

// 최대 조명의 개수입니다.
const max_lights: u32 = 8u;

// 깊이 bias입니다.
const bias: f32 = 0.005;

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
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

// 지형 재질 데이터 유니폼 버퍼입니다.
struct StageMaterialDataLayout {
    glossiness: f32,
    smoothness: f32,
    metallic: f32,
};

// 지역 조명 데이터 유니폼 버퍼입니다.
struct localLightDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
    constant: f32,
    color: vec3<f32>,
    linear: f32,
    quadratic: f32,
};

// 조명 데이터 집합 유니폼 버퍼입니다.
struct LightSetDataLayout {
    direction_w: vec3<f32>,
    color: vec3<f32>,
    global_lights: array<mat4x4<f32>, num_cascades>,
    local_lights: array<localLightDataLayout, max_lights>,
    num_lights: u32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> u_material: StageMaterialDataLayout;

@group(2) @binding(1)
var t_main_color: texture_2d<f32>;

@group(2) @binding(2)
var s_main_color: sampler;

@group(3) @binding(0)
var<uniform> u_lights: LightSetDataLayout;

@group(3) @binding(1)
var t_global_lights: texture_depth_2d_array;

@group(3) @binding(2)
var t_local_lights: texture_depth_2d_array;

@group(3) @binding(3)
var s_lights: sampler_comparison;

// 지형을 그리는 버텍스 쉐이더입니다.
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;

    let position_w = (u_trans * vec4<f32>(input.position, 1.0)).xyz;
    let normal_w = (u_trans * vec4<f32>(input.normal, 0.0)).xyz;

    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.position_w = position_w;
    out.normal_w = normal_w;
    out.texcoord = input.texcoord;

    return out;
}

// 지형을 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    // 노멀을 계산합니다.
    let normal = normalize(input.normal_w);
    // 카메라로부터 거리를 계산합니다.
    let distance = distance(u_camera.position_w, input.position_w);

    // 메인 텍스처를 가져옵니다.
    let main_color = textureSample(t_main_color, s_main_color, input.texcoord).rgb;
    
    // 전역 조명의 그림자 색상을 계산합니다.
    var shadow = 1.0;
    var diffuse = max(0.0, dot(normal, normalize(-u_lights.direction_w)));
    var color = main_color * 0.75; // ambient 조명
    for (var i = 0u; i < num_cascades; i += 1u) {
        let proj_view = u_lights.global_lights[i];
        let light_space_position = proj_view * vec4<f32>(input.position_w, 1.0);
        shadow = min(shadow, calculate_global_shadow(i, light_space_position));
    }
    color += main_color * u_lights.color * diffuse * shadow;

    var out: RenderTarget;
    out.color = vec4(pow(color.rgb, vec3(1.0 / 2.2)), 1.0); // 감마 보정
    return out;
}


/// 전역 조명의 그림자를 계산합니다.
fn calculate_global_shadow(index: u32, light_space_position: vec4<f32>) -> f32 {
    if (light_space_position.w <= 0.0) {
        return 1.0;
    }
    
    let curr_depth = clamp(light_space_position.z / light_space_position.w - bias, 0.0, 1.0);
    var proj_coords = light_space_position.xy / light_space_position.w;
    proj_coords = proj_coords * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    
    // 그림자 맵 경계 확인
    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 || 
        proj_coords.y < 0.0 || proj_coords.y > 1.0) {
        return 1.0; // 그림자 맵 밖은 그림자 없음
    }
    
    return textureSampleCompare(t_global_lights, s_lights, proj_coords, i32(index), curr_depth);
}
