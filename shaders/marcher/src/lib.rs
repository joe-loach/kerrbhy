#[allow(clippy::approx_constant)]
mod shader;

use std::sync::Arc;

use graphics::wgpu::{
    self,
    util::DeviceExt,
    ComputePassDescriptor,
    ComputePipeline,
    Sampler,
    Texture,
    TextureDescriptor,
    TextureView,
};
use shader::bind_groups::*;

pub struct Params {
    pub width: u32,
    pub height: u32,
    pub origin: glam::Vec3,
    pub fov: f32,
    pub disk_radius: f32,
    pub disk_height: f32,
}

pub struct Marcher {
    device: Arc<wgpu::Device>,

    pipeline: ComputePipeline,

    stars: Texture,
    star_sampler: Sampler,

    fov: f32,
    disk_radius: f32,
    disk_height: f32,
    sample_no: u32,

    texture: Texture,
}

impl Marcher {
    #[profiling::function]
    pub fn new(device: Arc<wgpu::Device>, queue: &wgpu::Queue) -> Self {
        let pipeline = create_pipeline(&device);

        let stars = {
            profiling::scope!("loading textures");

            let star_data = include_bytes!("../../../textures/starmap_2020_4k.exr");
            let star_image = image::load_from_memory(star_data).unwrap();
            let star_bytes = star_image.to_rgba8();

            device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: star_image.width(),
                        height: star_image.height(),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::MipMajor,
                &star_bytes,
            )
        };
        let star_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture = device.create_texture(&buffer_texture_descriptor());

        let fov = (90.0_f32).to_radians();

        Self {
            device,
            pipeline,
            texture,
            stars,
            fov,
            disk_radius: 8.0,
            disk_height: 3.0,
            sample_no: 0,
            star_sampler,
        }
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> TextureView {
        self.texture.create_view(&Default::default())
    }

    pub fn size(&self) -> wgpu::Extent3d {
        self.texture().size()
    }

    #[profiling::function]
    pub fn update(&mut self, params: Params) -> bool {
        let Params {
            width,
            height,
            origin: _,
            fov,
            disk_radius,
            disk_height,
            ..
        } = params;

        let dimensions_changed = width != self.texture.width() || height != self.texture.height();
        let disk_changed = self.disk_radius != disk_radius || self.disk_height != disk_height;
        let fov_changed = self.fov != fov;

        self.disk_radius = disk_radius;
        self.disk_height = disk_height;
        self.fov = fov;

        let dirty = dimensions_changed || fov_changed || disk_changed;

        if dirty {
            self.recreate_buffer(width, height);
            self.sample_no = 0;
        }

        dirty
    }

    #[profiling::function]
    pub fn record(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let [width, height] = [self.texture.width(), self.texture.height()];

        let bind_group0 = BindGroup0::from_bindings(
            &self.device,
            BindGroupLayout0 {
                buffer: &self.view(),
            },
        );

        let bind_group1 = BindGroup1::from_bindings(
            &self.device,
            BindGroupLayout1 {
                star_sampler: &self.star_sampler,
                stars: &self.stars.create_view(&Default::default()),
            },
        );

        let push = shader::PushConstants {
            origin: glam::Vec3::new(0.0, 0.5, 3.3),
            fov: self.fov,
            sample: self.sample_no,
            disk_color: glam::Vec3::new(0.3, 0.2, 0.1),
            disk_radius: self.disk_radius,
            disk_height: self.disk_height,
            pad: glam::UVec2::ZERO,
        };

        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_push_constants(0, bytemuck::bytes_of(&push));
        shader::set_bind_groups(&mut pass, &bind_group0, &bind_group1);

        let [x, y, _z] = shader::compute::COMP_WORKGROUP_SIZE;
        let x = (width as f32 / x as f32).ceil() as u32;
        let y = (height as f32 / y as f32).ceil() as u32;
        pass.dispatch_workgroups(x, y, 1);

        self.sample_no += 1;
    }

    #[profiling::function]
    fn recreate_buffer(&mut self, width: u32, height: u32) {
        self.texture = self.device.create_texture(&TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            ..buffer_texture_descriptor()
        });
    }
}

fn create_pipeline(device: &wgpu::Device) -> ComputePipeline {
    let module = {
        let source = std::borrow::Cow::Borrowed(shader::SOURCE);
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source),
        })
    };
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            &BindGroup0::get_bind_group_layout(device),
            &BindGroup1::get_bind_group_layout(device),
        ],
        push_constant_ranges: &[wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::COMPUTE,
            range: 0..std::mem::size_of::<shader::PushConstants>() as u32,
        }],
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline comp"),
        layout: Some(&layout),
        module: &module,
        entry_point: "comp",
    })
}

fn buffer_texture_descriptor() -> wgpu::TextureDescriptor<'static> {
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
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    }
}
