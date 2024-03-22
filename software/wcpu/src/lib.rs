use glam::{
    UVec2,
    Vec4,
};
use rayon::prelude::*;

pub mod texture;

pub use texture::{
    Sample,
    Sampler,
    Texture1D,
    Texture2D,
};

pub struct FrameBuffer {
    buffer: image::Rgba32FImage,
    width: u32,
    height: u32,
}

impl FrameBuffer {
    #[inline]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: image::ImageBuffer::new(width, height),
            width,
            height,
        }
    }

    #[inline]
    pub fn for_each(&mut self, f: impl Fn(UVec2, Vec4) -> Vec4) {
        for (x, y, p) in self.buffer.enumerate_pixels_mut() {
            let color = f(UVec2::new(x, y), Vec4::from_array(p.0));

            *p = image::Rgba(color.to_array());
        }
    }

    #[profiling::function]
    #[inline]
    pub fn par_for_each(&mut self, f: impl (Fn(UVec2, Vec4) -> Vec4) + Sync) {
        self.buffer
            .enumerate_pixels_mut()
            .par_bridge()
            .for_each(|(x, y, p)| {
                let color = f(UVec2::new(x, y), Vec4::from_array(p.0));

                *p = image::Rgba(color.to_array());
            });
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn into_vec(self) -> Vec<u8> {
        use image::buffer::ConvertBuffer;

        let buffer: image::RgbaImage = self.buffer.convert();
        buffer.into_vec()
    }
}
