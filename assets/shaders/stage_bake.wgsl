//! 지형을 생성하는 쉐이더 코드를 관리합니다.
//!

// 조명 데이터 유니폼 버퍼입니다.
struct LightDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
    _padding0: u32,
    color: vec3<f32>,
    _padding1: u32,
    constant: f32,
    linear: f32,
    quadratic: f32,
};

@group(0) @binding(0)
var<uniform> u_light: LightDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

// 지형의 그림자를 생성하는 버텍스 쉐이더입니다.
@vertex
fn vs_bake(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    let position_w = (u_trans * vec4<f32>(position, 1.0)).xyz;
    return u_light.proj_view * vec4<f32>(position_w, 1.0);
}
