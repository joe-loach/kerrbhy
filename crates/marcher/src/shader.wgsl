const MAX_STEPS: u32 = 128u;
const DELTA: f32 = 0.05;
const BLACKHOLE_RADIUS: f32 = 0.6;
const SKYBOX_RADIUS: f32 = 3.6;

const M_1_PI: f32 = 1.0 / 3.1415926535897932384626433832795;
const M_1_2PI: f32 = 1.0 / 6.283185307179586476925286766559;

struct PushConstants {
    origin: vec3<f32>,
    fov: f32,
}

@group(0) @binding(0)
var buffer: texture_storage_2d<rgba16float, read_write>;

@group(1) @binding(1)
var star_sampler: sampler;
@group(1) @binding(2)
var stars: texture_2d<f32>;

var<push_constant> pc: PushConstants;

fn sky(rd: vec3<f32>) -> vec3<f32> {
    let coord = vec2<f32>(
        0.5 - (atan2(rd.z, rd.x) * M_1_2PI),
        0.5 - (asin(-rd.y) * M_1_PI)
    );

    return textureSampleLevel(stars, star_sampler, coord, 0.0).xyz;
}

fn gravitational_field(p: vec3<f32>) -> vec3<f32>
{
    let r = p / BLACKHOLE_RADIUS;
    let R = length(r);
    return -6.0 * r / (R * R * R * R * R);
}

fn render(ro: vec3<f32>, rd: vec3<f32>) -> vec3<f32> {
    var r = vec3<f32>(0.0);

    var p = ro;
    var v = rd;

    for (var i = 0u; i < MAX_STEPS; i++) {
        if dot(p, p) < BLACKHOLE_RADIUS * BLACKHOLE_RADIUS {
            return r;
        }

        if dot(p, p) > SKYBOX_RADIUS * SKYBOX_RADIUS {
            break;
        }

        // TODO: use RK4
        let g = gravitational_field(p);
        v += g * DELTA;
        p += v * DELTA;
    }

    r += sky(normalize(v));

    return r;
}

@compute @workgroup_size(8, 8, 1)
fn comp(@builtin(global_invocation_id) id: vec3<u32>) {
    let dim: vec2<u32> = textureDimensions(buffer);

    // don't do work outside buffer
    if (id.x >= dim.x || id.y >= dim.y) {
        return;
    }

    // calculate uv coordinates
    let res = vec2<f32>(dim.xy);
    let coord = vec2<f32>(id.xy);
    let uv = 2.0 * (coord - 0.5*res) / max(res.x, res.y);

    let ro = pc.origin;
    let rd = normalize(vec3<f32>(uv * 2.0 * pc.fov * M_1_PI, -1.0));

    let color = vec4<f32>(render(ro, rd), 1.0);
    let old_color = textureLoad(buffer, id.xy);

    let acuum = normalize(color + old_color);

    textureStore(buffer, id.xy, acuum);
}
