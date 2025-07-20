//! 일반 캐릭터의 외곽을 그리는 쉐이더 코드를 관리합니다.
//! 

const MAX_BONES: u32 = 256u;

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) bone_index: vec4<u32>, 
    @location(2) bone_weight: vec4<f32>,
};

// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>,
    @location(1) emissive: vec4<f32>,
};

// 카메라 데이터 유니폼 버퍼입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>,

    position_w: vec3<f32>,
    _padding0: u32,
};

/// 외곽 데이터 유니폼 버퍼입니다.
struct OutlineDataLayout {
    color: vec3<f32>,
    scale: f32,
}

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_bindposes: array<mat4x4<f32>, MAX_BONES>;

@group(1) @binding(1)
var<uniform> u_bone_trans: array<mat4x4<f32>, MAX_BONES>;

@group(2) @binding(0)
var<uniform> u_outline: OutlineDataLayout;

// 캐릭터를 그리는 버텍스 쉐이더입니다.
@vertex
fn vs_main(input: InputAttributes) -> @builtin(position) vec4<f32> {
    var out: VertexOutput;

    let bone_transform_0 = u_bone_trans[input.bone_index[0]] * u_bindposes[input.bone_index[0]];
    let bone_transform_1 = u_bone_trans[input.bone_index[1]] * u_bindposes[input.bone_index[1]];
    let bone_transform_2 = u_bone_trans[input.bone_index[2]] * u_bindposes[input.bone_index[2]];
    let bone_transform_3 = u_bone_trans[input.bone_index[3]] * u_bindposes[input.bone_index[3]];

    let final_matrix = input.bone_weight[0] * bone_transform_0 +
                   input.bone_weight[1] * bone_transform_1 +
                   input.bone_weight[2] * bone_transform_2 +
                   input.bone_weight[3] * bone_transform_3;

    let scale = u_outline.scale;
    let scale_matrix = mat4<f32>(
        vec4<f32>(scale, 0.0, 0.0, 0.0), 
        vec4<f32>(0.0, scale, 0.0, 0.0), 
        vec4<f32>(0.0, 0.0, scale, 0.0), 
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    let position_w = (scale_matrix * final_matrix * vec4<f32>(input.position, 1.0)).xyz;
    return  u_camera.proj_view * vec4<f32>(position_w, 1.0);
}

// 캐릭터를 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    var out: RenderTarget;
    out.color = vec4(u_outline.color, 1.0);
    out.emissive = vec4<f32>(0.0);
    return out;
}
