struct InputAttributes {
    @location(0) position: vec3<f32>, 
    @location(1) texcoord0: vec2<f32> 
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>, 
    @location(0) texcoord0: vec2<f32>
};

struct RenderTarget {
    @location(0) color: vec4<f32>
};

struct CameraData {
    proj_view: mat4x4<f32>, 
    position: vec3<f32>, 
    direction: vec3<f32>
};

struct EntityData {
    trans: mat4x4<f32>, 
    position: vec3<f32>, 
    texture_flag: u32,
};



@group(0) @binding(0)
var<uniform> u_camera: CameraData;

@group(1) @binding(0)
var<uniform> u_entity: EntityData;

@group(1) @binding(3)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(4)
var s_diffuse: sampler;


@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;
    out.position = u_camera.proj_view * u_entity.trans * vec4<f32>(input.position, 1.0);
    out.texcoord0 = input.texcoord0;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    out.color = textureSample(t_diffuse, s_diffuse, input.texcoord0);
    return out;
}
