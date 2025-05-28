//! 일반 총알을 그리는 쉐이더 코드를 관리합니다.
//!

// 최대 조명의 개수입니다.
const max_lights: u32 = 8u;

// 깊이 bias입니다.
const bias: f32 = 0.001;

// 정점 입력 속성입니다.
struct InputAttributes {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

// 버텍스 쉐이더 출력 데이터입니다.
// 프래그먼트 쉐이더 입력 데이터로 사용됩니다.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position_w: vec3<f32>,
    @location(1) normal_w: vec3<f32>,
};

// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    @location(0) color: vec4<f32>,
    @location(1) bloom: vec4<f32>,
};

// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTargetTransparency {
    // 누적 값을 저장하는 렌더 타겟 텍스처
    @location(0) accum: vec4<f32>,
    // 노출 값을 저장하는 렌더 타겟 텍스처
    @location(1) reveal: f32,
    // 발광체 색상을 저장하는 렌더 타겟 텍스처
    @location(2) bloom: vec4<f32>,
};

// 카메라 데이터 유니폼 버퍼입니다.
struct CameraDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
};

// 총알 재질 데이터 유니폼 버퍼입니다.
struct BulletMaterialDataLayout {
    glossiness: f32,
    smoothness: f32,
    metallic: f32,
    main_color: vec4<f32>,
};

// 전역 조명 데이터 유니폼 버퍼입니다.
struct GlobalLightDataLayout {
    static_proj_view: mat4x4<f32>,
    proj_view: mat4x4<f32>,
    direction_w: vec3<f32>,
    color: vec3<f32>,
};

// 지역 조명 데이터 유니폼 버퍼입니다.
struct LocalLightDataLayout {
    proj_view: mat4x4<f32>,
    position_w: vec3<f32>,
    constant: f32,
    color: vec3<f32>,
    linear: f32,
    quadratic: f32,
};

// 지역 조명 데이터 집합 유니폼 버퍼입니다.
struct LocalLightSetDataLayout {
    lights: array<LocalLightDataLayout, max_lights>,
    num_lights: u32,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> u_material: BulletMaterialDataLayout;

@group(3) @binding(0)
var<uniform> u_global_light: GlobalLightDataLayout;

@group(3) @binding(1)
var<uniform> u_local_lights: LocalLightSetDataLayout;

@group(3) @binding(2)
var t_static_light: texture_2d<f32>;

@group(3) @binding(3)
var s_static_light: sampler;

@group(3) @binding(4)
var t_global_light: texture_depth_2d;

@group(3) @binding(5)
var t_local_lights: texture_depth_2d_array;

@group(3) @binding(6)
var s_lights: sampler_comparison;

// 총알을 그리는 버텍스 쉐이더입니다.
@vertex
fn vs_main(input: InputAttributes) -> VertexOutput {
    var out: VertexOutput;

    let position_w = (u_trans * vec4<f32>(input.position, 1.0)).xyz;
    let normal_w = (u_trans * vec4<f32>(input.normal, 0.0)).xyz;

    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.position_w = position_w;
    out.normal_w = normal_w;

    return out;
}

// 총알을 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    // 노멀을 계산합니다.
    let normal = normalize(input.normal_w);
    // 카메라로부터 거리를 계산합니다.
    let distance = distance(u_camera.position_w, input.position_w);

    // 주 색상을 가져옵니다.
    let main_color = u_material.main_color.rgb;

    // 전역 조명의 그림자 색상을 계산합니다.
    var shadow = 1.0;
    var diffuse = max(0.0, dot(normal, normalize(-u_global_light.direction_w)));
    var color = main_color * 0.75; // ambient 조명

    var light_space_position: vec4<f32>;
    if (distance > 10.0) {
        // 정적인 오브젝트의 그림자를 계산합니다.
        light_space_position = u_global_light.static_proj_view * vec4<f32>(input.position_w, 1.0);
        shadow = min(shadow, calculate_static_shadow(light_space_position));
        color += main_color * u_global_light.color * diffuse * shadow;
    } else {
        // 전역 조명의 그림자를 계산합니다.
        light_space_position = u_global_light.proj_view * vec4<f32>(input.position_w, 1.0);
        shadow = min(shadow, calculate_global_shadow(light_space_position));
        color += main_color * u_global_light.color * diffuse * shadow;
    }

    var out: RenderTarget;
    out.color = vec4<f32>(pow(color.rgb, vec3(1.0 / 2.2)), 1.0); // 감마 보정
    out.bloom = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    return out;
}

// 총알을 투명하게 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_transparency_main(input: VertexOutput) -> RenderTargetTransparency {
    // 노멀을 계산합니다.
    let normal = normalize(input.normal_w);
    // 카메라로부터 거리를 계산합니다.
    let distance = distance(u_camera.position_w, input.position_w);

    // 주 색상을 가져옵니다.
    let main_color = u_material.main_color.rgb;

    // 전역 조명의 그림자 색상을 계산합니다.
    var shadow = 1.0;
    var diffuse = max(0.0, dot(normal, normalize(-u_global_light.direction_w)));
    var color = main_color * 0.75; // ambient 조명

    var light_space_position: vec4<f32>;
    if (distance > 10.0) {
        // 정적인 오브젝트의 그림자를 계산합니다.
        light_space_position = u_global_light.static_proj_view * vec4<f32>(input.position_w, 1.0);
        shadow = min(shadow, calculate_static_shadow(light_space_position));
        color += main_color * u_global_light.color * diffuse * shadow;
    } else {
        // 전역 조명의 그림자를 계산합니다.
        light_space_position = u_global_light.proj_view * vec4<f32>(input.position_w, 1.0);
        shadow = min(shadow, calculate_global_shadow(light_space_position));
        color += main_color * u_global_light.color * diffuse * shadow;
    }

    let alpha = u_material.main_color.a;
    let depth = input.clip_position.z;
    let weight = calculate_weight(depth, alpha);

    var out: RenderTargetTransparency;
    out.accum = vec4<f32>(color.rgb * alpha, alpha) * weight;
    out.reveal = alpha;
    out.bloom = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    return out;
}

// Weighted Blended Order Independent Transparency의 가중치를 구합니다.
fn calculate_weight(z: f32, a: f32) -> f32 {
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 100.0 / (1e-5 + pow(abs(z) / 10.0, 3.0) + pow(abs(z) / 200.0, 6.0))));
}


/// 정적인 오브젝트의 전역 조명 그림자를 계산합니다.
fn calculate_static_shadow(light_space_position: vec4<f32>) -> f32 {
    if (light_space_position.w <= 0.0) {
        return 1.0;
    }

    let curr_depth = clamp(light_space_position.z / light_space_position.w - bias, 0.0, 1.0);
    var proj_coords = light_space_position.xy / light_space_position.w;
    proj_coords = proj_coords * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

    var uv: vec2<f32>;
    var depth: f32;
    var shadow = 0.0;
    let texel_size = 1.0 / vec2<f32>(textureDimensions(t_static_light));
    for (var x = -1; x <= 1; x += 1) {
        for (var y = -1; y <= 1; y += 1) {
            uv = proj_coords + vec2<f32>(f32(x), f32(y)) * texel_size;
            depth = textureSample(t_static_light, s_static_light, uv).r;
            if (curr_depth <= depth) {
                shadow += 1.0;
            }
        }
    }
    shadow /= 9.0;

    return shadow;
}

/// 전역 조명의 그림자를 계산합니다.
fn calculate_global_shadow(light_space_position: vec4<f32>) -> f32 {
    if (light_space_position.w <= 0.0) {
        return 1.0;
    }
    
    let curr_depth = clamp(light_space_position.z / light_space_position.w - bias, 0.0, 1.0);
    var proj_coords = light_space_position.xy / light_space_position.w;
    proj_coords = proj_coords * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    
    // 그림자 맵 경계 확인
    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 || 
        proj_coords.y < 0.0 || proj_coords.y > 1.0) {
        return 1.0; // 그림자 맵 밖은 그림자 없음
    }
    
    return textureSampleCompare(t_global_light, s_lights, proj_coords, curr_depth);
}

/// 지역 조명의 그림자를 계산합니다.
fn calculate_local_shadow(index: u32, light_space_position: vec4<f32>) -> f32 {
    if (light_space_position.w <= 0.0) {
        return 1.0;
    }
    
    let curr_depth = clamp(light_space_position.z / light_space_position.w - bias, 0.0, 1.0);
    var proj_coords = light_space_position.xy / light_space_position.w;
    proj_coords = proj_coords * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    
    // 그림자 맵 경계 확인
    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 || 
        proj_coords.y < 0.0 || proj_coords.y > 1.0) {
        return 1.0; // 그림자 맵 밖은 그림자 없음
    }
    
    return textureSampleCompare(t_local_lights, s_lights, proj_coords, i32(index), curr_depth);
}
