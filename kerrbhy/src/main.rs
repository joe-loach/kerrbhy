use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;
use common::Config;
use graphics::{
    wgpu,
    Context,
};
use hardware_renderer::Renderer as HardwareRenderer;
use profiler::{
    gpu::GpuProfiler,
    PuffinStream as _,
};
use software_renderer::Renderer as SoftwareRenderer;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum RendererKind {
    Hardware,
    Software,
}

enum Renderer {
    Hardware {
        renderer: HardwareRenderer,
        profiler: Option<GpuProfiler>,
    },
    Software(SoftwareRenderer),
}

#[derive(Parser, Debug, Clone)]
struct Args {
    renderer: RendererKind,

    width: u32,
    height: u32,

    #[clap(short, long, default_value = "1", value_parser=clap::value_parser!(u32).range(1..),)]
    samples: u32,

    #[clap(short, long)]
    config: Option<PathBuf>,

    #[clap(long)]
    save: bool,

    #[clap(long)]
    flamegraph: bool,
}

fn context() -> anyhow::Result<Context> {
    profiling::scope!("Creating context");

    // create graphics context without a window
    let cb = graphics::ContextBuilder::new(
        |adapter| adapter.features(),
        wgpu::Limits::downlevel_defaults(),
    );

    Ok(cb.build::<()>(None)?)
}

fn renderer(ctx: &Context, config: Config, args: &Args) -> anyhow::Result<Renderer> {
    profiling::scope!("renderer::new");

    let renderer = match args.renderer {
        RendererKind::Hardware => {
            let mut renderer = HardwareRenderer::new(ctx);
            renderer.update(args.width, args.height, config);

            let profiler = if args.flamegraph {
                Some(GpuProfiler::new(Default::default())?)
            } else {
                None
            };

            Renderer::Hardware { renderer, profiler }
        }
        RendererKind::Software => {
            Renderer::Software(SoftwareRenderer::new(args.width, args.height, config))
        }
    };

    Ok(renderer)
}

fn hardware_frame(
    renderer: &mut HardwareRenderer,
    mut profiler: Option<&mut GpuProfiler>,
    ctx: &Context,
    sample: u32,
) -> anyhow::Result<()> {
    let device = ctx.device();

    let mut encoder = device.create_command_encoder(&Default::default());

    {
        let mut encoder = if let Some(ref profiler) = profiler {
            graphics::Encoder::profiled(
                profiler,
                &mut encoder,
                format!("sample #{sample}"),
                &device,
            )
        } else {
            graphics::Encoder::Wgpu(&mut encoder)
        };

        renderer.compute(&mut encoder);
    }

    if let Some(ref mut profiler) = profiler {
        profiler.resolve_queries(&mut encoder);
    }

    let queue = ctx.queue();
    let gpu_start = puffin::now_ns();

    // submit the commands to finish the work
    queue.submit(Some(encoder.finish()));

    if let Some(ref mut profiler) = profiler {
        // record the GPU debug info for the flamegraph

        profiler.end_frame()?;

        // wait for the wgpu to be finished to get debug data
        device.poll(wgpu::Maintain::Wait).panic_on_timeout();

        match profiler.send_to_puffin(gpu_start, queue.get_timestamp_period(), None) {
            profiler::StreamResult::Success => (),
            profiler::StreamResult::Empty => (),
            profiler::StreamResult::Disabled => log::warn!("puffin is disabled"),
            profiler::StreamResult::Failure => log::error!("failed to send puffin data"),
        }
    }

    profiling::finish_frame!();

    Ok(())
}

fn software_frame(renderer: &mut SoftwareRenderer, sample: u32) {
    profiling::scope!("sample", format!("#{sample}"));

    renderer.compute(sample);

    profiling::finish_frame!();
}

fn compute(args: &Args) -> anyhow::Result<()> {
    let Args {
        width,
        height,
        samples,
        ..
    } = *args;

    let config = if let Some(path) = args.config.as_ref() {
        Config::load_from_path(path)?
    } else {
        log::warn!("using default config");

        Config::default()
    };

    let ctx = context()?;

    let mut renderer = renderer(&ctx, config, args)?;

    // compute the image
    match &mut renderer {
        Renderer::Hardware { renderer, profiler } => {
            for sample in 0..samples {
                hardware_frame(renderer, profiler.as_mut(), &ctx, sample)?;
            }
        }
        Renderer::Software(renderer) => {
            for sample in 0..samples {
                software_frame(renderer, sample);
            }
        }
    }

    match renderer {
        Renderer::Hardware { renderer, .. } => {
            if args.save {
                let frame_encoder = ctx.device().create_command_encoder(&Default::default());
                let bytes = renderer.into_frame(frame_encoder);
                save_image(&bytes, width, height)?;
            }
        }
        Renderer::Software(renderer) => {
            if args.save {
                let bytes = renderer.into_frame();
                save_image(&bytes, width, height)?;
            }
        }
    }

    Ok(())
}

fn save_image(bytes: &[u8], width: u32, height: u32) -> anyhow::Result<()> {
    profiling::scope!("Saving image");

    image::save_buffer("out.png", bytes, width, height, image::ColorType::Rgba8)?;

    Ok(())
}

fn init_logger() -> Result<(), fern::InitError> {
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
        .apply()?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    init_logger()?;

    let args = Args::parse();

    let bundle = if args.flamegraph {
        puffin::set_scopes_on(true);

        let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);

        let server = puffin_http::Server::new(&server_addr)?;

        let viewer = std::process::Command::new("puffin_viewer")
            .spawn()
            .context("puffin_viewer has to be installed to see flamegraph")?;

        Some((viewer, server))
    } else {
        None
    };

    compute(&args)?;

    if let Some((mut viewer, server)) = bundle {
        viewer.wait()?;

        drop(server);
    }

    Ok(())
}
