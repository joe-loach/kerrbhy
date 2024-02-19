mod error;
mod timer;

use std::sync::Arc;

use error::RunError;
use graphics::{
    wgpu,
    wgpu::{
        CommandEncoderDescriptor,
        Device,
        Queue,
        SurfaceConfiguration,
    },
};
use timer::Timer;
pub use winit::event_loop::EventLoopBuilder;
use winit::{
    event::{
        Event as WEvent,
        WindowEvent,
    },
    event_loop::{
        ControlFlow,
        EventLoop,
    },
    window::Window,
};

pub struct State<'a> {
    device: &'a Arc<Device>,
    queue: &'a Arc<Queue>,
    window: &'a Window,

    timer: &'a Timer,

    surface: &'a SurfaceConfiguration,
}

impl<'a> State<'a> {
    pub fn dimensions(&self) -> (u32, u32) {
        // both dimensions are guaranteed to be greater than 0
        (self.surface.width, self.surface.height)
    }

    pub fn device(&self) -> Arc<Device> {
        Arc::clone(self.device)
    }

    pub fn queue(&self) -> Arc<Queue> {
        Arc::clone(self.queue)
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
    fn update(&mut self, state: &State);
    fn draw(
        &mut self,
        state: &mut State,
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
    mut gfx: graphics::ContextBuilder,
    state: impl FnOnce(&EventLoop<T>, &graphics::Context) -> E,
) -> Result<(), RunError>
where
    E: EventHandler<T> + 'static,
{
    // build the graphics context
    // make sure that they have a window
    if !gfx.has_window() {
        gfx = gfx.with_window(winit::window::WindowBuilder::new())
    }

    let ctx = gfx.build(Some(&event_loop))?;

    // create the state
    let mut state = (state)(&event_loop, &ctx);

    // Poll by default
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = ctx.window().expect("created with a window");
    let surface = ctx.surface().expect("created with a window");
    let device = ctx.device();
    let queue = ctx.queue();

    let size = window.inner_size();

    let mut config = SurfaceConfiguration {
        desired_maximum_frame_latency: 2,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: ctx.view_format().expect("created with a window"),
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: ctx
            .capabilities()
            .expect("created with a window")
            .alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

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
                        reconfigure(&window, surface, &mut config, &device);
                        // On macos the window needs to be redrawn manually after resizing
                        window.request_redraw();
                    }
                    WindowEvent::CloseRequested => {
                        running = false;
                        target.exit();
                    }
                    WindowEvent::RedrawRequested => {
                        timer.tick();

                        let mut frame = surface.get_current_texture();

                        if let Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) = frame
                        {
                            reconfigure(&window, surface, &mut config, &device);
                            frame = surface.get_current_texture();
                        }

                        if let Err(wgpu::SurfaceError::OutOfMemory) = frame {
                            target.exit()
                        }

                        if let Ok(frame) = frame {
                            let mut context = State {
                                device: &device,
                                queue: &queue,
                                window: &window,
                                timer: &timer,
                                surface: &config,
                            };

                            state.update(&context);

                            let target = frame.texture.create_view(&Default::default());

                            let mut encoder =
                                device.create_command_encoder(&CommandEncoderDescriptor::default());

                            state.draw(&mut context, &mut encoder, &target);

                            queue.submit(Some(encoder.finish()));
                            frame.present();
                        }
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

    // just to check we never move
    let _ = ctx;
    let _ = state;

    Ok(())
}

fn reconfigure(
    window: &Window,
    surface: &wgpu::Surface,
    config: &mut SurfaceConfiguration,
    device: &wgpu::Device,
) {
    let size = window.inner_size();
    // update the surface
    config.width = size.width.max(1);
    config.height = size.height.max(1);
    surface.configure(device, config);
}
