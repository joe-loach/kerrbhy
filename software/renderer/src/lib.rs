use std::f32::consts::{
    FRAC_1_PI,
    PI,
    TAU,
};

use common::{
    Config,
    Features,
};
use glam::{
    mat3,
    Mat3,
    Vec2,
    Vec3,
    Vec3Swizzles as _,
    Vec4,
    Vec4Swizzles as _,
};
use wcpu::{
    texture::Filter,
    FrameBuffer,
    Sample,
    Sampler,
    Texture2D,
};

pub struct Renderer {
    buffer: FrameBuffer,
    config: Config,

    sampler: Sampler,
    stars: Texture2D,
}

const MAX_STEPS: u32 = 128;
const MAX_BOUNCES: u32 = 4;
const DELTA: f32 = 0.05;
const BLACKHOLE_RADIUS: f32 = 0.6;
const SKYBOX_RADIUS: f32 = 3.6;

const FRAC_1_2PI: f32 = FRAC_1_PI * 0.5;

fn mat2x3(x: Vec3, y: Vec3) -> Mat3 {
    mat3(x, y, Vec3::ZERO)
}

fn reflect(i: Vec3, n: Vec3) -> Vec3 {
    i - 2.0 * n.dot(i) * n
}

fn sin(v: Vec2) -> Vec2 {
    Vec2::new(v.x.sin(), v.y.sin())
}

fn cos(v: Vec2) -> Vec2 {
    Vec2::new(v.x.cos(), v.y.cos())
}

fn rand() -> f32 {
    fastrand::f32()
}

fn rand2() -> Vec2 {
    Vec2::new(rand(), rand())
}

fn udir2() -> Vec2 {
    // https://mathworld.wolfram.com/DiskPointPicking.html
    let u = rand(); // [0, 1]
    let r = TAU * u; // [0, 2pi] for trig
                     // convert to cartesian
    let s = r.sin();
    let c = r.cos();
    Vec2::new(s, c)
}

fn udir3() -> Vec3 {
    // https://mathworld.wolfram.com/SpherePointPicking.html
    let uv = rand2();
    let r = Vec2::new(TAU * uv.x, (2.0 * uv.y - 1.0).acos());
    // convert from spherical to cartesian
    // https://uk.mathworks.com/help/symbolic/transform-spherical-coordinates-and-plot.html
    let s = sin(r);
    let c = cos(r);
    Vec3::new(c.x * s.y, s.x * s.y, c.y)
}

// 2D gaussian normal random value
fn nrand2(mean: Vec2, sigma: f32) -> Vec2 {
    let z = rand2();
    // https://en.wikipedia.org/wiki/Box%E2%80%93Muller_transform
    let g = (-2.0 * z.x.log10()).sqrt() * Vec2::new((TAU * z.y).cos(), (TAU * z.y).sin());

    mean + sigma * g
}

fn rotate(v: Vec2, theta: f32) -> Vec2 {
    let s = theta.sin();
    let c = theta.cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn mod289(x: Vec4) -> Vec4 {
    x - (x * (1.0 / 289.0)).floor() * 289.0
}
fn perm4(x: Vec4) -> Vec4 {
    mod289(((x * 34.0) + 1.0) * x)
}

fn noise3(p: Vec3) -> f32 {
    let a = p.floor();
    let mut d = p - a;
    d = d * d * (3. - 2. * d);

    let b = a.xxyy() + Vec4::new(0., 1., 0., 1.);
    let k1 = perm4(b.xyxy());
    let k2 = perm4(k1.xyxy() + b.zzww());

    let c = k2 + a.zzzz();
    let k3 = perm4(c);
    let k4 = perm4(c + 1.);

    let o1 = (k3 * (1. / 41.)).fract();
    let o2 = (k4 * (1. / 41.)).fract();

    let o3 = o2 * d.z + o1 * (1. - d.z);
    let o4 = o3.yw() * d.x + o3.xz() * (1. - d.x);

    o4.y * d.y + o4.x * (1. - d.y)
}

fn fbm(p: Vec3, iter: u32) -> f32 {
    let mut value = 0.0;
    let mut accum = 0.0;
    let mut atten = 0.5;
    let mut scale = 1.0;

    for _ in 0..iter {
        value += atten * noise3(scale * p);
        accum += atten;
        atten *= 0.5;
        scale *= 2.5;
    }

    if accum == 0.0 {
        value
    } else {
        value / accum
    }
}

const XYZ2_SRGB: Mat3 = Mat3::from_cols(
    Vec3::new(3.240, -1.537, -0.499),
    Vec3::new(-0.969, 1.876, 0.042),
    Vec3::new(0.056, -0.204, 1.057),
);

// Convert XYZ to sRGB
fn xyz2rgb(color_xyz: Vec3) -> Vec3 {
    // Note: glsl uses column-major, not row-major matricies (as they are in glam)
    // transpose before multiplying
    XYZ2_SRGB.transpose() * color_xyz
}

#[allow(clippy::excessive_precision)]
fn blackbody_xyz(t: f32) -> Vec3 {
    // https://en.wikipedia.org/wiki/Planckian_locus
    #[rustfmt::skip]
    let u = (0.860117757 + 1.54118254E-4 * t + 1.28641212E-7 * t * t) / (1.0 + 8.42420235E-4 * t + 7.08145163E-7 * t * t);
    #[rustfmt::skip]
    let v = (0.317398726 + 4.22806245E-5 * t + 4.20481691E-8 * t * t) / (1.0 - 2.89741816E-5 * t + 1.61456053E-7 * t * t);

    // https://en.wikipedia.org/wiki/CIE_1960_color_space
    // https://en.wikipedia.org/wiki/XYZ_color_space

    // convert to x and y in CIE xy
    let xy = Vec2::new(3.0 * u, 2.0 * v) / (2.0 * u - 8.0 * v + 4.0);

    // convert to XYZ
    Vec3::new(xy.x / xy.y, 1.0, (1.0 - xy.x - xy.y) / xy.y)
}

fn aa_filter(coord: Vec2) -> Vec2 {
    const A: f32 = 0.35875;
    const B: f32 = 0.48829;
    const C: f32 = 0.14128;
    const D: f32 = 0.01168;

    // https://en.wikipedia.org/wiki/Window_function#Blackman%E2%80%93Harris_window
    // Window functions:
    // "Used to smoothly bring a sampled signal down to zero at the edges of the
    // region"
    let n = 0.5 * rand() + 0.5;
    let w = A - B * (2.0 * PI * n).cos() + C * (4.0 * PI * n).cos() - D * (6.0 * PI * n).cos();

    coord + (udir2() * 2.0 * w)
}

struct DiskInfo {
    // strength of the emissive color
    emission: Vec3,
    // distance travelled through volume
    distance: f32,
}

fn disk_volume(p: Vec3, radius: f32, thickness: f32) -> DiskInfo {
    if p.xz().length_squared() > radius || p.y * p.y > thickness {
        return DiskInfo {
            emission: Vec3::ZERO,
            distance: 0.0,
        };
    }

    let np = 20.0
        * rotate(p.xz(), (8.0 * p.y) + (4.0 * p.xz().length()))
            .extend(p.y)
            .xzy();
    let n0 = fbm(np, 8);

    let d_falloff = (Vec3::new(0.12, 7.50, 0.12) * p).length();
    let e_falloff = (Vec3::new(0.20, 8.00, 0.20) * p).length();

    // add random variations to temperature
    let t = rand();
    let mut e = xyz2rgb(blackbody_xyz((4000.0 * t * t) + 2000.0));
    // "normalize" e, but don't go to infinity
    e = (e / e.max_element().max(0.01)).clamp(Vec3::ZERO, Vec3::ONE);

    let h_p = 0.5 * p;
    e *= 128.0 * (n0 - e_falloff).max(0.0) / (h_p.length_squared() + 0.05);

    DiskInfo {
        emission: e,
        distance: 128.0 * (n0 - d_falloff).max(0.0),
    }
}

// https://www.shadertoy.com/view/wdXGDr
fn disk_sdf(p: Vec3, h: f32, r: f32) -> f32 {
    let d = Vec2::new(p.xz().length(), p.y).abs() - Vec2::new(r, h);
    d.x.clamp(d.y, 0.0) + d.max(Vec2::ZERO).length()
}

fn sky(sampler: Sampler, stars: &Texture2D, rd: Vec3) -> Vec3 {
    // https://en.wikipedia.org/wiki/Azimuth
    let azimuth = f32::atan2(rd.z, rd.x);
    let inclination = f32::asin(-rd.y);

    let coord = Vec2::new(
        0.5 - (azimuth * FRAC_1_2PI),
        0.5 - (inclination * FRAC_1_PI),
    );

    sampler.sample(stars, coord).xyz()
}

fn gravitational_field(p: Vec3) -> Vec3 {
    let r = p / BLACKHOLE_RADIUS;
    let rn = r.length();
    -6.0 * r / (rn * rn * rn * rn * rn)
}

/// s: state (position, velocity)
fn ode(s: Mat3) -> Mat3 {
    let p = s.x_axis;
    let v = s.y_axis;
    let a = gravitational_field(p);

    mat2x3(v, a)
}

/// Simpler Euler integration
/// s: state (position, velocity)
/// h: time step
/// returns: (delta position, delta velocity)
fn euler(s: Mat3, h: f32) -> Mat3 {
    ode(s) * h
}

/// Runge–Kutta (order 4)
/// s: state (position, velocity)
/// h: time step
/// returns: (delta position, delta velocity)
fn rk4(s: Mat3, h: f32) -> Mat3 {
    // calculate coefficients
    let k1 = ode(s);
    let k2 = ode(s + 0.5 * h * k1);
    let k3 = ode(s + 0.5 * h * k2);
    let k4 = ode(s + h * k3);

    // calculate timestep
    h / 6.0 * (k1 + 2.0 * (k2 + k3) + k4)
}

/// Bogacki-Shampine method
/// https://en.wikipedia.org/wiki/Bogacki%E2%80%93Shampine_method
fn bogacki_shampine(s: Mat3, h: &mut f32) -> Mat3 {
    const A: [f32; 3] = [2.0/9.0, 1.0/3.0, 4.0/9.0];
    const B: [f32; 4] = [7.0/24.0, 1.0/4.0, 1.0/3.0, 1.0/8.0];

    const H_MIN: f32 = 1e-8;
    const H_MAX: f32 = 1e-1;
    const ERR_TOLERANCE: f32 = 1e-5;

    let h0 = *h;

    // calculate coefficients
    let k1 = ode(s);
    let k2 = ode(s + 0.5 * h0 * k1);
    let k3 = ode(s + 0.75 * h0 * k2);

    // find step
    let step = A[0] * h0 * k1 + A[1] * h0 * k2 + A[2] * h0 * k3;

    // calculate next state
    let k4 = ode(s + step);

    // calculate better estimate using k4
    let better = B[0] * h0 * k1 + B[1] * h0 * k2 + B[2] * h0 * k3 + B[3] * h0 * k4;

    // compute the error
    let err = better - step; // difference between the two guesses
    let err = err.x_axis.max(err.y_axis).length(); // get the magnitude of the largest errors

    // find the step change coefficient
    let x = ERR_TOLERANCE * 0.5 / err;
    let dstep = x.powf(0.5);

    // update h and clamp within bounds
    // https://en.wikipedia.org/wiki/Adaptive_step_size
    (*h) = 0.9 * (h0 * dstep).clamp(H_MIN, H_MAX);

    step
}

#[profiling::function]
fn render(ro: Vec3, rd: Vec3, sampler: Sampler, stars: &Texture2D, config: &Config) -> Vec3 {
    let mut h = DELTA;

    let mut attenuation = Vec3::ONE;
    let mut r = Vec3::ZERO;

    let mut p = ro + (rand() * h * rd);
    let mut v = rd;

    let mut bounces = 0_u32;

    for _ in 0..MAX_STEPS {
        if bounces > MAX_BOUNCES {
            // discard sample, light gets stuck
            return Vec3::splat(-1.0);
        }

        if p.length_squared() < BLACKHOLE_RADIUS * BLACKHOLE_RADIUS {
            return r;
        }

        if p.length_squared() > SKYBOX_RADIUS * SKYBOX_RADIUS {
            break;
        }

        if config.features.contains(Features::DISK_VOL) {
            let sample = disk_volume(p, config.disk.radius, config.disk.thickness);
            r += attenuation * sample.emission * h;

            if sample.distance > 0.0 {
                // hit the disc

                let absorbance = (-1.0 * h * sample.distance).exp();
                if absorbance < rand() {
                    // change the direction of v but keep its magnitude
                    v = v.length() * reflect(v.normalize(), udir3());

                    attenuation *= config.disk.color;

                    bounces += 1;
                }
            }
        } else if config.features.contains(Features::DISK_SDF) {
            let dist = disk_sdf(p, config.disk.thickness, config.disk.radius.sqrt());

            if dist <= 0.0 {
                // hit the disc
                return config.disk.color;
            }
        }

        let s = mat2x3(p, v);

        let step = if config.features.contains(Features::ADAPTIVE) {
            bogacki_shampine(s, &mut h)
        } else if config.features.contains(Features::RK4) {
            rk4(s, h)
        } else {
            euler(s, h)
        };
        // eprintln!("{h}");

        // update system
        p += step.x_axis;
        v += step.y_axis;
    }

    if config.features.contains(Features::SKY_PROC) {
    } else {
        r += attenuation * sky(sampler, stars, v.normalize());
    }

    r
}

impl Renderer {
    #[profiling::function]
    pub fn new(width: u32, height: u32, config: crate::Config) -> Self {
        let sampler = Sampler {
            filter_mode: Filter::Nearest,
        };
        let stars =
            Texture2D::from_bytes(include_bytes!("../../../textures/starmap_2020_4k.exr")).unwrap();

        Self {
            buffer: FrameBuffer::new(width, height),
            config,

            sampler,
            stars,
        }
    }

    #[profiling::function]
    pub fn compute(&mut self) {
        let view = self.config.camera.view();
        let fov = self.config.camera.fov().as_f32();

        let origin = view.translation.into();
        let res = Vec2::new(self.buffer.width() as f32, self.buffer.height() as f32);

        self.buffer.par_for_each(|coord| {
            let coord = if self.config.features.contains(Features::AA) {
                aa_filter(coord)
            } else {
                coord
            };

            let mut uv = 2.0 * (coord - 0.5 * res) / f32::max(res.x, res.y);

            if self.config.features.contains(Features::BLOOM) {
                let r = rand();
                if r < 0.10 {
                    uv = nrand2(uv, rand() * 0.015);
                } else if r > 0.90 {
                    uv = nrand2(uv, rand() * 0.200);
                }
            }

            let ro = view.transform_vector3(origin);
            let rd = view
                .transform_vector3((uv * 2.0 * fov * FRAC_1_PI).extend(-1.0))
                .normalize();

            let mut acc = Vec3::ZERO;

            for _ in 0..self.config.samples {
                let col = render(ro, rd, self.sampler, &self.stars, &self.config);

                let col = if col.cmplt(Vec3::ZERO).any() || !col.is_finite() || col.is_nan() {
                    Vec3::ZERO
                } else {
                    col
                };

                acc += col;
            }

            let avg = acc / self.config.samples as f32;

            // gamma correction
            let avg = avg.powf(0.45);

            avg.extend(1.0)
        });
    }

    #[profiling::function]
    pub fn into_frame(self) -> Vec<u8> {
        self.buffer.into_vec()
    }
}
