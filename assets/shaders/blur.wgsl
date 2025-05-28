//! 가우시안 블러를 수행하는 컴퓨트 쉐이더 코드를 관리합니다.
//!

const offsets = array<i32, 9>(-4, -3, -2, -1, 0, 1, 2, 3, 4);
const weights = array<f32, 9>(0.0162, 0.0540, 0.1216, 0.1945, 0.2270, 0.1945, 0.1216, 0.0540, 0.0162);

@group(0) @binding(0)
var t_input: texture_2d<f32>;

@group(0) @binding(1)
var t_output: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(16, 16)
fn cs_horizontal_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tex_size = textureDimensions(t_input);
    if (gid.x >= tex_size.x || gid.y >= tex_size.y) {
        return;
    }

    let current_pos = vec2<i32>(gid.xy);

    var result = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < 9u; i += 1u) {
        let sample_pos = current_pos + vec2<i32>(offsets[i], 0);

        if (sample_pos.x >= 0 && sample_pos.x < i32(tex_size.x)
        && sample_pos.y >= 0 && sample_pos.y < i32(tex_size.y)) {
            result += textureLoad(t_input, sample_pos, 0).rgb * weights[i];
        }
    }

    textureStore(t_output, current_pos, vec4<f32>(result, 1.0));
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
