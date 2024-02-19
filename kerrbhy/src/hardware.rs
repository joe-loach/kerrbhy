use std::sync::Arc;

use graphics::wgpu;

use crate::Config;

pub struct Hardware {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    marcher: marcher::Marcher,
    encoder: Option<wgpu::CommandEncoder>,

    dirty: bool,
}

impl Hardware {
    pub fn new(ctx: &graphics::Context) -> Self {
        let device = ctx.device();
        let queue = ctx.queue();

        let marcher = marcher::Marcher::new(device.clone(), &queue);

        let encoder = if ctx.is_headless() {
            Some(device.create_command_encoder(&Default::default()))
        } else {
            None
        };

        Self {
            device,
            queue,
            marcher,
            encoder,

            dirty: true,
        }
    }

    pub fn must_render(&self) -> bool {
        self.dirty
    }

    pub fn view(&self) -> wgpu::TextureView {
        self.marcher.view()
    }

    pub fn update(&mut self, config: Config) {
        self.dirty = self.marcher.update(config.width, config.height, config.fov);
    }

    pub fn compute(&mut self, encoder: Option<&mut wgpu::CommandEncoder>) {
        let encoder = self.encoder.as_mut().or(encoder).expect("no encoder");

        self.marcher.record(encoder);
    }

    pub fn into_frame(self, encoder: Option<wgpu::CommandEncoder>) -> Vec<u8> {
        let mut encoder = self.encoder.or(encoder).expect("no encoder");

        let width = self.marcher.texture().width();
        let height = self.marcher.texture().height();
        let block_size = self
            .marcher
            .texture()
            .format()
            .block_copy_size(None)
            .unwrap();

        let frame = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: width as u64 * height as u64 * block_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            self.marcher.texture().as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &frame,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * block_size),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // submit the commands to finish the work before reading
        self.queue.submit(Some(encoder.finish()));

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
            let result = data.as_ref().to_vec();

            // get rid of the buffer from the CPU.
            drop(data);
            frame.unmap();

            result
        } else {
            panic!("failed to read frame from gpu")
        }
    }
}
