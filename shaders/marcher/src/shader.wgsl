//!include rng.wgsl
//!include f32.wgsl
//!include filter.wgsl

const MAX_STEPS: u32 = 128u;
const MAX_BOUNCES: u32 = 4u;
const DELTA: f32 = 0.05;
const BLACKHOLE_RADIUS: f32 = 0.6;
const SKYBOX_RADIUS: f32 = 3.6;

// Features
const DISK_SDF      = 1u << 0;
const DISK_VOL      = 1u << 1;
const SKY_PROC      = 1u << 2;
const AA            = 1u << 3;
const RK4           = 1u << 4;
const ADAPTIVE      = 1u << 5;
const BLOOM         = 1u << 6;

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
    // checks if the bits of f exist in features
    return (pc.features & f) == f;
}

fn rotate(v: vec2<f32>, theta: f32) -> vec2<f32> {
    // 2d rotation without using a matrix
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

fn gravitational_field(p: vec3<f32>) -> vec3<f32> {
    let r = p / BLACKHOLE_RADIUS;
    let R = length(r);
    return -6.0 * r / (R * R * R * R * R);
}

// ODE Integration methods
// https://stackoverflow.com/questions/53645649/cannot-get-rk4-to-solve-for-position-of-orbiting-body-in-python/53650879#53650879

// s: state (position, velocity)
fn ode(s: mat2x3<f32>) -> mat2x3<f32> {
    let p = s.x;
    let v = s.y;
    let a = gravitational_field(p);

    return mat2x3(v, a);
}

// Simpler Euler integration
// s: state (position, velocity)
// h: time step
// returns: (delta position, delta velocity)
fn euler(s: mat2x3<f32>, h: f32) -> mat2x3<f32> {
    return ode(s) * h;
}

// Runge–Kutta (order 4)
// s: state (position, velocity)
// h: time step
// returns: (delta position, delta velocity)
fn rk4(s: mat2x3<f32>, h: f32) -> mat2x3<f32> {
    // calculate coefficients
    let k1 = ode(s);
    let k2 = ode(s + 0.5 * h * k1);
    let k3 = ode(s + 0.5 * h * k2);
    let k4 = ode(s + h * k3);
    // calculate timestep
    let step = h / 6.0 * (k1 + 2.0 * (k2 + k3) + k4);

    return step;
}

const H_MIN: f32 = 1e-8;
const H_MAX: f32 = 1e-1;
const ERR_TOLERANCE: f32 = 1e-5;

/// Bogacki-Shampine method (adaptive step size)
/// https://en.wikipedia.org/wiki/Bogacki%E2%80%93Shampine_method
fn bogacki_shampine(s: mat2x3<f32>, h: ptr<function, f32>) -> mat2x3<f32> {
    let h0 = *h;

    // calculate coefficients
    let k1 = ode(s);
    let k2 = ode(s + 0.5 * h0 * k1);
    let k3 = ode(s + 0.75 * h0 * k2);

    // find step
    let step = (2.0/9.0) * h0 * k1 + (1.0/3.0) * h0 * k2 + (4.0/9.0) * h0 * k3;

    // calculate next state
    let k4 = ode(s + step);

    // calculate better estimate using k4
    let better = (7.0/24.0) * h0 * k1 + (1.0/4.0) * h0 * k2 + (1.0/3.0) * h0 * k3 + (1.0/8.0) * h0 * k4;

    // compute the error
    let err = better - step; // difference between the two guesses
    let err_mag = length(max(err.x, err.y)); // get the magnitude of the largest errors

    // find the step change coefficient
    let x = ERR_TOLERANCE * 0.5 / err_mag;
    let dstep = pow(x, 0.5);

    // update h and clamp within bounds
    // https://en.wikipedia.org/wiki/Adaptive_step_size
    (*h) = 0.9 * clamp((h0 * dstep), H_MIN, H_MAX);

    return step;
}

struct DiskInfo {
    // strength of the emissive color
    emission: vec3<f32>,
    // distance travelled through volume
    distance: f32,
}

fn diskVolume(p: vec3<f32>) -> DiskInfo {
    var ret: DiskInfo;
    ret.emission = vec3<f32>(0.0);
    ret.distance = 0.0;

    // define the bounds of the disk volume
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

// https://www.shadertoy.com/view/wdXGDr
fn diskSdf(p: vec3<f32>, h: f32, r: f32) -> f32 {
    let d = abs(vec2(length(p.xz),p.y)) - vec2(r,h);
    return min(max(d.x,d.y),0.0) + length(max(d, vec2<f32>(0.0)));
}

fn sampleSky(rd: vec3<f32>) -> vec3<f32> {
    // https://en.wikipedia.org/wiki/Azimuth
    let azimuth = atan2(rd.z, rd.x);
    let inclination = asin(-rd.y);

    let uv = vec2<f32>(
        0.5 - (azimuth * FRAC_1_2PI),
        0.5 - (inclination * FRAC_1_PI)
    );

    return textureSampleLevel(stars, star_sampler, uv, 0.0).xyz;
}

fn proceduralSky(rd: vec3<f32>) -> vec3<f32> {
    // https://en.wikipedia.org/wiki/Azimuth
    let azimuth = atan2(rd.z, rd.x);
    let inclination = asin(-rd.y);

    let uv = vec2<f32>(
        0.5 - (azimuth * FRAC_1_2PI),
        0.5 - (inclination * FRAC_1_PI)
    );

    var intensity = 0.0;

    // create a grid of cells and sample radial points (stars)
    // idea from https://www.shadertoy.com/view/ll3yDr
    for (var i = 0; i < 8; i += 1) {
        let uv_s = uv * vec2(f32(i) + 600.0);

        let cells = floor(uv_s + f32(i * 1199));
        let hash = (hash22(cells) * 2.0 - 1.0) * 1.5 * 2.0;
        let hash_magnitude = 1.0-length(hash);

        let grid = fract(uv_s) - 0.5;

        let radius = clamp(hash_magnitude - 0.5, 0.0, 1.0);
        var radialGradient = length(grid - hash) / radius;
        radialGradient = clamp(1.0 - radialGradient, 0.0, 1.0);
        radialGradient *= radialGradient;

        intensity += radialGradient;
    }

    let t = snoise2(uv * 2000.0);
    //http://hyperphysics.phy-astr.gsu.edu/hbase/Starlog/staspe.html
    let color = xyz2rgb(blackbodyXYZ((10000.0 * t * t) + 4000.0));

    return intensity * color;
}

fn render(ro: vec3<f32>, rd: vec3<f32>) -> vec3<f32> {
    // our timestep, start at a low value
    var h = DELTA;

    // color information
    var attenuation = vec3<f32>(1.0);
    var r = vec3<f32>(0.0);

    // add variation to our start point along the direction
    var p = ro + (rand() * h * rd);
    // our inital velocity is just ray direction
    var v = rd;

    // keep track of the number of bounces the light takes
    // this is useful when integrating volumes
    var bounces = 0u;

    for (var i = 0u; i < MAX_STEPS; i++) {
        if bounces > MAX_BOUNCES {
            // discard sample, light gets stuck
            return vec3<f32>(-1.0);
        }

        if dot(p, p) < BLACKHOLE_RADIUS * BLACKHOLE_RADIUS {
            // light has entered the black hole...
            // dont just return black, we might have gone through a volume to get here
            return r;
        }

        if dot(p, p) > SKYBOX_RADIUS * SKYBOX_RADIUS {
            // we have hit the skybox
            // no need to integrate anymore
            break;
        }

        if has_feature(DISK_VOL) {
            let sample = diskVolume(p);
            r += attenuation * sample.emission * h;

            if sample.distance > 0.0 {
                // hit the disc

                // the equation for absorbance
                // https://en.wikipedia.org/wiki/Absorbance#Beer-Lambert_law
                let absorbance = exp(-1.0 * h * sample.distance);
                if absorbance < rand() {
                    // change the direction of v but keep its magnitude
                    v = length(v) * reflect(normalize(v), udir3());

                    attenuation *= pc.disk_color;

                    bounces++;
                }
            }
        } else if has_feature(DISK_SDF) {
            // represent the disk as a cylinder
            // it's much easier to see the entire volume of the disk this way,
            // without any fancy volume and fbm
            let dist = diskSdf(p, pc.disk_thickness, sqrt(pc.disk_radius));

            if dist <= 0.0 {
                // hit the disk
                return pc.disk_color;
            }
        }

        // create state
        let s = mat2x3(p, v);

        // integrate
        var step = mat2x3f();

        // choose the method of integration
        if has_feature(ADAPTIVE) {
            step = bogacki_shampine(s, &h);
        } else if has_feature(RK4) {
            step = rk4(s, DELTA);
        } else {
            step = euler(s, DELTA);
        }

        // update system
        p += step.x;
        v += step.y;
    }

    if has_feature(SKY_PROC) {
        // procedurally create the skybox
        r += attenuation * proceduralSky(normalize(v));
    } else {
        // sample the sky from a texture
        r += attenuation * sampleSky(normalize(v));
    }

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

    if has_feature(AA) {
        coord = aa_filter(coord);
    }

    // calculate uv coordinates
    var uv = 2.0 * (coord - 0.5 * res) / max(res.x, res.y);

    if has_feature(BLOOM) {
        // monte carlo bloom
        // uses a guassian distribution centered around the current uv
        // the sigma (variance) is "how far the pixel is offset" (chosen by random)
        // this has the effect of creating a nice bloom effect when accumulation is on.
        // this is much more performant than running a post rendering bloom pass,
        // as we're using the path renderer to do this for us.
        let r = rand();
        if r < 0.10 {
            uv = nrand2(uv, rand() * 0.015);
        } else if r > 0.90 {
            uv = nrand2(uv, rand() * 0.200);
        }
    }

    // since we have to pass in the transform as a Mat4, we have to extend these vectors with a zero (to ignore translation)
    // the ray origin
    let ro = (vec4<f32>(pc.origin, 0.0) * pc.transform).xyz;
    // the ray direction (multiplied by the fov factor 2 * FOV * 1/PI, which gives us 90 degrees = 1.0 factor)
    let rd = normalize((vec4<f32>(uv * 2.0 * pc.fov * FRAC_1_PI, -1.0, 0.0) * pc.transform).xyz);

    // render using the ray information
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
