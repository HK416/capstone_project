/// 정점의 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32> 
};

/// 버텍스 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32> 
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32> 
}



/// 카메라의 데이터 레이아웃입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>, 
    position_w: vec3<f32>, 
    direction_w: vec3<f32> 
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

@group(1) @binding(0)
var<uniform> u_model: StaticMeshDataLayout;

@group(2) @binding(0)
var<uniform> u_material: UniversalMaterialDataLayout;



/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;
    
    let position_w = (u_model.trans * vec4<f32>(input.position, 1.0)).xyz;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);

    return out;
}

/// 프레그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;

    out.color = u_material.albedo;

    return out;
}
