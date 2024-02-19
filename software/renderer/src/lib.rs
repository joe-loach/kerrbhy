use glam::{
    Vec2,
    Vec3,
    Vec4,
    Vec4Swizzles,
};
use image::GenericImageView;

pub struct Params {
    pub width: u32,
    pub height: u32,
    pub fov: f32,
    pub samples: u32,
}

pub struct Renderer {
    buffer: image::RgbaImage,
    config: Params,

    stars: image::DynamicImage,
}

const MAX_STEPS: u32 = 128;
const DELTA: f32 = 0.05;
const BLACKHOLE_RADIUS: f32 = 0.6;
const SKYBOX_RADIUS: f32 = 3.6;

#[allow(clippy::excessive_precision)]
const M_1_PI: f32 = 1.0 / 3.1415926535897932384626433832795;
#[allow(clippy::excessive_precision)]
const M_1_2PI: f32 = 1.0 / 6.283185307179586476925286766559;

fn tof32(x: u8) -> f32 {
    x as f32 / 255.0
}

fn tou8(x: f32) -> u8 {
    (x / 255.0) as u8
}

/// sample a texture bilinearly
fn sample(tex: &image::DynamicImage, coord: Vec2) -> Vec4 {
    let x = coord.x;
    let y = coord.x;

    let x1 = coord.x.floor();
    let y1 = coord.y.floor();
    let x2 = coord.x.ceil();
    let y2 = coord.x.ceil();

    let q11 = Vec4::from_array(tex.get_pixel(x1 as u32, y1 as u32).0.map(tof32));
    let q12 = Vec4::from_array(tex.get_pixel(x1 as u32, y2 as u32).0.map(tof32));
    let q21 = Vec4::from_array(tex.get_pixel(x2 as u32, y1 as u32).0.map(tof32));
    let q22 = Vec4::from_array(tex.get_pixel(x2 as u32, y2 as u32).0.map(tof32));

    (q11 * (x2 - x) * (y2 - y)
        + q21 * (x - x1) * (y2 - y)
        + q12 * (x2 - x) * (y - y1)
        + q22 * (x - x1) * (y - y1))
        / (x2 - x1)
        * (y2 - y1)
}

fn gravitational_field(p: Vec3) -> Vec3 {
    let r = p / BLACKHOLE_RADIUS;
    let rn = r.length();
    -6.0 * r / (rn * rn * rn * rn * rn)
}

fn sky(stars: &image::DynamicImage, rd: Vec3) -> Vec3 {
    let coord = Vec2::new(
        0.5 - (rd.z.atan2(rd.x) * M_1_2PI),
        0.5 - ((-rd.y).asin() * M_1_PI),
    );

    sample(stars, coord).xyz()
}

fn render(ro: Vec3, rd: Vec3, stars: &image::DynamicImage) -> Vec3 {
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

    r += sky(stars, v.normalize());

    r
}

impl Renderer {
    pub fn new(config: crate::Params) -> Self {
        let star_data = include_bytes!("../../../textures/starmap_2020_4k.exr");
        let stars = image::load_from_memory(star_data).unwrap();

        Self {
            buffer: image::ImageBuffer::new(config.width, config.height),
            config,

            stars,
        }
    }

    pub fn compute(&mut self) {
        let res = Vec2::new(self.config.width as f32, self.config.height as f32);

        for (x, y, pixels) in self.buffer.enumerate_pixels_mut() {
            let coord = Vec2::new(x as f32, y as f32);
            let uv = 2.0 * (coord - 0.5 * res) / res.x.max(res.y);

            let ro = Vec3::new(0.0, 0.2, 3.3);
            let rd = (uv * 2.0 * self.config.fov * M_1_PI)
                .extend(-1.0)
                .normalize();

            let mut acc = Vec3::ZERO;

            for _ in 0..self.config.samples {
                acc += render(ro, rd, &self.stars);
            }

            let [r, g, b] = (acc / self.config.samples as f32).to_array();

            *pixels = image::Rgba([tou8(r), tou8(g), tou8(b), 255]);
        }
    }

    pub fn into_frame(self) -> Vec<u8> {
        self.buffer.into_vec()
    }
}
