//! 일반 캐릭터의 그림자를 생성하는 쉐이더 코드를 관리합니다.
//! 

// 최대 뼈 노드의 개수입니다.
const max_bones: u32 = 256u;

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) bone_index: vec4<u32>, 
    @location(2) bone_weight: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u_light_trans: mat4x4<f32>;

@group(1) @binding(0)
var<uniform> u_bindposes: array<mat4x4<f32>, max_bones>;

@group(1) @binding(1)
var<uniform> u_bone_trans: array<mat4x4<f32>, max_bones>;

// 캐릭터의 그림자를 생성하는 버텍스 쉐이더입니다.
@vertex
fn vs_bake(input: InputAttributes) -> @builtin(position) vec4<f32> {
    let bone_transform_0 = u_bone_trans[input.bone_index[0]] * u_bindposes[input.bone_index[0]];
    let bone_transform_1 = u_bone_trans[input.bone_index[1]] * u_bindposes[input.bone_index[1]];
    let bone_transform_2 = u_bone_trans[input.bone_index[2]] * u_bindposes[input.bone_index[2]];
    let bone_transform_3 = u_bone_trans[input.bone_index[3]] * u_bindposes[input.bone_index[3]];

    let final_matrix = input.bone_weight[0] * bone_transform_0 +
                   input.bone_weight[1] * bone_transform_1 +
                   input.bone_weight[2] * bone_transform_2 +
                   input.bone_weight[3] * bone_transform_3;
    
    let position_w = (final_matrix * vec4<f32>(input.position, 1.0)).xyz;
    
    return u_light_trans * vec4<f32>(position_w, 1.0);
}
