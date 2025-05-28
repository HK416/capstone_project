//! 각 렌더 타겟 텍스처에 나뉜 데이터를 총합하는 쉐이더
//!

/// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec2<f32>,
    @location(1) texcoord: vec2<f32>,
};

/// 버텍스 쉐이더 출력 데이터입니다.
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
var t_bloom: texture_2d<f32>;

@group(0) @binding(1)
var s_bloom: sampler;

/// 버텍스 쉐이더
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
    out.color = textureSample(t_bloom, s_bloom, input.texcoord);
    return out;
}
