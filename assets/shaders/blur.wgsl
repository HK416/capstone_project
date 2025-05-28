//! 가우시안 블러를 수행하는 컴퓨트 쉐이더 코드를 관리합니다.
//!

const num_kernels = 5u;
const offsets = array<i32, num_kernels>(-2, -1, 0, 1, 2);
const weights = array<f32, num_kernels>(0.06136, 0.24477, 0.38774, 0.24477, 0.06136);

@group(0) @binding(0)
var t_input: texture_2d<f32>;

@group(0) @binding(1)
var t_output: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(16, 16)
fn cs_horizontal_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tex_size = textureDimensions(t_input);
    let current_pos = vec2<i32>(gid.xy) * 2;
    if (u32(current_pos.x) >= tex_size.x 
    || u32(current_pos.y) >= tex_size.y) {
        return;
    }

    var result = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < 9u; i += 1u) {
        let sample_pos = current_pos + vec2<i32>(offsets[i], 0);

        if (sample_pos.x >= 0 && sample_pos.x < i32(tex_size.x)
        && sample_pos.y >= 0 && sample_pos.y < i32(tex_size.y)) {
            result += textureLoad(t_input, sample_pos, 0).rgb * weights[i];
        }
    }

    textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(result, 1.0));
}

@compute @workgroup_size(16, 16)
fn cs_vertical_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tex_size = textureDimensions(t_input);
    if (gid.x >= tex_size.x || gid.y >= tex_size.y) {
        return;
    }

    let current_pos = vec2<i32>(gid.xy);

    var result = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < 9u; i += 1u) {
        let sample_pos = current_pos + vec2<i32>(0, offsets[i]);

        if (sample_pos.x >= 0 && sample_pos.x < i32(tex_size.x)
        && sample_pos.y >= 0 && sample_pos.y < i32(tex_size.y)) {
            result += textureLoad(t_input, sample_pos, 0).rgb * weights[i];
        }
    }

    textureStore(t_output, current_pos, vec4<f32>(result, 1.0));
}
