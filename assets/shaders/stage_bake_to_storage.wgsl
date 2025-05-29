//! 지형을 생성하는 쉐이더 코드를 관리합니다.
//!

@group(0) @binding(0)
var<uniform> u_light_trans: mat4x4<f32>;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var depth_out: texture_storage_2d<r32float, write>;

// 지형의 그림자를 생성하는 버텍스 쉐이더입니다.
@vertex
fn vs_bake(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    let position_w = (u_trans * vec4<f32>(position, 1.0)).xyz;
    return u_light_trans * vec4<f32>(position_w, 1.0);
}
