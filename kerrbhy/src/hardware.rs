use std::sync::Arc;

use graphics::wgpu;

use crate::{
    Config,
    Simulator,
};

pub struct Hardware {
    marcher: marcher::Marcher,
    dirty: bool,
}

impl Hardware {
    pub fn new(device: Arc<wgpu::Device>, queue: &wgpu::Queue) -> Self {
        let marcher = marcher::Marcher::new(device, queue);

        Self {
            marcher,
            dirty: true,
        }
    }

    pub fn must_render(&self) -> bool {
        self.dirty
    }
}

impl Simulator for Hardware {
    type Buffer = wgpu::TextureView;
    type Encoder = wgpu::CommandEncoder;

    fn update(&mut self, config: Config) {
        self.dirty = self.marcher.update(config.width, config.height, config.fov);
    }

    fn record(&mut self, enc: &mut Self::Encoder) {
        self.marcher.record(enc);
    }

    fn get_frame(&self) -> Self::Buffer {
        self.marcher.view()
    }
}

impl crate::Buffer for wgpu::TextureView {
    fn to_bytes(&self) -> &[u8] {
        todo!()
    }
}
