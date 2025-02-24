struct InputAttributes {
    @builtin(vertex_index) vertex_index: u32,
};

/// 버텍스 쉐이더 출력 데이터입니다.
/// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, 
    @location(0) texcoord: vec2<f32>,
};

/// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    // 누적 값을 저장하는 렌더 타겟 텍스처
    @location(0) accum: vec4<f32>,
    // 노출 값을 저장하는 렌더 타겟 텍스처
    @location(1) reveal: f32,
};

/// 카메라 데이터 레이아웃입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>, 
    position_w: vec3<f32>, 
    direction_w: vec3<f32>, 
};

/// 데미지 파티클 데이터 레이아웃입니다.
struct FxDamageDataLayout {
    trans: mat4x4<f32>,
    position_v: vec3<f32>,
    number: u32,
    width: f32,
    height: f32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_particle: FxDamageDataLayout;

@group(1) @binding(1)
var t_font: texture_2d<f32>;

@group(1) @binding(2)
var s_font: sampler;

/// 버텍스 쉐이더
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    let number = f32(u_particle.number % 10);
    var offset: vec3<f32>;
    var texcoord: vec2<f32>;
    switch (input.vertex_index) {
        case 0u: {
            offset = vec3<f32>(
                -0.5 * u_particle.width, 
                -0.5 * u_particle.height, 
                0.0
            );
            texcoord = vec2<f32>(0.2 * (number % 5), 0.5 * floor(number / 5) + 0.5);
        }
        case 1u: {
            offset = vec3<f32>(
                -0.5 * u_particle.width, 
                0.5 * u_particle.height, 
                0.0
            );
            texcoord = vec2<f32>(0.2 * (number % 5), 0.5 * floor(number / 5));
        }
        case 2u: {
            offset = vec3<f32>(
                0.5 * u_particle.width, 
                -0.5 * u_particle.height, 
                0.0
            );
            texcoord = vec2<f32>(0.2 * (number % 5) + 0.2, 0.5 * floor(number / 5) + 0.5);
        }
        case 3u: {
            offset = vec3<f32>(
                0.5 * u_particle.width, 
                0.5 * u_particle.height, 
                0.0
            );
            texcoord = vec2<f32>(0.2 * (number % 5) + 0.2, 0.5 * floor(number / 5));
        }
        default { }
    }

    // 카메라가 바라보는 방향을 계산합니다.
    var position_w = (u_particle.trans * vec4<f32>(0.0, 0.0, 0.0, 1.0)).xyz;
    var up = vec3<f32>(0.0, 1.0, 0.0);
    let look = normalize(position_w - u_camera.position_w);
    let right = cross(up, look);
    up = cross(look, right);
    
    // 위치를 계산합니다.
    position_w = position_w 
        + right * offset.x + right * u_particle.position_v.x
        + up * offset.y + up * u_particle.position_v.y
        + look * offset.z + look * u_particle.position_v.z;

    var out: VertexOutput;
    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.texcoord = texcoord;
    return out;
}

/// 프래그먼트 쉐이더
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    let depth = input.clip_position.z;
    var color = textureSample(t_font, s_font, input.texcoord);
    color.a = floor(color.a); // 다른 불투명 오브젝트와 가능한 겹치지 않도록 하기 위함
    let weight = get_weight(depth, color.a);

    var out: RenderTarget;
    out.accum = vec4<f32>(color.rgb * color.a, color.a) * weight;
    out.reveal = color.a;
    return out;
}

fn get_weight(z: f32, a: f32) -> f32 {
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 100.0 / (1e-5 + pow(abs(z) / 10.0, 3.0) + pow(abs(z) / 200.0, 6.0))));
}
