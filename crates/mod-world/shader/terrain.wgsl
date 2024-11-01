/// 지역 조명의 최대 개수입니다.
const MAX_LIGHTS = 16;



/// 정점의 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>, 
    @location(1) normal: vec3<f32>, 
    @location(2) texcoord0: vec2<f32>, 
    @location(3) texcoord1: vec2<f32>  
};

/// 버텍스 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, 
    @location(0) texcoord0: vec2<f32>, 
    @location(1) texcoord1: vec2<f32> 
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>, 
};

/// 카메라의 데이터 레이아웃입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>, 
    position_w: vec3<f32>, 
    direction_w: vec3<f32>
};

/// 전역 조명의 데이터 레이아웃입니다.
struct GlobalLightDataLayout {
    color: vec4<f32>, 
    direction_w: vec3<f32>
}

/// 지역 조명의 데이터 레이아웃입니다.
struct LocalLightDataLayout {
    color: vec4<f32>, 
    position_w: vec3<f32>, 
    range_w: f32, 
    direction_w: vec3<f32>, 
    angle: f32
};

/// 지역 조명 집합 레이아웃입니다.
struct LocalLightSetLayout {
    lights: array<LocalLightDataLayout, MAX_LIGHTS>, 
    num_lights: u32, 
};

/// 정적 메쉬 데이터 레이아웃입니다.
struct StaticMeshDataLayout {
    trans: mat4x4<f32> 
};

/// `Universal` 재질 데이터 레이아웃입니다.
struct UniversalMaterialDataLayout {
    glossiness: f32, 
    smoothness: f32, 
    metallic: f32, 
    height: f32, 
    albedo: vec4<f32>, 
    specular: vec4<f32>, 
    emissive: vec4<f32> 
};



@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

// @group(0) @binding(1)
// var<uniform> u_global_light: GlobalLightDataLayout;

// @group(0) @binding(2)
// var<uniform> u_local_lights: LocalLightSetLayout;

@group(1) @binding(0)
var<uniform> u_model: StaticMeshDataLayout;

@group(2) @binding(0)
var<uniform> u_material: UniversalMaterialDataLayout;

@group(2) @binding(1)
var t_diffuse: texture_2d<f32>;

@group(2) @binding(2)
var s_diffuse: sampler;

@group(2) @binding(3)
var t_specular: texture_2d<f32>;

@group(2) @binding(4)
var s_specular: sampler;

@group(2) @binding(5)
var t_emissive: texture_2d<f32>;

@group(2) @binding(6)
var s_emissive: sampler;

@group(2) @binding(7)
var t_normal: texture_2d<f32>;

@group(2) @binding(8)
var s_normal: sampler;

@group(2) @binding(9)
var t_height: texture_2d<f32>;

@group(2) @binding(10)
var s_height: sampler;




/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;
    
    let height = textureSampleLevel(t_height, s_height, input.texcoord0, 0.0).r * u_material.height;
    let distance = input.normal * height;
    let position_w = (u_model.trans * vec4<f32>(input.position + distance, 1.0)).xyz;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.texcoord0 = input.texcoord0;
    out.texcoord1 = input.texcoord1;

    return out;
}

/// 프레그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    // out.color = textureSample(t_diffuse, s_diffuse, input.texcoord1);
    out.color = vec4<f32>(input.texcoord1.xy, 0.0, 1.0);
    return out;
}
