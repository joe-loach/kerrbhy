mod gui;
mod input;
mod ui;

use std::sync::mpsc;

use egui_file::FileDialog;
use egui_toast::{
    Toast,
    ToastKind,
    ToastOptions,
    Toasts,
};
use event::EventHandler;
use fullscreen::Fullscreen;
use glam::vec2;
use graphics::wgpu;
use gui::GuiState;
use hardware_renderer::*;
use time::format_description::well_known::Rfc3339;
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

    file_dialog: Option<FileDialog>,

    profiler: bool,

    accumulate: bool,
    config: Config,

    error_logs: mpsc::Receiver<String>,
}

impl App {
    fn new<T>(
        _event_loop: &EventLoop<T>,
        ctx: &graphics::Context,
        errors: mpsc::Receiver<String>,
    ) -> Self {
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

            file_dialog: None,

            profiler: false,

            accumulate: true,
            config: Config::default(),

            error_logs: errors,
        }
    }

    #[profiling::function]
    fn ui(&mut self, ctx: egui::Context, state: &mut event::State) {
        let mut vsync = state.is_vsync();

        // create toast notifications
        let mut toasts = Toasts::new()
            .anchor(egui::Align2::CENTER_BOTTOM, (0.0, -10.0))
            .direction(egui::Direction::TopDown);

        let toast_options = ToastOptions::default().duration_in_seconds(4.0);

        egui::TopBottomPanel::top("Top Bar").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().visuals.button_frame = false;

                let dir = self
                    .file_dialog
                    .as_ref()
                    .map(|fd| fd.directory().to_owned());

                ui.add_space(10.0);

                if ui.button("Save").clicked() {
                    let mut dialog = FileDialog::save_file(dir.clone());
                    dialog.open();
                    self.file_dialog = Some(dialog);
                }

                if ui.button("Open").clicked() {
                    let mut dialog = FileDialog::open_file(dir.clone());
                    dialog.open();
                    self.file_dialog = Some(dialog);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);

                    if ui.button("Profiler").clicked() {
                        self.profiler = true;
                        puffin::set_scopes_on(true);
                    }
                });
            });
        });

        egui::Area::new("Settings Area")
            .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
            .show(&ctx, |ui| {
                ui.collapsing("Settings", |ui| {
                    ui.checkbox(&mut vsync, "vsync");
                    ui.checkbox(&mut self.accumulate, "accumulate");

                    ui::config::show(ui, &mut self.config);
                });
            });

        match ui::file_dialog::show(&ctx, self.file_dialog.as_mut(), &mut self.config) {
            Ok(Some(ui::file_dialog::Action::Opened)) => {
                toasts.add(Toast {
                    kind: ToastKind::Success,
                    text: "Opened file".into(),
                    options: toast_options,
                });
            }
            Ok(Some(ui::file_dialog::Action::Saved)) => {
                toasts.add(Toast {
                    kind: ToastKind::Success,
                    text: "Saved file".into(),
                    options: toast_options,
                });
            }
            Ok(None) => (),
            Err(e) => {
                log::error!(target: "file dialog", "{e}");
            }
        }

        let response = egui::Window::new("Profiler")
            .open(&mut self.profiler)
            .show(&ctx, |ui| {
                profiling::scope!("profiler");
                puffin_egui::profiler_ui(ui);
            });

        if puffin::are_scopes_on() && response.is_none() {
            puffin::set_scopes_on(false);
        }

        // read error notifications from channel
        if let Ok(msg) = self.error_logs.try_recv() {
            toasts.add(Toast {
                kind: ToastKind::Error,
                text: msg.into(),
                options: toast_options,
            });
        }

        // show all the toasts at the end
        toasts.show(&ctx);

        state.set_vsync(vsync);
    }
}

impl EventHandler for App {
    fn update(&mut self, state: &mut event::State) {
        let (width, height) = state.dimensions();

        let dt = state.timer().dt();

        // update the camera controls
        match self.config.camera {
            common::Camera::Orbit(ref mut cam) => {
                let mut v = vec2(0.0, 0.0);

                if self.keyboard.is_down(KeyCode::KeyW) {
                    v.y += 1.0 * dt;
                }
                if self.keyboard.is_down(KeyCode::KeyS) {
                    v.y += -1.0 * dt;
                }
                if self.keyboard.is_down(KeyCode::KeyA) {
                    v.x += 1.0 * dt;
                }
                if self.keyboard.is_down(KeyCode::KeyD) {
                    v.x += -1.0 * dt;
                }
                cam.orbit(v);

                let zoom = -self.mouse.scroll_delta().y / input::Mouse::PIXELS_PER_LINE;
                cam.zoom(zoom * dt);
            }
        };

        self.mouse.smooth(dt);

        self.renderer.update(width, height, self.config.clone());

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
            self.renderer.compute(encoder);
        }

        self.fullscreen.draw(encoder, &self.renderer.view(), target);

        self.gui.draw(state, encoder, target);
    }

    fn event(&mut self, state: &event::State, event: event::Event<()>) -> bool {
        let consumed = self.gui.handle_event(&event);

        if !consumed {
            self.mouse.update_state(state.window(), &event);
            self.keyboard.update_state(&event);
        }

        consumed
    }
}

fn init_logger() -> Result<mpsc::Receiver<String>, fern::InitError> {
    const LOG_LEVEL_ENV: &str = "KERRBHY_LOG";

    // try and get the log level and parse it from ENV
    let level = std::env::var(LOG_LEVEL_ENV)
        .ok()
        .and_then(|level| level.parse::<log::LevelFilter>().ok())
        .unwrap_or({
            // choose specific defaults if not in release
            if cfg!(debug_assertions) {
                log::LevelFilter::Warn
            } else {
                log::LevelFilter::Error
            }
        });

    // create a channel for listening to logs
    let (tx, rx) = mpsc::channel();

    fern::Dispatch::new()
        .level(level)
        // output to std-error with as much info as possible
        .chain(
            fern::Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{} {} {}] {}",
                        time::OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
                        record.level(),
                        record.target(),
                        message
                    ))
                })
                .chain(std::io::stderr()),
        )
        // output simple errors to the channel
        .chain(
            fern::Dispatch::new()
                .format(|out, message, _| out.finish(format_args!("{}", message)))
                .level(log::LevelFilter::Error)
                .chain(fern::Output::sender(tx, "")),
        )
        .apply()?;

    Ok(rx)
}

fn main() -> anyhow::Result<()> {
    let error_logs = init_logger()?;

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

    event::run(event_loop, cb, |el, ctx| App::new(el, ctx, error_logs))?;

    Ok(())
}
