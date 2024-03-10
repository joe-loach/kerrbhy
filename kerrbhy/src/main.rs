use std::{
    str::FromStr,
    sync::{
        atomic::{
            AtomicBool,
            Ordering,
        },
        Arc,
    },
};

use clap::Parser;
use common::{
    Config,
    Features,
};
use eframe::egui;
use graphics::wgpu;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum RendererKind {
    Hardware,
    Software,
}

enum Renderer {
    Hardware(hardware_renderer::Renderer, wgpu::CommandEncoder),
    Software(software_renderer::Renderer),
}

#[derive(Parser, Debug, Clone)]
struct Args {
    renderer: RendererKind,

    width: u32,
    height: u32,

    #[clap(long)]
    fov: Option<f32>,

    // have to have at least one sample
    #[clap(long, value_parser = clap::value_parser!(u32).range(1..))]
    samples: Option<u32>,

    #[clap(long)]
    flamegraph: bool,

    #[clap(short, long, value_delimiter = ',', num_args = 1..)]
    features: Option<Vec<String>>,
}

#[derive(Default)]
struct State(Arc<StateInner>);

impl Clone for State {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl std::ops::Deref for State {
    type Target = StateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct StateInner {
    pub started: AtomicBool,
    pub finished: AtomicBool,
}

impl Default for StateInner {
    fn default() -> Self {
        Self {
            started: AtomicBool::new(false),
            finished: AtomicBool::new(false),
        }
    }
}

fn compute_and_save(args: &Args, state: State) -> anyhow::Result<()> {
    let Args {
        width,
        height,
        fov,
        samples,
        renderer,
        ..
    } = *args;

    {
        profiling::scope!("Waiting");

        while !state.started.load(Ordering::Relaxed) {
            // wait, but not for very long
            std::hint::spin_loop()
        }
    }

    // create graphics context without a window
    let ctx = {
        profiling::scope!("Creating context");

        let cb = graphics::ContextBuilder::new(
            |adapter| adapter.features(),
            wgpu::Limits::downlevel_defaults(),
        );

        cb.build::<()>(None)?
    };

    let mut config = Config::default();

    if let Some(fov) = fov {
        config.fov = fov;
    }

    if let Some(samples) = samples {
        config.samples = samples;
    }

    if let Some(ref features) = args.features {
        config.features = features
            .iter()
            .filter_map(|f| Features::from_str(f).ok())
            .fold(Features::empty(), |acc, f| acc.union(f));
    }

    let mut renderer = match renderer {
        RendererKind::Hardware => {
            profiling::scope!("hardware::new");

            let enc = ctx.device().create_command_encoder(&Default::default());

            let mut h = hardware_renderer::Renderer::new(&ctx);
            h.update(width, height, config);
            Renderer::Hardware(h, enc)
        }
        RendererKind::Software => {
            profiling::scope!("software::new");

            let s = software_renderer::Renderer::new(width, height, config);
            Renderer::Software(s)
        }
    };

    match &mut renderer {
        Renderer::Software(s) => s.compute(),
        Renderer::Hardware(h, enc) => h.compute(enc),
    }

    let bytes = match renderer {
        Renderer::Software(s) => s.into_frame(),
        Renderer::Hardware(h, enc) => h.into_frame(enc),
    };

    {
        profiling::scope!("Saving image");

        image::save_buffer("out.png", &bytes, width, height, image::ColorType::Rgba8)?;
    }

    profiling::finish_frame!();

    state.finished.store(true, Ordering::Relaxed);

    Ok(())
}

fn show_flamegraph(state: State, mut profiler: puffin_egui::GlobalProfilerUi) {
    let mut started = false;
    let mut finished = false;

    eframe::run_simple_native(
        "flamegraph",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 800.0])
                .with_min_inner_size([600.0, 600.0]),
            vsync: true,
            ..Default::default()
        },
        move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| profiler.ui(ui));

            if !started {
                state.started.store(true, Ordering::Relaxed);
                started = true;
            }
            if !finished {
                finished = state.finished.load(Ordering::Relaxed);
                ctx.request_repaint()
            }
        },
    )
    .expect("failed to setup graphics");
}

const COMPUTE_THREAD: &str = "compute";

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let profiler = if args.flamegraph {
        puffin::set_scopes_on(true);

        let mut profiler = puffin_egui::GlobalProfilerUi::default();
        profiler.profiler_ui.flamegraph_options.rounding = 0.0;
        profiler.profiler_ui.flamegraph_options.merge_scopes = true;

        Some(profiler)
    } else {
        None
    };

    let state = State::default();

    std::thread::scope(|s| -> anyhow::Result<()> {
        let state_clone = state.clone();

        let compute = std::thread::Builder::new()
            .name(COMPUTE_THREAD.to_owned())
            .spawn_scoped(s, || compute_and_save(&args, state_clone))?;

        if let Some(profiler) = profiler {
            show_flamegraph(state.clone(), profiler);
        } else {
            state.started.store(true, Ordering::Relaxed);
            let _ = compute.join();
        }

        Ok(())
    })?;

    Ok(())
}
