mod hardware;
mod software;

pub use hardware::Hardware;

pub struct Config {
    pub width: u32,
    pub height: u32,
    pub fov: f32,
    pub samples: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fov: 90.0_f32.to_radians(),
            samples: 1,
        }
    }
}

pub trait Simulator {
    type Encoder;

    fn update(&mut self, config: Config);
    fn record(&mut self, enc: &mut Self::Encoder);
    fn into_frame(self, enc: &mut Self::Encoder) -> Vec<u8>;
}
