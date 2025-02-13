/// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) texcoord: vec2<f32>,
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

/// 재질 데이터 레이아웃입니다.
struct MaterialDataLayout {
    glossiness: f32, 
    smoothness: f32, 
    metallic: f32, 
    bump_scale: f32, 
    parallax: f32,
    strength: f32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> u_material: MaterialDataLayout;

@group(2) @binding(1)
var t_albedo: texture_2d<f32>;

@group(2) @binding(2)
var s_albedo: sampler;

// @group(2) @binding(3)
// var t_specular: texture_2d<f32>;

// @group(2) @binding(4)
// var s_specular: sampler;

// @group(2) @binding(5)
// var t_emissive: texture_2d<f32>;

// @group(2) @binding(6)
// var s_emissive: sampler;

// @group(2) @binding(7)
// var t_normal: texture_2d<f32>;

// @group(2) @binding(8)
// var s_normal: sampler;

// @group(2) @binding(9)
// var t_parallax: texture_2d<f32>;

// @group(2) @binding(10)
// var s_parallax: sampler;

// @group(2) @binding(11)
// var t_occlusion: texture_2d<f32>;

// @group(2) @binding(12)
// var s_occlusion: sampler;

/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;
    let position_w = (u_trans * vec4<f32>(input.position, 1.0)).xyz;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.texcoord = input.texcoord;
    return out;
}

/// 프래그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    out.color = textureSample(t_albedo, s_albedo, input.texcoord);
    return out;
}
