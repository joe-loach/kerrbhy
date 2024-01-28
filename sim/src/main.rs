mod fullscreen;

use event::EventHandler;
use fullscreen::Fullscreen;
use graphics::wgpu;
use gui::{
    egui,
    Gui,
};
use marcher::Marcher;
use winit::event_loop::EventLoop;

struct State {
    marcher: Marcher,
    fullscreen: Fullscreen,
    gui: Gui,
}

impl State {
    fn new<T>(_event_loop: &EventLoop<T>, ctx: &graphics::Context) -> Self {
        let marcher = Marcher::new(ctx.device());
        let fullscreen = Fullscreen::new(ctx);
        let gui = Gui::new(ctx);
        Self {
            marcher,
            fullscreen,
            gui,
        }
    }

    fn ui(&mut self, ctx: egui::Context) {
        egui::Window::new("Kerrbhy").show(&ctx, |ui| {
            ui.label("Hey");
        });
    }
}

impl EventHandler for State {
    fn update(&mut self, ctx: &mut event::Context) {
        let (width, height) = ctx.dimensions();
        self.marcher.update(ctx.device(), width, height);

        {
            let ctx = self.gui.begin();
            self.ui(ctx);
        }
        self.gui.end(ctx);
    }

    fn draw(
        &mut self,
        ctx: &mut event::Context,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        self.marcher.draw(ctx, encoder);
        self.fullscreen
            .draw(ctx.device(), encoder, &self.marcher.view(), target);
        self.gui.draw(ctx, encoder, target);
    }

    fn event(&mut self, event: event::Event<()>) -> bool {
        self.gui.handle_event(&event)
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = event::EventLoopBuilder::with_user_event().build()?;
    let window = winit::window::WindowBuilder::new().with_title("Kerrbhy");

    let window = window.with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080));

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
