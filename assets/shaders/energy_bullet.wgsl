//! 에너지 볼 형태의 총알을 그리는 쉐이더 코드를 관리합니다.
//!

const PI = 3.14159265359;
const MAX_LIGHTS: u32 = 8u;

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
    @location(2) view_dir: vec3<f32>,
    @location(3) dist: f32,
};

// 프래그먼트 쉐이더 출력 데이터입니다.
struct RenderTarget {
    // 누적 값을 저장하는 렌더 타겟 텍스처
    @location(0) accum: vec4<f32>,
    // 노출 값을 저장하는 렌더 타겟 텍스처
    @location(1) reveal: f32,
    // 발광체 색상을 저장하는 렌더 타겟 텍스처
    @location(2) emissive: vec4<f32>,
};

// 카메라 데이터 유니폼 버퍼
struct CameraDataLayout {
    proj_view: mat4x4<f32>,

    position_w: vec3<f32>,
    _padding0: u32,
};

// 총알 재질 데이터 유니폼 버퍼
struct EnergyBulletMaterialDataLayout {
    main_color: vec3<f32>,
    alpha: f32,

    emissive_color: vec3<f32>,
    _padding0: u32,

    metallic: f32,
    roughness: f32,
    _padding1: u32,
    specular_steps: f32,

    rim_strength: f32,
    rim_power: f32,
    _padding2: vec2<u32>,
};

// 전역 조명 데이터 유니폼 버퍼입니다.
struct GlobalLightDataLayout {
    static_light_proj_view: mat4x4<f32>,

    light_proj_view: mat4x4<f32>,

    direction_w: vec3<f32>,
    intensity: f32,

    color: vec3<f32>,
    _padding0: u32,
};

// 지역 조명 데이터 유니폼 버퍼입니다.
struct LocalLightDataLayout {
    light_proj_view: mat4x4<f32>,

    position_w: vec3<f32>,
    constant: f32,

    color: vec3<f32>,
    linear: f32,

    quadratic: f32,
    _padding0: vec3<u32>,
};

// 지역 조명 데이터 집합 유니폼 버퍼입니다.
struct LocalLightSetDataLayout {
    num_lights: u32,
    _padding0: vec3<u32>,

    lights: array<LocalLightDataLayout, MAX_LIGHTS>,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraDataLayout;

@group(1) @binding(0)
var<uniform> u_trans: mat4x4<f32>;

@group(2) @binding(0)
var<uniform> u_material: EnergyBulletMaterialDataLayout;

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
    let normal_w = normalize((u_trans * vec4<f32>(input.normal, 0.0)).xyz);
    let view_dir = normalize(u_camera.position_w - position_w);
    let dist = distance(u_camera.position_w, position_w);

    out.clip_position = u_camera.proj_view * vec4<f32>(position_w, 1.0);
    out.position_w = position_w;
    out.normal_w = normal_w;
    out.view_dir = view_dir;
    out.dist = dist;

    return out;
}

fn toon_step(value: f32, steps: f32) -> f32 {
    return floor(value * steps) / steps;
}

fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(1.0 - cos_theta, 5.0);
}

fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;

    let denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (PI * denom * denom);
}

fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx1 = geometry_schlick_ggx(NdotV, roughness);
    let ggx2 = geometry_schlick_ggx(NdotL, roughness);
    return ggx1 * ggx2;
}

fn rim_light(N: vec3<f32>, V: vec3<f32>, strength: f32, power: f32) -> f32 {
    let rim = 1.0 - max(dot(N, V), 0.0);
    return pow(rim, power) * strength;
}

fn compute_static_shadow(light_space_pos: vec4<f32>) -> f32 {
    if (light_space_pos.w <= 0.0) {
        return 1.0;
    }

    let proj = light_space_pos / light_space_pos.w;
    let proj_coords = proj.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    let curr_depth = proj.z;

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

fn compute_global_shadow(light_space_pos: vec4<f32>) -> f32 {
    if (light_space_pos.w <= 0.0) {
        return 1.0;
    }
    
    let proj = light_space_pos / light_space_pos.w;
    let proj_coords = proj.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    let curr_depth = proj.z;

    // 그림자 맵 경계 확인
    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 || 
        proj_coords.y < 0.0 || proj_coords.y > 1.0) {
        return 1.0; // 그림자 맵 밖은 그림자 없음
    }
    
    return textureSampleCompare(t_global_light, s_lights, proj_coords, curr_depth);
}

fn get_transparency_weight(z: f32, a: f32) -> f32 {
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 100.0 / (1e-5 + pow(abs(z) / 10.0, 3.0) + pow(abs(z) / 200.0, 6.0))));
}

// 총알을 그리는 프래그먼트 쉐이더입니다.
@fragment
fn fs_main(input: VertexOutput) -> RenderTarget {
    let N = normalize(input.normal_w);
    let V = normalize(input.view_dir);
    let L = normalize(-u_global_light.direction_w);
    let H = normalize(V + L);

    let NdotL = max(dot(N, L), 0.0);
    let NdotV = max(dot(N, V), 0.0);
    let VdotH = max(dot(V, H), 0.0);

    // Fresenel
    let F0 = mix(vec3<f32>(0.04), u_material.main_color, u_material.metallic);
    let F = fresnel_schlick(VdotH, F0);

    // Microfacet BRDF
    let D = distribution_ggx(N, H, u_material.roughness);
    let G = geometry_smith(N, V, L, u_material.roughness);
    let numerator = D * G * F;
    let denominator = max(4.0 * NdotV * NdotL, 0.001);
    let spec = numerator / denominator;

    let kS = F;
    let kD = vec3<f32>(1.0);

    // Toon Banding
    let specular_banded = toon_step(max(max(spec.x, spec.y), spec.z), u_material.specular_steps);

    let diffuse = kD * u_material.main_color * NdotL;
    let specular = kS * specular_banded;

    // Rim
    let rim = rim_light(N, V, u_material.rim_strength, u_material.rim_power);

    // Shadow
    var shadow = 1.0;
    if (input.dist > 10.0) {
        let light_space_pos = u_global_light.static_light_proj_view * vec4<f32>(input.position_w, 1.0);
        shadow = min(shadow, compute_static_shadow(light_space_pos));
    } else {
        let light_space_pos = u_global_light.light_proj_view * vec4<f32>(input.position_w, 1.0);
        shadow = min(shadow, compute_global_shadow(light_space_pos));
    }

    // 최종 색상
    let color = (diffuse + specular) * u_global_light.color + u_global_light.intensity * shadow;
    let final_color = color + rim * u_material.main_color;

    let depth = input.clip_position.z;
    let weight = get_transparency_weight(depth, u_material.alpha);

    var out: RenderTarget;
    out.accum = vec4<f32>(final_color * u_material.alpha, u_material.alpha) * weight;
    out.reveal = u_material.alpha;
    out.emissive = vec4<f32>(u_material.emissive_color, 1.0);
    return out;
}
