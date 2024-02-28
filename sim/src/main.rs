mod gui;

use event::EventHandler;
use fullscreen::Fullscreen;
use graphics::wgpu;
use gui::GuiState;
use hardware_renderer::*;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::WindowBuilder,
};

struct App {
    renderer: Renderer,
    fullscreen: Fullscreen,
    gui: GuiState,

    accumulate: bool,
    config: Config,
}

impl App {
    fn new<T>(_event_loop: &EventLoop<T>, ctx: &graphics::Context) -> Self {
        let renderer = Renderer::new(ctx);
        let fullscreen = Fullscreen::new(ctx);
        let gui = GuiState::new(ctx);

        gui.context().style_mut(|style| {
            style.visuals.window_shadow = egui::epaint::Shadow::NONE;
            style.visuals.window_rounding = egui::Rounding::ZERO;
        });

        Self {
            renderer,
            fullscreen,
            gui,

            accumulate: true,
            config: Config::default(),
        }
    }

    #[profiling::function]
    fn ui(&mut self, ctx: egui::Context, state: &mut event::State) {
        let mut vsync = state.is_vsync();
        egui::Window::new("Info").show(&ctx, |ui| {
            ui.checkbox(&mut vsync, "vsync");
            ui.checkbox(&mut self.accumulate, "accumulate");

            ui.horizontal(|ui| {
                ui.label("Fov: ");
                ui.drag_angle(&mut self.config.fov);
            });
            ui.add(
                egui::DragValue::new(&mut self.config.disk_radius)
                    .speed(0.1)
                    .prefix("Disk radius: ")
                    .clamp_range(0.0..=10.0),
            );
            ui.add(
                egui::DragValue::new(&mut self.config.disk_height)
                    .speed(0.1)
                    .prefix("Disk height: ")
                    .clamp_range(0.0..=10.0),
            );

            let on = !ui
                .collapsing("Profiler", |ui| {
                    profiling::scope!("profiler");
                    puffin_egui::profiler_ui(ui);
                })
                .fully_closed();

            puffin::set_scopes_on(on);
        });
        state.set_vsync(vsync);
    }
}

impl EventHandler for App {
    fn update(&mut self, state: &mut event::State) {
        let (width, height) = state.dimensions();

        self.renderer.update(width, height, self.config);

        let ctx = self.gui.begin();
        self.ui(ctx, state);
        self.gui.end();
    }

    fn draw(
        &mut self,
        state: &mut event::State,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        // only compute more work when it's needed
        if self.accumulate || self.renderer.must_render() {
            self.renderer.compute(Some(encoder));
        }

        self.fullscreen.draw(encoder, &self.renderer.view(), target);

        self.gui.draw(state, encoder, target);
    }

    fn event(&mut self, event: event::Event<()>) -> bool {
        self.gui.handle_event(&event)
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = event::EventLoopBuilder::with_user_event().build()?;
    let window = WindowBuilder::new().with_title("Kerrbhy");

    let window = window
        .with_inner_size(PhysicalSize::new(600, 600))
        .with_min_inner_size(PhysicalSize::new(400, 400));

    let cb = graphics::ContextBuilder::new(
        |adapter| adapter.features(),
        wgpu::Limits::downlevel_defaults(),
    )
    .with_window(window);

    let app = |event_loop: &_, ctx: &_| App::new(event_loop, ctx);

    event::run(event_loop, cb, app)?;

    Ok(())
}
