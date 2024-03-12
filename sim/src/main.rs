mod gui;
mod input;

use std::path::PathBuf;

use common::Features;
use egui_file::{
    DialogType,
    FileDialog,
};
use event::EventHandler;
use fullscreen::Fullscreen;
use glam::vec2;
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

    file_dialog: Option<FileDialog>,
    last_path: Option<PathBuf>,

    profiler: bool,

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

            file_dialog: None,
            last_path: None,

            profiler: false,

            accumulate: true,
            config: Config::default(),
        }
    }

    #[profiling::function]
    fn ui(&mut self, ctx: egui::Context, state: &mut event::State) {
        let mut vsync = state.is_vsync();

        egui::Area::new("Info Area")
            .anchor(egui::Align2::RIGHT_TOP, [0.0, 0.0])
            .show(&ctx, |ui| {
                ui.collapsing("Info", |ui| {
                    ui.checkbox(&mut vsync, "vsync");
                    ui.checkbox(&mut self.accumulate, "accumulate");

                    config_ui(ui, &mut self.config);

                    if ui.button("Save").clicked() {
                        let mut dialog = FileDialog::save_file(self.last_path.clone());
                        dialog.open();
                        self.file_dialog = Some(dialog);
                    }
                    if ui.button("Load").clicked() {
                        let mut dialog = FileDialog::open_file(self.last_path.clone());
                        dialog.open();
                        self.file_dialog = Some(dialog);
                    }

                    if let Some(dialog) = self.file_dialog.as_mut() {
                        if dialog.show(&ctx).selected() {
                            match dialog.dialog_type() {
                                DialogType::OpenFile => {
                                    if let Some(path) = dialog.path() {
                                        let contents = std::fs::read_to_string(path)
                                            .expect("failed to read config file");
                                        if let Ok(cfg) = Config::load(&contents) {
                                            self.config = cfg;
                                        }
                                    }
                                }
                                DialogType::SaveFile => {
                                    if let Some(path) = dialog.path() {
                                        let mut file = std::fs::File::options()
                                            .write(true)
                                            .truncate(true)
                                            .create(true)
                                            .open(path)
                                            .expect("failed to create config file");
                                        self.config.save(&mut file).expect("failed to save config");
                                        self.last_path = Some(dialog.directory().to_path_buf());
                                    }
                                }
                                _ => unreachable!(),
                            }
                        }
                    }

                    ui.separator();

                    if ui.button("Profiler").clicked() {
                        self.profiler = true;
                        puffin::set_scopes_on(true);
                    }
                });
            });

        let response = egui::Window::new("Profiler")
            .open(&mut self.profiler)
            .show(&ctx, |ui| {
                profiling::scope!("profiler");
                puffin_egui::profiler_ui(ui);
            });

        if puffin::are_scopes_on() && response.is_none() {
            puffin::set_scopes_on(false);
        }

        state.set_vsync(vsync);
    }
}

fn config_ui(ui: &mut egui::Ui, cfg: &mut Config) {
    ui.separator();

    ui.vertical(|ui| {
        ui.label("Features:");
        for (name, f) in Features::all().iter_names() {
            let mut on = cfg.features.contains(f);
            ui.checkbox(&mut on, name);
            cfg.features.set(f, on);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Fov: ");
        ui.drag_angle(&mut cfg.camera.fov_mut().0);
    });
    ui.vertical(|ui| {
        ui.set_enabled(cfg.features.contains(Features::DISK));

        ui.label("Disk");
        ui.add(
            egui::DragValue::new(&mut cfg.disk.radius)
                .speed(0.1)
                .prefix("Radius: ")
                .clamp_range(0.0..=10.0),
        );
        ui.add(
            egui::DragValue::new(&mut cfg.disk.thickness)
                .speed(0.1)
                .prefix("Thickness: ")
                .clamp_range(0.0..=10.0),
        );
    });

    ui.separator();
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

    event::run(event_loop, cb, App::new)?;

    Ok(())
}
