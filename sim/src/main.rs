use event::EventHandler;
use graphics::wgpu;
use winit::event_loop::EventLoop;

struct State {}

impl State {
    fn new<T>(event_loop: &EventLoop<T>, ctx: &graphics::Context) -> Self {
        Self {}
    }
}

impl EventHandler for State {
    fn update(&mut self, ctx: &mut event::Context) {}

    fn draw(
        &mut self,
        ctx: &mut event::Context,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = event::EventLoopBuilder::with_user_event().build()?;
    let window = winit::window::WindowBuilder::new().with_title("Orbital");

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
