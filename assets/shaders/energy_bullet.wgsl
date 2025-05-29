//! 에너지 볼 형태의 총알을 그리는 쉐이더 코드를 관리합니다.
//!

// 최대 조명의 개수입니다.
const max_lights: u32 = 8u;

// 깊이 bias입니다.
const bias: f32 = 0.001;

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) texcoord: vec2<f32>,
};

// 버텍스 쉐이더 출력 데이터입니다.
// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
};

// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    // 누적 값을 저장하는 렌더 타겟 텍스처
    @location(0) accum: vec4<f32>,
    // 노출 값을 저장하는 렌더 타겟 텍스처
    @location(1) reveal: f32,
    // 발광체 색상을 저장하는 렌더 타겟 텍스처
    @location(2) bloom: vec4<f32>,
};

// 카메라 데이터 유니폼 버퍼
struct CameraDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
};

// 총알 재질 데이터 유니폼 버퍼
struct EnergyBulletMaterialDataLayout {
    emissive: vec4<f32>,
    main_color: vec4<f32>,
};

// 전역 조명 데이터 유니폼 버퍼입니다.
struct GlobalLightDataLayout {
    static_proj_view: mat4x4<f32>,
    proj_view: mat4x4<f32>,
    direction_w: vec3<f32>,
    color: vec3<f32>,
};

// 지역 조명 데이터 유니폼 버퍼입니다.
struct LocalLightDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
    constant: f32,
    color: vec3<f32>,
    linear: f32,
    quadratic: f32,
};

// 지역 조명 데이터 집합 유니폼 버퍼입니다.
struct LocalLightSetDataLayout {
    lights: array<LocalLightDataLayout, max_lights>,
    num_lights: u32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> u_material: EnergyBulletMaterialDataLayout;

@group(3) @binding(0)
var<uniform> u_global_light: GlobalLightDataLayout;

@group(3) @binding(1)
var<uniform> u_local_lights: LocalLightSetDataLayout;

@group(3) @binding(2)
var t_static_light: texture_2d<f32>;

@group(3) @binding(3)
var s_static_light: sampler;

@group(3) @binding(4)
var t_global_light: texture_depth_2d;

@group(3) @binding(5)
var t_local_lights: texture_depth_2d_array;

@group(3) @binding(6)
var s_lights: sampler_comparison;

// 총알을 그리는 버텍스 쉐이더입니다.
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;

    let position_w = (u_trans * vec4<f32>(input.position, 1.0)).xyz;

    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.texcoord = input.texcoord;

    return out;
}

// 총알을 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    let color = u_material.main_color;
    let alpha = color.a;

    let depth = input.clip_position.z;
    let weight = calculate_weight(depth, alpha);

    var out: RenderTarget;
    out.accum = vec4<f32>(color.rgb * alpha, alpha) * weight;
    out.reveal = alpha;
    out.bloom = u_material.emissive;
    return out;
}

// Weighted Blended Order Independent Transparency의 가중치를 구합니다.
fn calculate_weight(z: f32, a: f32) -> f32 {
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 100.0 / (1e-5 + pow(abs(z) / 10.0, 3.0) + pow(abs(z) / 200.0, 6.0))));
}
