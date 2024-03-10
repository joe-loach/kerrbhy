use std::sync::Arc;

use graphics::wgpu;
use rayon::{
    iter::ParallelIterator,
    slice::ParallelSlice,
};

pub use common::Config;

pub struct Renderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    marcher: marcher::Marcher,
    encoder: Option<wgpu::CommandEncoder>,

    dirty: bool,
}

impl Renderer {
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

    #[profiling::function]
    pub fn update(&mut self, width: u32, height: u32, cfg: Config) {
        self.dirty = self.marcher.update(width, height, cfg);
    }

    #[profiling::function]
    pub fn compute(&mut self, encoder: Option<&mut wgpu::CommandEncoder>) {
        let encoder = self.encoder.as_mut().or(encoder).expect("no encoder");

        self.marcher.record(encoder);
    }

    #[profiling::function]
    pub fn into_frame(self, encoder: Option<wgpu::CommandEncoder>) -> Vec<u8> {
        let mut encoder = self.encoder.or(encoder).expect("no encoder");

        let (frame, row, aligned_row) = copy_texture_to_buffer(
            &self.device,
            &mut encoder,
            self.marcher.texture(),
            self.marcher.size(),
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

            let result = {
                profiling::scope!("Trimming image");
                // trim the edges of the data
                // to make sure that the resulting image is the correct size
                let whole_rows = data.par_chunks_exact(aligned_row as usize);
                whole_rows
                    .flat_map(|chunk| chunk.split_at(row as usize).0.to_vec())
                    .collect()
            };

            // get rid of the buffer from the CPU.
            drop(data);
            frame.unmap();

            result
        } else {
            panic!("failed to read frame from gpu")
        }
    }
}

#[profiling::function]
fn copy_texture_to_buffer(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    source_texture: &wgpu::Texture,
    size: wgpu::Extent3d,
) -> (wgpu::Buffer, u32, u32) {
    assert!(source_texture.dimension() == wgpu::TextureDimension::D2);

    let block_size = source_texture.format().block_copy_size(None).unwrap();
    let row = size.width * block_size;
    let aligned_row = pad_to(row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: aligned_row as u64 * size.height as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let source = wgpu::ImageCopyTexture {
        texture: source_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    };

    let destination = wgpu::ImageCopyBuffer {
        buffer: &buffer,
        layout: wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(aligned_row),
            rows_per_image: None,
        },
    };

    encoder.copy_texture_to_buffer(source, destination, size);

    (buffer, row, aligned_row)
}

fn pad_to(x: u32, y: u32) -> u32 {
    ((x + y - 1) / y) * y
}
