mod error;
mod timer;

use error::RunError;
use graphics::{
    wgpu,
    wgpu::{CommandEncoderDescriptor, Device, Queue, SurfaceConfiguration},
};
use timer::Timer;
pub use winit::event_loop::EventLoopBuilder;
use winit::{
    event::{Event as WEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub struct Context<'a> {
    device: &'a Device,
    queue: &'a Queue,
    window: &'a Window,

    timer: &'a Timer,

    surface: &'a SurfaceConfiguration,
}

impl<'a> Context<'a> {
    pub fn device(&self) -> &Device {
        self.device
    }

    pub fn queue(&self) -> &Queue {
        self.queue
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    pub fn surface(&self) -> &SurfaceConfiguration {
        self.surface
    }

    pub fn timer(&self) -> &Timer {
        self.timer
    }
}

pub enum Event<'a, T = ()> {
    Window(&'a WindowEvent),
    User(T),
}

pub trait EventHandler<T = ()>: Sized {
    fn update(&mut self, ctx: &mut Context);
    fn draw(
        &mut self,
        ctx: &mut Context,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    );

    #[inline(always)]
    #[allow(unused_variables)]
    fn event(&mut self, event: Event<T>) -> bool {
        false
    }
}

pub fn run<E, T>(
    event_loop: EventLoop<T>,
    ctx: graphics::Context,
    mut state: E,
) -> Result<(), RunError>
where
    E: EventHandler<T> + 'static,
{
    // Poll by default
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = ctx.window();
    let surface = ctx.surface();
    let device = ctx.device();
    let queue = ctx.queue();

    let size = window.inner_size();

    let mut config = SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: ctx.formats()[0],
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: ctx.capabilities().alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(device, &config);

    window.set_visible(true);

    let mut timer = Timer::new();

    let mut running = true;
    timer.start();

    event_loop.run(move |event, target| {
        if !running && !target.exiting() {
            target.exit();
            return;
        }

        match event {
            WEvent::UserEvent(user) => {
                // pass on user events to the state
                let _ = state.event(Event::User(user));
            }

            WEvent::WindowEvent { event, window_id } if window_id == window.id() => {
                let handled = state.event(Event::Window(&event));

                if !handled {
                    // TODO: keep input state etc
                }

                match event {
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                        // get the new size
                        let size = window.inner_size();
                        // update the surface
                        config.width = size.width.max(1);
                        config.height = size.height.max(1);
                        surface.configure(device, &config);
                        // On macos the window needs to be redrawn manually after resizing
                        window.request_redraw();
                    }
                    WindowEvent::CloseRequested => {
                        running = false;
                        target.exit();
                    }
                    WindowEvent::RedrawRequested => {
                        timer.tick();

                        let mut context = Context {
                            device,
                            queue,
                            window,
                            timer: &timer,
                            surface: &config,
                        };

                        state.update(&mut context);

                        let frame = surface
                            .get_current_texture()
                            .expect("Failed to acquire next swap chain texture");

                        let target = frame.texture.create_view(&Default::default());

                        let mut encoder =
                            device.create_command_encoder(&CommandEncoderDescriptor::default());

                        state.draw(&mut context, &mut encoder, &target);

                        queue.submit(Some(encoder.finish()));
                        frame.present();
                    }
                    _ => (),
                }
            }
            WEvent::AboutToWait => {
                window.request_redraw();
            }
            _ => (),
        }
    })?;

    // just to check we never move ctx
    let _ = ctx;

    Ok(())
}
