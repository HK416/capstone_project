//! 총구 화염 파티클 이팩트를 그리는 쉐이더 코드를 관리합니다.
//!

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) trans_row_0: vec4<f32>,
    @location(3) trans_row_1: vec4<f32>,
    @location(4) trans_row_2: vec4<f32>,
    @location(5) trans_row_3: vec4<f32>,
    @location(6) tint: vec4<f32>,
    @location(7) index: u32,
};

// 버텍스 쉐이더 출력 데이터입니다.
// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) index: u32,
};

// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    // 누적 값을 저장하는 렌더 타겟 텍스처
    @location(0) accum: vec4<f32>,
    // 노출 값을 저장하는 렌더 타겟 텍스처
    @location(1) reveal: f32,
    // 발광체 색상을 저장하는 렌더 타겟 텍스처
    @location(2) emissive: vec4<f32>,
};

// 카메라 데이터 유니폼 버퍼
struct CameraDataLayout {
    proj_view: mat4x4<f32>,

    position_w: vec3<f32>,
    _padding0: u32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var t_gray_scale: texture_2d_array<f32>;

@group(1) @binding(1)
var s_gray_scale: sampler;

/// 고정된 총구 화염 이펙트 파티클을 그리는 버텍스 쉐이더입니다.
@vertex 
fn vs_main(input: InputAttributes) -> VertexOutput {
    let trans = mat4x4<f32>(
        input.trans_row_0, 
        input.trans_row_1, 
        input.trans_row_2, 
        input.trans_row_3, 
    );
    let position_w = (trans * vec4<f32>(input.position, 1.0)).xyz;

    var out: VertexOutput;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    out.index = input.index;
    return out;
}

fn get_transparency_weight(z: f32, a: f32) -> f32 {
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 100.0 / (1e-5 + pow(abs(z) / 10.0, 3.0) + pow(abs(z) / 200.0, 6.0))));
}

@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    let gray_scale = textureSample(t_gray_scale, s_gray_scale, input.uv, input.index).r;
    let final_color = input.tint.xyz * vec3<f32>(gray_scale);
    let alpha = input.tint.w * gray_scale;

    let depth = input.clip_position.z;
    let weight = get_transparency_weight(depth, alpha);

    var out: RenderTarget;
    out.accum = vec4<f32>(final_color * alpha, alpha) * weight;
    out.reveal = alpha;
    out.emissive = vec4<f32>(0.0);
    return out;
}
