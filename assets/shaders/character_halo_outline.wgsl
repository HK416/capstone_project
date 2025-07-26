//! 캐릭터의 헤일로의 외곽선을 그리는 쉐이더 코드를 관리합니다.
//! 

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
};

// 버텍스 쉐이더 출력 데이터입니다.
// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>,
    @location(1) emissive: vec4<f32>,
};

// 카메라 데이터 유니폼 버퍼입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>,

    position_w: vec3<f32>,
    _padding0: u32,
};

// 헤일로 외곽선 유니폼 버퍼입니다.
struct HaloOutlineDataLayout {
    color: vec3<f32>,
    _padding0: u32,
    scale: vec3<f32>,
    _padding1: u32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> u_outline: HaloOutlineDataLayout;

// 캐릭터 외곽선을 그리는 버텍스 쉐이더입니다.
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    let scale = mat4x4<f32>(
        vec4<f32>(u_outline.scale.x, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, u_outline.scale.y, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, u_outline.scale.z, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );
    var position_w = (u_trans * scale * vec4<f32>(input.position, 1.0)).xyz;
    let V = normalize(position_w - u_camera.position_w);
    position_w = position_w + V * 0.05;

    var out: VertexOutput;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    return out;
}

// 캐릭터 외곽선을 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    out.color = vec4<f32>(u_outline.color, 1.0);
    out.emissive = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    return out;
}
