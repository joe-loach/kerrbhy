mod fullscreen;

use event::EventHandler;
use fullscreen::Fullscreen;
use graphics::wgpu;
use gui::{
    egui,
    Gui,
};
use kerrbhy::Simulator;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::WindowBuilder,
};

struct State {
    renderer: kerrbhy::Hardware,
    fullscreen: Fullscreen,
    gui: Gui,

    accumulate: bool,
    fov: f32,
}

impl State {
    fn new<T>(_event_loop: &EventLoop<T>, ctx: &graphics::Context) -> Self {
        let renderer = kerrbhy::Hardware::new(ctx.device(), &ctx.queue());
        let fullscreen = Fullscreen::new(ctx);
        let gui = Gui::new(ctx);

        gui.context().style_mut(|style| {
            style.visuals.window_shadow = egui::epaint::Shadow::NONE;
            style.visuals.window_rounding = egui::Rounding::ZERO;
        });

        Self {
            renderer,
            fullscreen,
            gui,

            accumulate: true,
            fov: 90.0_f32.to_radians(),
        }
    }

    fn ui(&mut self, ctx: egui::Context, state: &event::State) {
        egui::Window::new("Config").show(&ctx, |ui| {
            ui.label(format!("\r{:0>8} FPS", 1.0 / state.timer().dt()));

            ui.horizontal(|ui| {
                ui.label("Fov: ");
                ui.drag_angle(&mut self.fov);
            });
            ui.checkbox(&mut self.accumulate, "Accumulate?");
        });
    }
}

impl EventHandler for State {
    fn update(&mut self, state: &event::State) {
        let (width, height) = state.dimensions();

        self.renderer.update(kerrbhy::Config {
            width,
            height,
            fov: self.fov,
            ..Default::default()
        });

        let ctx = self.gui.begin();
        self.ui(ctx, state);
        self.gui.end();
    }

    fn draw(
        &mut self,
        ctx: &mut event::State,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        // only compute more work when it's needed
        if self.accumulate || self.renderer.must_render() {
            self.renderer.record(encoder);
        }
        self.fullscreen
            .draw(encoder, &self.renderer.get_frame(), target);
        self.gui.draw(ctx, encoder, target);
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

    let ctx = graphics::Context::new(
        &event_loop,
        window,
        |adapter| adapter.features(),
        wgpu::Limits::downlevel_defaults(),
    )?;

    let state = State::new(&event_loop, &ctx);

    event::run(event_loop, ctx, state)?;

    Ok(())
}
