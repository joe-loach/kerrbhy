mod fullscreen;

use event::EventHandler;
use fullscreen::Fullscreen;
use graphics::wgpu;
use marcher::Marcher;
use winit::event_loop::EventLoop;

struct State {
    marcher: Marcher,
    fullscreen: Fullscreen,
}

impl State {
    fn new<T>(_event_loop: &EventLoop<T>, ctx: &graphics::Context) -> Self {
        let marcher = Marcher::new(ctx.device());
        let fullscreen = Fullscreen::new(ctx);

        Self {
            marcher,
            fullscreen,
        }
    }
}

impl EventHandler for State {
    fn update(&mut self, _ctx: &mut event::Context) {}

    fn draw(
        &mut self,
        ctx: &mut event::Context,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        self.marcher.draw(ctx, encoder);
        self.fullscreen
            .draw(ctx.device(), encoder, &self.marcher.view(), target);
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
