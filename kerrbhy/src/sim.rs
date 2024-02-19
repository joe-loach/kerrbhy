mod fullscreen;

use event::EventHandler;
use fullscreen::Fullscreen;
use graphics::wgpu;
use gui::{
    egui,
    Gui,
};
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
        let renderer = kerrbhy::Hardware::new(ctx);
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

    fn ui(&mut self, ctx: egui::Context, _state: &event::State) {
        egui::Window::new("Config").show(&ctx, |ui| {
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
            self.renderer.compute(Some(encoder));
        }
        self.fullscreen.draw(encoder, &self.renderer.view(), target);
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

    let cb = graphics::ContextBuilder::new(
        |adapter| adapter.features(),
        wgpu::Limits::downlevel_defaults(),
    )
    .with_window(window);

    let state = |event_loop: &_, ctx: &_| State::new(event_loop, ctx);

    event::run(event_loop, cb, state)?;

    Ok(())
}
