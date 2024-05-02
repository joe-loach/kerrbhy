use std::path::{Path, PathBuf};

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
    /// The kind of renderer to use.
    renderer: RendererKind,

    /// The width of the image to create.
    width: u32,
    /// The height of the image to create.
    height: u32,

    /// The number of samples to compute.
    /// 
    /// Must be greater than 0.
    /// 
    /// The higher the number, the more frames are produced and a higher quality image will be produced.
    #[clap(short, long, default_value = "1", value_parser=clap::value_parser!(u32).range(1..),)]
    samples: u32,

    /// The config file to load.
    /// 
    /// For more interesting configs, save them in the simulator and load them here.
    #[clap(short, long)]
    config: Option<PathBuf>,

    /// Saves the frame output to disk.
    #[clap(long)]
    save: bool,

    /// Configures the output path of the frame on disk.
    /// 
    /// Defaults to `out.png`.
    #[clap(long)]
    output: Option<PathBuf>,

    /// Creates and shows trace information.
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
            // need to update the state with the correct config before computing
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

    // load the supplied config
    let config = if let Some(path) = args.config.as_ref() {
        Config::load_from_path(path)?
    } else {
        log::warn!("using default config");

        Config::default()
    };

    // create our context
    let ctx = context()?;

    // create the renderer
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

    // save the frame if they requested it
    if args.save {
        match renderer {
            Renderer::Hardware { renderer, .. } => {
                let frame_encoder = ctx.device().create_command_encoder(&Default::default());
                let bytes = renderer.into_frame(frame_encoder);
                save_image(&bytes, width, height, args.output.as_deref())?;
            }
            Renderer::Software(renderer) => {
                let bytes = renderer.into_frame();
                save_image(&bytes, width, height, args.output.as_deref())?;
            }
        }
    }

    profiling::finish_frame!();

    Ok(())
}

fn save_image(bytes: &[u8], width: u32, height: u32, path: Option<&Path>) -> anyhow::Result<()> {
    profiling::scope!("Saving image");

    let path = path.unwrap_or_else(|| Path::new("out.png"));
    image::save_buffer(path, bytes, width, height, image::ColorType::Rgba8)?;

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
        // if we're creating a flamegraph,
        // we need to enable puffin and
        // create a new server to send the information to `puffin_viewer`.

        puffin::set_scopes_on(true);

        let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);

        let server = puffin_http::Server::new(&server_addr)?;

        // open puffin viewer as a child process
        let viewer = std::process::Command::new("puffin_viewer")
            .spawn()
            .context("puffin_viewer has to be installed to see flamegraph")?;

        Some((viewer, server))
    } else {
        None
    };

    // start the computation
    compute(&args)?;

    if let Some((mut viewer, server)) = bundle {
        // wait for the viewer to close after we've finished computation
        viewer.wait()?;

        drop(server);
    }

    Ok(())
}
