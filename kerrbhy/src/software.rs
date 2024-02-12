use crate::Simulator;

pub struct Software {
    buffer: image::RgbaImage,
    config: crate::Config
}

impl Simulator for Software {
    type Buffer = Vec<u8>;

    type Encoder = ();

    fn update(&mut self, config: crate::Config) {
        self.config = config;
    }

    fn record(&mut self, _: &mut Self::Encoder) {
        let iter = self.buffer.enumerate_pixels_mut();
    }

    fn get_frame(&self) -> Self::Buffer {
        self.buffer.as_raw()
    }
}

impl crate::Buffer for Vec<u8> {
    fn to_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}