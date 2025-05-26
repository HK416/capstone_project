//! 각 렌더 타겟 텍스처에 나뉜 데이터를 총합하는 쉐이더
//!

const EPSILON: f32 = 1e-5;


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
var t_accum: texture_2d<f32>;
@group(0) @binding(1)
var t_reveal: texture_2d<f32>;
@group(0) @binding(2)
var s_composite: sampler;


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
    let accum = textureSample(t_accum, s_composite, input.texcoord);
    let reveal = textureSample(t_reveal, s_composite, input.texcoord).r;

    let color = accum.rgb / max(accum.a, EPSILON);
    let alpha = 1.0 - reveal;

    var out: RenderTarget;
    out.color = vec4<f32>(color, alpha);
    return out;
}
