use std::sync::Arc;

use graphics::wgpu;

use crate::{
    Config,
    Simulator,
};

pub struct Hardware {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    marcher: marcher::Marcher,
    dirty: bool,
}

impl Hardware {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let marcher = marcher::Marcher::new(device.clone(), &queue);

        Self {
            device,
            queue,
            marcher,
            dirty: true,
        }
    }

    pub fn must_render(&self) -> bool {
        self.dirty
    }

    pub fn view(&self) -> wgpu::TextureView {
        self.marcher.view()
    }
}

impl Simulator for Hardware {
    type Encoder = wgpu::CommandEncoder;

    fn update(&mut self, config: Config) {
        self.dirty = self.marcher.update(config.width, config.height, config.fov);
    }

    fn record(&mut self, enc: &mut Self::Encoder) {
        self.marcher.record(enc);
    }

    fn into_frame(self, enc: &mut Self::Encoder) -> Vec<u8> {
        let width = self.marcher.buffer().width();
        let height = self.marcher.buffer().height();

        let frame = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: width as u64 * height as u64 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        enc.copy_texture_to_buffer(
            self.marcher.buffer().as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &frame,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let (tx, rx) = flume::bounded(1);

        // we want to read the entire buffer off of the gpu
        let slice = frame.slice(..);
        slice.map_async(wgpu::MapMode::Read, move |cb| tx.send(cb).unwrap());

        // we have to poll the device here ourselves,
        // because we're assuming there is no runtime polling for us
        self.device.poll(wgpu::Maintain::Wait).panic_on_timeout();

        // block until we get a result
        if let Ok(Ok(())) = rx.recv() {
            let data = slice.get_mapped_range();
            let result = bytemuck::cast_slice(&data).to_vec();

            // get rid of the buffer from the CPU.
            drop(data);
            frame.unmap();

            result
        } else {
            panic!("failed to read frame from gpu")
        }
    }
}
