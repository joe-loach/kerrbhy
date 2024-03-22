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
    /// Create a new [`FrameBuffer`] of `width` and `height`.
    #[inline]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: image::ImageBuffer::new(width, height),
            width,
            height,
        }
    }

    /// Iterates through each pixel in the [`FrameBuffer`].
    /// 
    /// For each pixel, it calls a function (id, color) and expects you to return an updated color.
    #[inline]
    pub fn for_each(&mut self, f: impl Fn(UVec2, Vec4) -> Vec4) {
        for (x, y, p) in self.buffer.enumerate_pixels_mut() {
            let color = f(UVec2::new(x, y), Vec4::from_array(p.0));

            *p = image::Rgba(color.to_array());
        }
    }

    /// Iterates through each pixel in the [`FrameBuffer`] in parallel.
    /// 
    /// For each pixel, it calls a function (id, color) and expects you to return an updated color.
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

    /// Width of the [`FrameBuffer`].
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height of the [`FrameBuffer`].
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Converts this [`FrameBuffer`] into an array of bytes `[r, g, b, a]`.
    pub fn into_vec(self) -> Vec<u8> {
        use image::buffer::ConvertBuffer;

        let buffer: image::RgbaImage = self.buffer.convert();
        buffer.into_vec()
    }
}
