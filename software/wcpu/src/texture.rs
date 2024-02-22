use glam::{
    UVec2,
    Vec2,
    Vec4,
};

pub type Texture1D = Texture<1>;
pub type Texture2D = Texture<2>;

pub struct Texture<const DIM: u32> {
    img: image::Rgba32FImage,
}

impl<const DIM: u32> Texture<DIM> {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
        assert!(DIM > 0 && DIM <= 2, "Incorrect dimensions");

        let dyn_img = image::load_from_memory(bytes)?;

        Ok(Self {
            img: dyn_img.into_rgba32f(),
        })
    }
}

impl Texture<1> {
    pub fn size(&self) -> u32 {
        self.img.width()
    }

    pub fn get(&self, x: u32) -> Vec4 {
        pixel_to_vec(*self.img.get_pixel(x, 0))
    }

    pub fn get_checked(&self, x: u32) -> Option<Vec4> {
        self.img.get_pixel_checked(x, 0).copied().map(pixel_to_vec)
    }
}

impl Texture<2> {
    pub fn size(&self) -> UVec2 {
        self.img.dimensions().into()
    }

    pub fn get(&self, x: u32, y: u32) -> Vec4 {
        pixel_to_vec(*self.img.get_pixel(x, y))
    }

    pub fn get_checked(&self, x: u32, y: u32) -> Option<Vec4> {
        self.img.get_pixel_checked(x, y).copied().map(pixel_to_vec)
    }
}

fn pixel_to_vec(pixel: image::Rgba<f32>) -> Vec4 {
    Vec4::from_array(pixel.0)
}

#[derive(Clone, Copy)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Clone, Copy)]
pub struct Sampler {
    pub filter_mode: Filter,
}

pub trait Sample<const D: u32> {
    type Point;

    fn sample(&self, tex: &Texture<D>, uv: Self::Point) -> Vec4;
}

impl Sample<1> for Sampler {
    type Point = f32;

    fn sample(&self, tex: &Texture<1>, uv: Self::Point) -> Vec4 {
        let pos = uv * tex.size() as f32;

        match self.filter_mode {
            Filter::Nearest => {
                let x = pos.round();
                tex.get(x as u32)
            }
            Filter::Linear => {
                unimplemented!()
            }
        }
    }
}

impl Sample<2> for Sampler {
    type Point = Vec2;

    fn sample(&self, tex: &Texture<2>, uv: Self::Point) -> Vec4 {
        let pos = uv * tex.size().as_vec2();

        match self.filter_mode {
            Filter::Nearest => {
                let Vec2 { x, y } = pos.round();

                tex.get_checked(x as u32, y as u32).unwrap_or(Vec4::ZERO)
            }
            Filter::Linear => {
                let Vec2 { x, y } = pos;

                let x1 = x.floor();
                let y1 = y.floor();
                let x2 = x.ceil();
                let y2 = y.ceil();

                let q11 = tex.get(x1 as u32, y1 as u32);
                let q12 = tex.get(x1 as u32, y2 as u32);
                let q21 = tex.get(x2 as u32, y1 as u32);
                let q22 = tex.get(x2 as u32, y2 as u32);

                (q11 * (x2 - x) * (y2 - y)
                    + q21 * (x - x1) * (y2 - y)
                    + q12 * (x2 - x) * (y - y1)
                    + q22 * (x - x1) * (y - y1))
                    / (x2 - x1)
                    * (y2 - y1)
            }
        }
    }
}
