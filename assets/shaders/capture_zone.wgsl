//! 점령 지역을 그리는 쉐이더 코드를 관리합니다.
//!

/// 버텍스 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position_w: vec3<f32>,
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    // 누적 값을 저장하는 렌더 타겟 텍스처
    @location(0) accum: vec4<f32>,
    // 노출 값을 거장하는 렌더 타겟 텍스처
    @location(1) reveal: f32,
};

// 카메라 데이터 유니폼 버퍼입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
};

// 점령 지역 재질 데이터 유니폼 버퍼입니다.
struct CaptureZoneMaterialDataLayout {
    color0: vec4<f32>,
    color1: vec4<f32>,
    time: f32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> u_material: CaptureZoneMaterialDataLayout;

// 점령 지역을 그리는 버텍스 쉐이더입니다.
@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    let position_w = (u_trans * vec4<f32>(position, 1.0)).xyz;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.position_w = position_w;
    return out;
}

// 점령 지역을 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    // 픽셀의 색상을 생성합니다.
    const PI: f32 = 3.141592;
    let tx = fract(cos(input.position_w.x + u_material.time * PI));
    let ty = fract(sin(input.position_w.y + u_material.time * PI));
    let tz = fract(sin(input.position_w.z + u_material.time * PI));
    let ta = fract(u_material.time);
    
    var r = u_material.color0.r * (1.0 - tx) + u_material.color1.r * tx;
    var g = u_material.color0.g * (1.0 - ty) + u_material.color1.g * ty;
    var b = u_material.color0.b * (1.0 - tz) + u_material.color1.b * tz;
    let alpha = u_material.color0.a * (1.0 - ta) + u_material.color1.a * ta;

    let depth = input.clip_position.z;
    let weight = get_weight(depth, alpha);

    var out: RenderTarget;
    out.accum = vec4<f32>(vec3<f32>(r, g, b) * alpha, alpha) * weight;
    out.reveal = alpha;
    return out;
}

// Weighted Blended Order Independent Transparency의 가중치를 구합니다.
fn get_weight(z: f32, a: f32) -> f32 {
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 100.0 / (1e-5 + pow(abs(z) / 10.0, 3.0) + pow(abs(z) / 200.0, 6.0))));
}
