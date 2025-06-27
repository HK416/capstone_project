//! 프레임 버퍼에 출력하는 쉐이더
//!

/// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec2<f32>,
    @location(1) texcoord: vec2<f32>,
};

/// 정점 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>,
};

@group(0) @binding(0)
var t_content: texture_2d<f32>;

@group(0) @binding(1)
var s_content: sampler;

/// 정점 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    out.texcoord = input.texcoord;
    return out;
}

/// 프래그먼트 쉐이더
@fragment 
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    let color = textureSample(t_content, s_content, input.texcoord).rgb;
    out.color = vec4(pow(color, vec3(1.0 / 2.2)), 1.0); // 감마 보정
    return out;
}
