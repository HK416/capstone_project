/// 버텍스 입력 구조체
struct InputAttributes {
    @location(0) position: vec3<f32>, 
    @location(1) texcoord0: vec2<f32>, 
};

/// 버텍스 출력 구조체
struct VertexOutput {
    @builtin(position) position: vec4<f32>, 
    @location(0) texcoord0: vec2<f32>, 
};

/// 엔티티 정보 구조체
struct EntityBlob {
    trans: mat4x4<f32>, 
};

/// 카메라 정보 구조체
struct CameraBlob {
    view_proj: mat4x4<f32>, 
    position: vec3<f32>, 
};



@group(0) @binding(0)
var<uniform> u_entity: EntityBlob;

@group(1) @binding(0)
var<uniform> u_camera: CameraBlob;

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(2) @binding(1)
var s_diffuse: sampler;



@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var output: VertexOutput;
    output.position = u_camera.view_proj * u_entity.trans * vec4<f32>(input.position, 1.0);
    output.texcoord0 = input.texcoord0;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let diffuse = textureSample(t_diffuse, s_diffuse, input.texcoord0);
    return diffuse;
}
