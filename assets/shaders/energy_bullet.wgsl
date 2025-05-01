//! 에너지 볼 형태의 총알을 그리는 쉐이더 코드를 관리합니다.
//!

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
    // 노출 값을 거장하는 렌더 타겟 텍스처
    @location(1) reveal: f32,
};

// 카메라 데이터 유니폼 버퍼
struct CameraDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
};

// 총알 재질 데이터 유니폼 버퍼
struct EnergyBulletMaterialDataLayout {
    emissive: vec3<f32>,
    main_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> u_material: EnergyBulletMaterialDataLayout;

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
    let weight = get_weight(depth, alpha);

    var out: RenderTarget;
    out.accum = vec4<f32>(color.rgb * alpha, alpha) * weight;
    out.reveal = alpha;
    return out;
}

// Weighted Blended Order Independent Transparency의 가중치를 구합니다.
fn get_weight(z: f32, a: f32) -> f32 {
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 100.0 / (1e-5 + pow(abs(z) / 10.0, 3.0) + pow(abs(z) / 200.0, 6.0))));
}
