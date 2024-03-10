mod gui;
mod input;

use event::EventHandler;
use fullscreen::Fullscreen;
use glam::vec3;
use graphics::wgpu;
use gui::GuiState;
use hardware_renderer::*;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    keyboard::KeyCode,
    window::WindowBuilder,
};

struct App {
    renderer: Renderer,
    fullscreen: Fullscreen,
    gui: GuiState,

    mouse: input::Mouse,
    keyboard: input::Keyboard,

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

            mouse: input::Mouse::new(),
            keyboard: input::Keyboard::new(),

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

            config_ui(ui, &mut self.config);

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

fn config_ui(ui: &mut egui::Ui, cfg: &mut Config) {
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Fov: ");
        ui.drag_angle(&mut cfg.fov);
    });
    ui.vertical(|ui| {
        ui.add(
            egui::DragValue::new(&mut cfg.disk_radius)
                .speed(0.1)
                .prefix("Disk radius: ")
                .clamp_range(0.0..=10.0),
        );
        ui.add(
            egui::DragValue::new(&mut cfg.disk_height)
                .speed(0.1)
                .prefix("Disk height: ")
                .clamp_range(0.0..=10.0),
        );
    });

    ui.separator();
}

impl EventHandler for App {
    fn update(&mut self, state: &mut event::State) {
        let (width, height) = state.dimensions();

        let dt = state.timer().dt();

        if self.keyboard.is_down(KeyCode::KeyW) {
            self.config.pos += vec3(0.0, 0.0, -1.0) * dt;
        }
        if self.keyboard.is_down(KeyCode::KeyS) {
            self.config.pos += vec3(0.0, 0.0, 1.0) * dt;
        }
        if self.keyboard.is_down(KeyCode::KeyA) {
            self.config.pos += vec3(-1.0, 0.0, 0.0) * dt;
        }
        if self.keyboard.is_down(KeyCode::KeyD) {
            self.config.pos += vec3(1.0, 0.0, 0.0) * dt;
        }
        if self.keyboard.is_down(KeyCode::Space) {
            self.config.pos += vec3(0.0, 1.0, 0.0) * dt;
        }
        if self.keyboard.is_down(KeyCode::ControlLeft) {
            self.config.pos += vec3(0.0, -1.0, 0.0) * dt;
        }

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
        let consumed = self.gui.handle_event(&event);

        if !consumed {
            self.mouse.update_state(&event);
            self.keyboard.update_state(&event);
        }

        consumed
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
