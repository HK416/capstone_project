/// 정점 입력 속성
struct InputAttributes {
    @builtin(vertex_index) vertex_index: u32,
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

/// 카메라 데이터 레이아웃입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>, 
    position_w: vec3<f32>, 
    direction_w: vec3<f32>, 
};

/// 배경화면 데이터 레이아웃
struct BackgroundDataLayout {
    ratio: f32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_background: BackgroundDataLayout;

@group(1) @binding(1)
var t_color: texture_2d<f32>;

@group(1) @binding(2)
var s_color: sampler;

/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var position: vec3<f32>;
    var texcoord: vec2<f32>;

    switch (input.vertex_index) {
        case 0u: {
            position = vec3<f32>(-u_background.ratio, -1.0, 0.0);
            texcoord = vec2<f32>(0.0, 1.0);
        }
        case 1u: {
            position = vec3<f32>(-u_background.ratio, 1.0, 0.0);
            texcoord = vec2<f32>(0.0, 0.0);
        }
        case 2u: {
            position = vec3<f32>(u_background.ratio, -1.0, 0.0);
            texcoord = vec2<f32>(1.0, 1.0);
        }
        case 3u: {
            position = vec3<f32>(u_background.ratio, 1.0, 0.0);
            texcoord = vec2<f32>(1.0, 0.0);
        }
        default { }
    }

    var out: VertexOutput;
    out.clip_position = u_camera.proj_view * vec4<f32>(position, 1.0);
    out.clip_position.z = 1.0;
    out.texcoord = texcoord;
    return out;
}

/// 프래그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    out.color = textureSample(t_color, s_color, input.texcoord);
    return out;
}
