//! 방어막 파티클 이펙트를 그리는 쉐이더 코드를 관리합니다.
//! 

const PI: f32 = 3.141592;

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) trans_row_0: vec4<f32>,
    @location(3) trans_row_1: vec4<f32>,
    @location(4) trans_row_2: vec4<f32>,
    @location(5) trans_row_3: vec4<f32>,
};

// 버텍스 쉐이더 출력 데이터입니다.
// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal_w: vec3<f32>,
    @location(1) view_dir: vec3<f32>,
    @location(2) position_y: f32,
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

/// 방어막 데이터 유니폼 버퍼
struct ShieldDataLayout {
    color: vec3<f32>,
    time: f32,
    rim_strength: f32,
    rim_power: f32,
    _padding0: vec2<u32>,
}

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_shield: ShieldDataLayout; 

@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    let trans = mat4x4<f32>(
        input.trans_row_0,
        input.trans_row_1,
        input.trans_row_2,
        input.trans_row_3,
    );
    let position_w = (trans * vec4<f32>(input.position, 1.0)).xyz;
    let normal_w = (trans * vec4<f32>(input.normal, 0.0)).xyz;
    let view_dir = normalize(u_camera.position_w - position_w);

    var out: VertexOutput;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.normal_w = normal_w;
    out.view_dir = view_dir;
    out.position_y = input.position.y;
    return out;
}

fn get_transparency_weight(z: f32, a: f32) -> f32 {
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 100.0 / (1e-5 + pow(abs(z) / 10.0, 3.0) + pow(abs(z) / 200.0, 6.0))));
}

fn rim_light(N: vec3<f32>, V: vec3<f32>, strength: f32, power: f32) -> f32 {
    let rim = 1.0 - max(dot(N, V), 0.0);
    return pow(rim, power) * strength;
}

@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    let N = normalize(input.normal_w);
    let V = normalize(input.view_dir);

    // Rim
    let rim = rim_light(N, V, u_shield.rim_strength, u_shield.rim_power);
    let rim_color = vec3<f32>(rim);

    // Alpha
    let y = input.position_y;
    let t = fract(u_shield.time * 0.2);
    let alpha = max(0.3 * sin(y + 2.0 * t * PI) + 0.15, 0.0);

    let depth = input.clip_position.z;
    let weight = get_transparency_weight(depth, alpha);
    
    let final_color = u_shield.color + u_shield.color * rim;

    var out: RenderTarget;
    out.accum = vec4<f32>(final_color * alpha, alpha) * weight;
    out.reveal = alpha;
    out.emissive = vec4<f32>(0.0);
    return out;
}
