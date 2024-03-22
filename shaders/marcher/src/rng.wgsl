// https://gist.github.com/munrocket/236ed5ba7e409b8bdf1ff6eca5dcdc39
// https://www.shadertoy.com/view/WttyWX

var<private> state: vec4<u32>;

// Creates a good seed for the rng
fn seed_rng(p: vec2<u32>, r: vec2<u32>, s: u32) {
    state = vec4<u32>(
        (p.x << 16) ^ p.y,
        p.x ^ r.y * s,
        p.y ^ r.x * s,
        (r.x << 16) ^ r.y,
    );
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(p.xyx * vec3f(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx+19.19);
    return fract((p3.xx+p3.yz)*p3.zy);
}

// https://www.pcg-random.org/
// http://www.jcgt.org/published/0009/03/02/
fn pcg4d(p: vec4u) -> vec4u {
    var v = p * 1664525u + 1013904223u;
    v.x += v.y * v.w; v.y += v.z * v.x; v.z += v.x * v.y; v.w += v.y * v.z;
    v ^= v >> vec4u(16u);
    v.x += v.y * v.w; v.y += v.z * v.x; v.z += v.x * v.y; v.w += v.y * v.z;
    return v;
}

fn rand() -> f32 {
    state = pcg4d(state);
    return f32(state.x) / f32(0xffffffffu);
}

fn rand2() -> vec2<f32> {
    state = pcg4d(state);
    return vec2<f32>(state.xy) / f32(0xffffffffu);
}

fn rand3() -> vec3<f32> {
    state = pcg4d(state);
    return vec3<f32>(state.xyz) / f32(0xffffffffu);
}

fn rand4() -> vec4<f32> {
    state = pcg4d(state);
    return vec4<f32>(state) / f32(0xffffffffu);
}

fn udir2() -> vec2<f32> {
    // https://mathworld.wolfram.com/DiskPointPicking.html
    let u = rand();     // [0, 1]
    let r = TAU * u;    // [0, 2pi] for trig
    // convert to cartesian
    let s = sin(r);
    let c = cos(r);
    return vec2(s, c);
}

fn udir3() -> vec3<f32> {
    // https://mathworld.wolfram.com/SpherePointPicking.html
    let uv = rand2();
    let r = vec2<f32>(TAU * uv.x, acos(2.0 * uv.y - 1.0));
    // convert from spherical to cartesian
    // https://uk.mathworks.com/help/symbolic/transform-spherical-coordinates-and-plot.html
    let s = sin(r);
    let c = cos(r);
    return vec3<f32>(c.x * s.y, s.x * s.y, c.y);
}

// 2D gaussian normal random value
fn nrand2(mean: vec2<f32>, sigma: f32) -> vec2<f32> {
    let z = rand2();
    // https://en.wikipedia.org/wiki/Box%E2%80%93Muller_transform
    let g = sqrt(-2.0 * log(z.x)) * vec2(cos(TAU * z.y), sin(TAU * z.y));
    return mean + sigma * g;
}

fn mod289_2(x: vec2f) -> vec2f { return x - floor(x * (1. / 289.)) * 289.; }
fn mod289_3(x: vec3f) -> vec3f { return x - floor(x * (1. / 289.)) * 289.; }
fn mod289_4(x: vec4f) -> vec4f { return x - floor(x * (1. / 289.)) * 289.; }
fn perm3(x: vec3f) -> vec3f { return mod289_3(((x * 34.) + 1.) * x); }
fn perm4(x: vec4f) -> vec4f { return mod289_4(((x * 34.) + 1.) * x); }

// Optimized Ashima SimplexNoise2D
// https://www.shadertoy.com/view/4sdGD8
fn snoise2(v: vec2f) -> f32 {
    var i = floor((v.x + v.y) * 0.36602540378443 + v);
    let x0 = v + (i.x + i.y) * 0.211324865405187 - i;
    let s = step(x0.x, x0.y);
    let j = vec2f(1.0 - s, s);
    let x1 = x0 - j + 0.211324865405187; 
    let x3 = x0 - 0.577350269189626;
    i = mod289_2(i);
    let p = perm3(perm3(i.y + vec3f(0.0, j.y, 1.0)) + i.x + vec3f(0.0, j.x, 1.0));
    let x = 2.0 * fract(p * 0.024390243902439) - 1.0;
    let h = abs(x) - 0.5;
    let a0 = x - floor(x + 0.5);
    let mSq = vec3f(x0.x*x0.x + x0.y*x0.y, x1.x*x1.x + x1.y*x1.y, x3.x*x3.x + x3.y*x3.y);
    let m = max(0.5 - mSq, vec3f(0.0));
    return 0.5 + 65.0 * dot(m*m*m*m * (-0.85373472095314 * (a0*a0 + h*h) + 1.79284291400159), a0 * vec3f(x0.x, x1.x, x3.x) + h * vec3f(x0.y, x1.y, x3.y));
}

fn noise3(p: vec3f) -> f32 {
    let a = floor(p);
    var d: vec3f = p - a;
    d = d * d * (3. - 2. * d);

    let b = a.xxyy + vec4f(0., 1., 0., 1.);
    let k1 = perm4(b.xyxy);
    let k2 = perm4(k1.xyxy + b.zzww);

    let c = k2 + a.zzzz;
    let k3 = perm4(c);
    let k4 = perm4(c + 1.);

    let o1 = fract(k3 * (1. / 41.));
    let o2 = fract(k4 * (1. / 41.));

    let o3 = o2 * d.z + o1 * (1. - d.z);
    let o4 = o3.yw * d.x + o3.xz * (1. - d.x);

    return o4.y * d.y + o4.x * (1. - d.y);
}

fn fbm(p: vec3f, iter: u32) -> f32 {
    var value = 0.0;
    var accum = 0.0;
    var atten = 0.5;
    var scale = 1.0;

    for (var i = 0u; i < iter; i++) {
        value += atten * noise3(scale * p);
        accum += atten;
        atten *= 0.5;
        scale *= 2.5;
    }

    return select(value / accum, value, accum == 0.0);
}