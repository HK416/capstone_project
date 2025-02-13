//! 각 렌더 타겟 텍스처에 나뉜 데이터를 총합하는 쉐이더
//!
const EPSILON: f32 = 1.192092896e-07f;

/// 정점 입력 속성입니다.
struct InputAttributes {
    @builtin(vertex_index) index: u32,
};

/// 버텍스 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>,
};


@group(0) @binding(0)
var accum: texture_2d<f32>;
@group(0) @binding(1)
var reveal: texture_2d<f32>;


/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;
    switch (input.index) {
        case 0u: {
            out.clip_position = vec4<f32>(-1.0, -1.0, 0.0, 1.0);
            break;
        }
        case 1u: {
            out.clip_position = vec4<f32>(-1.0, 1.0, 0.0, 1.0);
            break;
        }
        case 2u: {
            out.clip_position = vec4<f32>(1.0, -1.0, 0.0, 1.0);
            break;
        }
        case 3u: {
            out.clip_position = vec4<f32>(1.0, 1.0, 0.0, 1.0);
            break;
        }
        default { }
    }
    return out;
}

/// 프래그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    let coordinate = vec2<i32>(input.clip_position.xy);
    let revealage = textureLoad(reveal, coordinate, 0).r;
    if (is_approximately_equal(revealage, 1.0)) {
        discard;
    }

    var accumulation = textureLoad(accum, coordinate, 0);
    let maximum = max(max(abs(accumulation.x), abs(accumulation.y)), abs(accumulation.z));
    if (is_infinite(maximum)) {
        accumulation = vec4<f32>(accumulation.a, accumulation.a, accumulation.a, accumulation.a);
    }
    let average_color = accumulation.rgb / max(accumulation.a, EPSILON);

    var out: RenderTarget;
    out.color = vec4<f32>(average_color, 1.0 - revealage);
    return out;
}

fn is_infinite(v: f32) -> bool {
    return v != 0.0 && v * 2.0 == v;
}

fn is_approximately_equal(a: f32, b: f32) -> bool {
    return abs(a - b) <= max(abs(a), abs(b)) * EPSILON;
}
