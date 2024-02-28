mod shader;

use std::sync::Arc;

use graphics::wgpu;

pub struct Fullscreen {
    device: Arc<wgpu::Device>,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl Fullscreen {
    pub fn new(ctx: &graphics::Context) -> Self {
        let device = ctx.device();

        let module = shader::create_shader_module(&device);
        let layout = shader::create_pipeline_layout(&device);
        let entry = shader::vert_entry();
        let vertex = shader::vertex_state(&module, &entry);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex,
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: shader::ENTRY_FRAG,
                targets: &[Some(wgpu::ColorTargetState::from(
                    ctx.view_format().unwrap(),
                ))],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Fullscreen {
            device,
            pipeline,
            sampler,
        }
    }

    #[profiling::function]
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
    ) {
        let binding = shader::bind_groups::BindGroup0::from_bindings(
            &self.device,
            shader::bind_groups::BindGroupLayout0 {
                color_texture: source,
                color_sampler: &self.sampler,
            },
        );

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        shader::set_bind_groups(&mut pass, &binding);
        // only need to draw 3 vertices
        pass.draw(0..3, 0..1);
    }
}
