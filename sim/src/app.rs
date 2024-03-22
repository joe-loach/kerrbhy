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
use graphics::{
    wgpu,
    Encoder,
};
use gui::GuiState;
use hardware_renderer::*;
use profiler::PuffinStream;
use winit::{
    event_loop::EventLoop,
    keyboard::KeyCode,
};

use crate::{
    gui,
    input, ui,
};

pub(crate) struct App {
    renderer: Renderer,
    fullscreen: Fullscreen,
    gui: GuiState,

    mouse: input::Mouse,
    keyboard: input::Keyboard,

    file_dialog: Option<FileDialog>,

    gpu_start: i64,
    profiler_id_cache: profiler::IdCache,
    profiler: profiler::gpu::GpuProfiler,
    show_profiler: bool,

    accumulate: bool,
    config: Config,

    error_logs: mpsc::Receiver<String>,
}

impl App {
    pub(crate) fn new<T>(
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
            style.visuals.widgets.active.rounding = egui::Rounding::ZERO;
            style.visuals.widgets.open.rounding = egui::Rounding::ZERO;
            style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
            style.visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
            style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        });

        Self {
            renderer,
            fullscreen,
            gui,

            mouse: input::Mouse::new(),
            keyboard: input::Keyboard::new(),

            file_dialog: None,

            gpu_start: puffin::now_ns(),
            profiler_id_cache: profiler::IdCache::new(),
            profiler: profiler::gpu::GpuProfiler::new(Default::default()).unwrap(),
            show_profiler: false,

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
                        self.show_profiler = true;
                        puffin::set_scopes_on(true);
                    }
                });
            });
        });

        egui::Area::new("Settings Area")
            .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
            .show(&ctx, |ui| {
                ui.collapsing("Settings", |ui| {
                    ui.group(|ui| {
                        ui.strong("Renderer");
                        ui.checkbox(&mut vsync, "vsync");
                        ui.checkbox(&mut self.accumulate, "accumulate");
                    });

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

        let profiler_open = egui::Window::new("Profiler")
            .open(&mut self.show_profiler)
            .show(&ctx, |ui| {
                profiling::scope!("profiler");
                puffin_egui::profiler_ui(ui);
            })
            .is_some();

        if puffin::are_scopes_on() && !profiler_open {
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
        if self.keyboard.is_down(KeyCode::Space) {
            eprintln!("cleared!");
            self.profiler_id_cache.clear();
        }

        // update the camera controls
        match self.config.camera {
            common::Camera::Orbit(ref mut cam) => {
                let mut v = vec2(0.0, 0.0);

                if self.keyboard.is_down(KeyCode::KeyW) {
                    v.y += -1.0 * dt;
                }
                if self.keyboard.is_down(KeyCode::KeyS) {
                    v.y += 1.0 * dt;
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
        {
            // let encoder = &mut Encoder::from(encoder);
            let encoder =
                &mut Encoder::profiled(&self.profiler, encoder, "render", &state.device());

            // only compute more work when it's needed
            if self.accumulate || self.renderer.must_render() {
                self.renderer.compute(encoder);
            }

            self.fullscreen.draw(encoder, &self.renderer.view(), target);

            self.gui.draw(state, encoder.inner(), target);
        }

        self.profiler.resolve_queries(encoder);

        self.gpu_start = puffin::now_ns();
    }

    fn event(&mut self, state: &event::State, event: event::Event<()>) -> bool {
        let consumed = self.gui.handle_event(&event);

        if !consumed {
            self.mouse.update_state(state.window(), &event);
            self.keyboard.update_state(&event);
        }

        consumed
    }

    fn frame_end(&mut self, state: &event::State) {
        if self.profiler.end_frame().is_ok() {
            let _ = self.profiler.send_to_puffin(
                self.gpu_start,
                state.queue().get_timestamp_period(),
                Some(&mut self.profiler_id_cache),
            );
        }
    }
}
