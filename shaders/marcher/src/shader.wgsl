//!include rng.wgsl
//!include f32.wgsl
//!include filter.wgsl

const MAX_STEPS: u32 = 128u;
const MAX_BOUNCES: u32 = 4u;
const DELTA: f32 = 0.05;
const BLACKHOLE_RADIUS: f32 = 0.6;
const SKYBOX_RADIUS: f32 = 3.6;

// Features
const DISK: u32 = 1u;
const AA: u32 = 2u;

struct PushConstants {
    origin: vec3<f32>,
    fov: f32,
    disk_color: vec3<f32>,
    disk_radius: f32,
    disk_thickness: f32,
    sample: u32,
    features: u32,
    pad: u32,
    transform: mat4x4<f32>,
}

@group(0) @binding(0)
var buffer: texture_storage_2d<rgba8unorm, read_write>;

@group(1) @binding(1)
var star_sampler: sampler;
@group(1) @binding(2)
var stars: texture_2d<f32>;

var<push_constant> pc: PushConstants;

fn has_feature(f: u32) -> bool {
    return (pc.features & f) == f;
}

fn rotate(v: vec2<f32>, theta: f32) -> vec2<f32> {
    let s = sin(theta);
    let c = cos(theta);
    return vec2<f32>(v.x * c - v.y * s, v.x * s + v.y * c);
}

const XYZ2sRGB: mat3x3<f32> = mat3x3<f32>(
    3.240, -1.537, -0.499,
    -0.969, 1.876, 0.042,
    0.056, -0.204, 1.057
);

// Convert XYZ to sRGB
fn xyz2rgb(color_xyz: vec3<f32>) -> vec3<f32> {
    return color_xyz * XYZ2sRGB;
}

fn blackbodyXYZ(t: f32) -> vec3<f32> {
    // https://en.wikipedia.org/wiki/Planckian_locus
    let u = (0.860117757 + 1.54118254E-4 * t + 1.28641212E-7 * t * t) / (1.0 + 8.42420235E-4 * t + 7.08145163E-7 * t * t);
    let v = (0.317398726 + 4.22806245E-5 * t + 4.20481691E-8 * t * t) / (1.0 - 2.89741816E-5 * t + 1.61456053E-7 * t * t);

    // https://en.wikipedia.org/wiki/CIE_1960_color_space
    // https://en.wikipedia.org/wiki/XYZ_color_space

    // convert to x and y in CIE xy
    let xy = vec2<f32>(3.0 * u, 2.0 * v) / (2.0 * u - 8.0 * v + 4.0);

    // convert to XYZ
    return vec3(xy.x / xy.y, 1.0, (1.0 - xy.x - xy.y) / xy.y);
}

struct DiskInfo {
    // strength of the emissive color
    emission: vec3<f32>,
    // distance travelled through volume
    distance: f32,
}

fn disk(p: vec3<f32>) -> DiskInfo {
    var ret: DiskInfo;
    ret.emission = vec3<f32>(0.0);
    ret.distance = 0.0;

    if dot(p.xz, p.xz) > pc.disk_radius || p.y * p.y > pc.disk_thickness {
        return ret;
    }

    let np = 20.0 * vec3<f32>(rotate(p.xz, (8.0 * p.y) + (4.0 * length(p.xz))), p.y).xzy;
    let n0 = fbm(np, 8u);

    let d_falloff = length(vec3(0.12, 7.50, 0.12) * p);
    let e_falloff = length(vec3(0.20, 8.00, 0.20) * p);

    // add random variations to temperature
    let t = rand();
    var e = xyz2rgb(blackbodyXYZ((4000.0 * t * t) + 2000.0));
    // "normalize" e, but don't go to infinity
    e = clamp(
        e / max(max(max(e.r, e.g), e.b), 0.01),
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );

    let h_p = 0.5 * p;
    e *= 128.0 * max(n0 - e_falloff, 0.0) / (dot(h_p, h_p) + 0.05);

    ret.emission = e;
    ret.distance = 128.0 * max(n0 - d_falloff, 0.0);

    return ret;
}

fn sky(rd: vec3<f32>) -> vec3<f32> {
    // https://en.wikipedia.org/wiki/Azimuth
    let azimuth = atan2(rd.z, rd.x);
    let inclination = asin(-rd.y);

    let coord = vec2<f32>(
        0.5 - (azimuth * FRAC_1_2PI),
        0.5 - (inclination * FRAC_1_PI)
    );

    return textureSampleLevel(stars, star_sampler, coord, 0.0).xyz;
}

fn gravitational_field(p: vec3<f32>) -> vec3<f32> {
    let r = p / BLACKHOLE_RADIUS;
    let R = length(r);
    return -6.0 * r / (R * R * R * R * R);
}

fn render(ro: vec3<f32>, rd: vec3<f32>) -> vec3<f32> {
    var attenuation = vec3<f32>(1.0);
    var r = vec3<f32>(0.0);

    var p = ro + (rand() * DELTA * rd);
    var v = rd;

    var bounces = 0u;

    for (var i = 0u; i < MAX_STEPS; i++) {
        if bounces > MAX_BOUNCES {
            // discard sample, light gets stuck
            return vec3<f32>(-1.0);
        }

        if dot(p, p) < BLACKHOLE_RADIUS * BLACKHOLE_RADIUS {
            return r;
        }

        if dot(p, p) > SKYBOX_RADIUS * SKYBOX_RADIUS {
            break;
        }

        if has_feature(DISK) {
            let sample = disk(p);
            r += attenuation * sample.emission * DELTA;

            if sample.distance > 0.0 {
                // hit the disc

                let absorbance = exp(-1.0 * DELTA * sample.distance);
                if absorbance < rand() {
                    // change the direction of v but keep its magnitude
                    v = length(v) * reflect(normalize(v), udir3());

                    attenuation *= pc.disk_color;

                    bounces++;
                }
            }
        }

        // TODO: use RK4
        // https://en.wikipedia.org/wiki/Runge%E2%80%93Kutta%E2%80%93Fehlberg_method
        let g = gravitational_field(p);
        v += g * DELTA;
        p += v * DELTA;
    }

    r += attenuation * sky(normalize(v));

    return r;
}

@compute @workgroup_size(8, 8, 1)
fn comp(@builtin(global_invocation_id) id: vec3<u32>) {
    let dim: vec2<u32> = textureDimensions(buffer);

    // don't do work outside buffer
    if id.x >= dim.x || id.y >= dim.y {
        return;
    }

    // seed the rng
    seed_rng(id.xy, dim.xy, pc.sample);

    let res = vec2<f32>(dim.xy);
    var coord = vec2<f32>(id.xy);

    if (has_feature(AA)) {
        coord = aa_filter(coord);
    }

    // calculate uv coordinates
    var uv = 2.0 * (coord - 0.5 * res) / max(res.x, res.y);
    // switch y because wgpu uses strange texture coords
    uv.y = -uv.y;

    // TODO: add AA filtering to the uv
    // https://en.wikipedia.org/wiki/Spatial_anti-aliasing

    let ro = (vec4<f32>(pc.origin, 0.0) * pc.transform).xyz;
    let rd = normalize((vec4<f32>(uv * 2.0 * pc.fov * FRAC_1_PI, -1.0, 0.0) * pc.transform).xyz);

    var color = render(ro, rd);

    // remove unused samples
    color = select(
        color,
        vec3<f32>(0.0),
        any(color < vec3<f32>(0.0)) || any(isInf(color)) || any(isNan(color))
    );

    // gamma correction
    color = pow(color, vec3<f32>(0.45));

    // accumulate the color in the buffer
    let old_color = textureLoad(buffer, id.xy);
    let acc = mix(old_color, vec4<f32>(color, 1.0), 1.0 / f32(pc.sample + 1));

    textureStore(buffer, id.xy, acc);
}
