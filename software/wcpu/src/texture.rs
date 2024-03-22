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
    /// Loads an Rgba texture from bytes in memory.
    #[profiling::function]
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
pub enum EdgeMode {
    Wrap,
}

impl EdgeMode {
    pub fn apply2d(&self, tex: &Texture2D, x: u32, y: u32) -> (u32, u32) {
        let size = tex.size();
        match self {
            EdgeMode::Wrap => (x % size.x, y % size.y),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Sampler {
    /// What filter is applied to each point.
    pub filter_mode: Filter,
    /// What the sampler does at the edge of a texture
    pub edge_mode: EdgeMode,
}

/// Allows samplers to Sample [`Textures`](Texture) of dimension `D`, using different types of points.
pub trait Sample<const D: u32> {
    /// The type of query point.
    type Point;

    /// Samples a [`Texture`] and returns the color at that point.
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
        let size = tex.size();
        let pos = uv * size.as_vec2();

        match self.filter_mode {
            Filter::Nearest => {
                let Vec2 { x, y } = pos.round();

                let (x, y) = self.edge_mode.apply2d(tex, x as u32, y as u32);

                tex.get(x, y)
            }
            Filter::Linear => {
                let Vec2 { x, y } = pos;

                let x1 = x.floor();
                let y1 = y.floor();
                let x2 = x.ceil();
                let y2 = y.ceil();

                let (q11, q12, q21, q22) = {
                    let (x1, y1) = self.edge_mode.apply2d(tex, x1 as u32, y1 as u32);
                    let (x2, y2) = self.edge_mode.apply2d(tex, x2 as u32, y2 as u32);

                    (
                        tex.get(x1, y1),
                        tex.get(x1, y2),
                        tex.get(x2, y1),
                        tex.get(x2, y2),
                    )
                };

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
