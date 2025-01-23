/// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
};

/// 버텍스 쉐이더 출력 데이터
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texcoord: vec3<f32>,
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>,
};

/// Skybox 데이터 레이아웃입니다.
struct SkyboxDataLayout {
    proj_view: mat4x4<f32>,
    color: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> u_camera: SkyboxDataLayout;

@group(0) @binding(1)
var t_skybox: texture_cube<f32>;

@group(0) @binding(2)
var s_skybox: sampler;

/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;

    let position = u_camera.proj_view * vec4<f32>(input.position, 0.0);
    out.clip_position = position.xyww;
    out.texcoord = input.position;

    return out;
}

/// 프래그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;

    out.color = textureSample(t_skybox, s_skybox, input.texcoord);

    return out;
}
