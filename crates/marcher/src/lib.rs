mod shader;

use graphics::wgpu::{
    self,
    ComputePassDescriptor,
    ComputePipeline,
    Texture,
    TextureDescriptor,
    TextureView,
};
use shader::bind_groups::*;

pub struct Marcher {
    pipeline: ComputePipeline,
    tex: Texture,
}

impl Marcher {
    pub fn new(device: &wgpu::Device) -> Self {
        let pipeline = shader::compute::create_comp_pipeline(device);

        let tex = device.create_texture(&texture_descriptor());

        Self { pipeline, tex }
    }

    pub fn update(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width != self.tex.width() || height != self.tex.height() {
            // recreate texture
            self.tex = device.create_texture(&TextureDescriptor {
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                ..texture_descriptor()
            });
        }
    }

    pub fn view(&self) -> TextureView {
        self.tex.create_view(&Default::default())
    }

    pub fn draw(&mut self, ctx: &mut event::Context, encoder: &mut wgpu::CommandEncoder) {
        let [width, height] = [self.tex.width(), self.tex.height()];

        let bindings = BindGroup0::from_bindings(
            ctx.device(),
            BindGroupLayout0 {
                out_tex: &self.view(),
            },
        );

        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        set_bind_groups(
            &mut pass,
            BindGroups {
                bind_group0: &bindings,
            },
        );

        let [x, y, _z] = shader::compute::COMP_WORKGROUP_SIZE;
        pass.dispatch_workgroups(width / x, height / y, 1);
    }
}

fn texture_descriptor() -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    }
}
