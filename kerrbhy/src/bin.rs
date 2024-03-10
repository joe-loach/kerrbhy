use std::sync::{
    atomic::{
        AtomicBool,
        Ordering,
    },
    Arc,
};

use clap::Parser;
use eframe::egui;
use graphics::wgpu;
use kerrbhy::*;

#[derive(Parser, Debug, Clone)]
struct Args {
    width: u32,
    height: u32,
    fov: f32,

    // have to have at least one sample
    #[clap(value_parser = clap::value_parser!(u32).range(1..))]
    samples: u32,

    #[clap(long)]
    hardware: bool,

    #[clap(long)]
    flamegraph: bool,
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
        hardware,
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

    let config = kerrbhy::Config {
        fov: fov.to_radians(),
        samples,
        ..Default::default()
    };

    let mut renderer = if hardware {
        profiling::scope!("hardware::new");

        let mut h = Hardware::new(&ctx);
        h.update(width, height, config.into());
        Simulator::Hardware(h)
    } else {
        profiling::scope!("software::new");

        Simulator::Software(Software::new(width, height, config.into()))
    };

    match &mut renderer {
        Simulator::Software(s) => s.compute(),
        Simulator::Hardware(h) => h.compute(None),
    }

    let bytes = match renderer {
        Simulator::Software(s) => s.into_frame(),
        Simulator::Hardware(h) => h.into_frame(None),
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
