use crate::Simulator;

pub struct Software {
    buffer: image::RgbaImage,
    config: crate::Config,
}

impl Simulator for Software {
    type Encoder = ();

    fn update(&mut self, config: crate::Config) {
        self.config = config;
    }

    fn record(&mut self, _: &mut Self::Encoder) {
        let iter = self.buffer.enumerate_pixels_mut();
    }

    fn into_frame(self, _: &mut Self::Encoder) -> Vec<u8> {
        self.buffer.into_vec()
    }
}
