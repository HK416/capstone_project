//! 데미지 폰트를 그리는 쉐이더 코드를 관리합니다.
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
    @location(0) color: vec4<f32>,
};

// 카메라 데이터 유니폼 버퍼입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
};

// 데미지 폰트 데이터 유니폼 버퍼입니다.
struct DamageFontDataLayout {
    trans: mat4x4<f32>,
    number: u32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_font: DamageFontDataLayout;

@group(1) @binding(1)
var t_font: texture_2d_array<f32>;

@group(1) @binding(2)
var s_font: sampler;

// 데미지 폰트를 그리는 버텍스 쉐이더입니다.
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;

    let position_w = (u_font.trans * vec4<f32>(input.position, 1.0)).xyz;

    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.texcoord = input.texcoord;
    
    return out;
}

// 데미지 폰트를 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    let number = clamp(u_font.number, 0u, 9u);
    let color = textureSample(t_font, s_font, input.texcoord, number);
    if (color.a < 0.5) {
        discard;
    }

    var out: RenderTarget;
    out.color = vec4<f32>(color.rgb, 1.0);
    return out;
}
