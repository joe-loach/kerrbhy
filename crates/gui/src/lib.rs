mod renderer;
mod state;

use std::sync::Arc;

pub use egui;
use egui::{
    epaint::{
        self,
        ClippedShape,
    },
    FullOutput,
    TexturesDelta,
};
use graphics::wgpu;
use wgpu::RenderPassDescriptor;
use winit::window::Window;

struct PartialOutput {
    textures_delta: TexturesDelta,
    shapes: Vec<ClippedShape>,
}

pub struct Gui {
    window: Arc<Window>,
    renderer: renderer::Renderer,
    state: state::State,

    // keep state over update/draw calls
    pixels_per_point: f32,
    partial: Option<PartialOutput>,
}

impl Gui {
    pub fn new(ctx: &graphics::Context) -> Self {
        let window = ctx.window().unwrap();
        let pixels_per_point = window.scale_factor() as f32;

        let context = egui::Context::default();
        let viewport = context.viewport_id();

        let state = state::State::new(
            context,
            viewport,
            &window,
            Some(pixels_per_point),
            Some(ctx.device().limits().max_texture_dimension_2d as usize),
        );

        let renderer = renderer::Renderer::new(&ctx.device(), ctx.view_format().unwrap(), None, 1);

        Self {
            window,
            renderer,
            state,
            pixels_per_point,
            partial: None,
        }
    }

    pub fn context(&self) -> egui::Context {
        self.state.egui_ctx().clone()
    }

    pub fn begin(&mut self) -> egui::Context {
        // update state
        // state::update_viewport_info(viewport_info, &self.context(), &self.window);
        let raw = self.state.take_egui_input(&self.window);

        let ctx = self.context();
        ctx.begin_frame(raw);
        ctx
    }

    pub fn end(&mut self) {
        let FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            // Ignore viewports for the time being, they're not used!
            viewport_output: _,
        } = self.context().end_frame();

        self.pixels_per_point = pixels_per_point;

        self.state
            .handle_platform_output(&self.window, platform_output);

        self.partial = Some(PartialOutput {
            textures_delta,
            shapes,
        });
    }

    pub fn draw(
        &mut self,
        ctx: &event::State,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        let device = &ctx.device();
        let queue = &ctx.queue();

        let PartialOutput {
            textures_delta,
            shapes,
        } = self
            .partial
            .take()
            .expect("need to call `update` before `draw`");

        for (id, delta) in textures_delta.set {
            self.renderer.update_texture(device, queue, id, &delta);
        }

        let paint_jobs = self.context().tessellate(shapes, self.pixels_per_point);

        let surface = ctx.surface();
        let screen_descriptor = &renderer::ScreenDescriptor {
            size_in_pixels: [surface.width, surface.height],
            pixels_per_point: self.pixels_per_point,
        };

        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            paint_jobs.as_slice(),
            screen_descriptor,
        );

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("gui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.renderer
                .render(&mut pass, paint_jobs.as_slice(), screen_descriptor);
        }

        for id in textures_delta.free {
            self.renderer.free_texture(&id);
        }
    }

    pub fn handle_event<T>(&mut self, event: &event::Event<T>) -> bool {
        if let event::Event::Window(window_event) = event {
            let response = self.state.on_window_event(&self.window, window_event);
            response.consumed
        } else {
            false
        }
    }

    /// Fetches text from the clipboard and returns it.
    pub fn clipboard_text(&mut self) -> Option<String> {
        self.state.clipboard_text()
    }

    /// Places the text onto the clipboard.
    pub fn set_clipboard_text(&mut self, text: String) {
        self.state.set_clipboard_text(text);
    }

    /// Get the WGPU texture and bind group associated to a texture that has
    /// been allocated by egui.
    ///
    /// This could be used by custom paint hooks to render images that have been
    /// added through [`epaint::Context::load_texture`](https://docs.rs/egui/latest/egui/struct.Context.html#method.load_texture).
    pub fn texture(
        &self,
        id: &epaint::TextureId,
    ) -> Option<&(Option<wgpu::Texture>, wgpu::BindGroup)> {
        self.renderer.texture(id)
    }

    /// Registers a `wgpu::Texture` with a `epaint::TextureId`.
    ///
    /// This enables the application to reference the texture inside an image ui
    /// element. This effectively enables off-screen rendering inside the
    /// egui UI. Texture must have the texture format
    /// `TextureFormat::Rgba8UnormSrgb` and Texture usage
    /// `TextureUsage::SAMPLED`.
    pub fn register_native_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        texture_filter: wgpu::FilterMode,
    ) -> epaint::TextureId {
        self.renderer
            .register_native_texture(device, texture, texture_filter)
    }

    /// Registers a `wgpu::Texture` with an existing `epaint::TextureId`.
    ///
    /// This enables applications to reuse `TextureId`s.
    pub fn update_egui_texture_from_wgpu_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        texture_filter: wgpu::FilterMode,
        id: epaint::TextureId,
    ) {
        self.renderer
            .update_egui_texture_from_wgpu_texture(device, texture, texture_filter, id)
    }

    /// Registers a `wgpu::Texture` with a `epaint::TextureId` while also
    /// accepting custom `wgpu::SamplerDescriptor` options.
    ///
    /// This allows applications to specify individual
    /// minification/magnification filters as well as custom mipmap and
    /// tiling options.
    ///
    /// The `Texture` must have the format `TextureFormat::Rgba8UnormSrgb` and
    /// usage `TextureUsage::SAMPLED`. Any compare function supplied in the
    /// `SamplerDescriptor` will be ignored.
    pub fn register_native_texture_with_sampler_options(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        sampler_descriptor: wgpu::SamplerDescriptor<'_>,
    ) -> epaint::TextureId {
        self.renderer.register_native_texture_with_sampler_options(
            device,
            texture,
            sampler_descriptor,
        )
    }

    /// Registers a `wgpu::Texture` with an existing `epaint::TextureId` while
    /// also accepting custom `wgpu::SamplerDescriptor` options.
    ///
    /// This allows applications to reuse `TextureId`s created with custom
    /// sampler options.
    pub fn update_egui_texture_from_wgpu_texture_with_sampler_options(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        sampler_descriptor: wgpu::SamplerDescriptor<'_>,
        id: epaint::TextureId,
    ) {
        self.renderer
            .update_egui_texture_from_wgpu_texture_with_sampler_options(
                device,
                texture,
                sampler_descriptor,
                id,
            )
    }
}
