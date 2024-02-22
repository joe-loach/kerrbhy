use std::f32::consts::FRAC_1_PI;

use glam::{
    Vec2,
    Vec3,
    Vec4Swizzles,
};
use wcpu::{
    texture::Filter,
    FrameBuffer,
    Sample,
    Sampler,
    Texture2D,
};

pub struct Params {
    pub width: u32,
    pub height: u32,
    pub fov: f32,
    pub samples: u32,
}

pub struct Renderer {
    buffer: FrameBuffer,
    config: Params,

    sampler: Sampler,
    stars: Texture2D,
}

const MAX_STEPS: u32 = 128;
const DELTA: f32 = 0.05;
const BLACKHOLE_RADIUS: f32 = 0.6;
const SKYBOX_RADIUS: f32 = 3.6;

const FRAC_1_2PI: f32 = FRAC_1_PI * 0.5;

fn gravitational_field(p: Vec3) -> Vec3 {
    let r = p / BLACKHOLE_RADIUS;
    let rn = r.length();
    -6.0 * r / (rn * rn * rn * rn * rn)
}

fn sky(sampler: Sampler, stars: &Texture2D, rd: Vec3) -> Vec3 {
    let coord = Vec2::new(
        0.5 - (f32::atan2(rd.z, rd.x) * FRAC_1_2PI),
        0.5 - (f32::asin(-rd.y) * FRAC_1_PI),
    );

    sampler.sample(stars, coord).xyz()
}

fn render(ro: Vec3, rd: Vec3, sampler: Sampler, stars: &Texture2D) -> Vec3 {
    let mut r = Vec3::ZERO;

    let mut p = ro;
    let mut v = rd;

    for _ in 0..MAX_STEPS {
        if p.length_squared() < BLACKHOLE_RADIUS * BLACKHOLE_RADIUS {
            return r;
        }

        if p.length_squared() > SKYBOX_RADIUS * SKYBOX_RADIUS {
            break;
        }

        // TODO: use RK4
        let g = gravitational_field(p);
        v += g * DELTA;
        p += v * DELTA;
    }

    r += sky(sampler, stars, v.normalize());

    r
}

impl Renderer {
    pub fn new(config: crate::Params) -> Self {
        let sampler = Sampler {
            filter_mode: Filter::Nearest,
        };
        let stars =
            Texture2D::from_bytes(include_bytes!("../../../textures/starmap_2020_4k.exr")).unwrap();

        Self {
            buffer: FrameBuffer::new(config.width, config.height),
            config,

            sampler,
            stars,
        }
    }

    pub fn compute(&mut self) {
        let origin = Vec3::new(0.0, 0.2, 3.3);
        let res = Vec2::new(self.config.width as f32, self.config.height as f32);

        self.buffer.for_each(|coord| {
            let uv = 2.0 * (coord - 0.5 * res) / f32::max(res.x, res.y);

            let ro = origin;
            let rd = (uv * 2.0 * self.config.fov * FRAC_1_PI)
                .extend(-1.0)
                .normalize();

            let mut acc = Vec3::ZERO;

            for _ in 0..self.config.samples {
                acc += render(ro, rd, self.sampler, &self.stars);
            }

            let avg = acc / self.config.samples as f32;
            let avg = avg.powf(0.45);

            avg.extend(1.0)
        });
    }

    pub fn into_frame(self) -> Vec<u8> {
        self.buffer.into_vec()
    }
}
