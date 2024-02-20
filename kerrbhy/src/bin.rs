use clap::Parser;
use graphics::wgpu;
use kerrbhy::*;

#[derive(Parser, Debug)]
struct Args {
    width: u32,
    height: u32,
    fov: f32,

    // have to have at least one sample
    #[clap(value_parser = clap::value_parser!(u32).range(1..))]
    samples: u32,

    #[clap(short, long)]
    hardware: bool,
}

fn main() -> anyhow::Result<()> {
    let Args {
        width,
        height,
        fov,
        samples,
        hardware,
        ..
    } = Args::parse();

    // create graphics context without a window
    let cb = graphics::ContextBuilder::new(
        |adapter| adapter.features(),
        wgpu::Limits::downlevel_defaults(),
    );

    let ctx = cb.build::<()>(None)?;

    let config = kerrbhy::Config {
        width,
        height,
        fov: fov.to_radians(),
        samples,
    };

    let mut renderer = if hardware {
        let mut h = Hardware::new(&ctx);
        h.update(config.into());
        Simulator::Hardware(h)
    } else {
        Simulator::Software(Software::new(config.into()))
    };

    // TODO: time this
    match &mut renderer {
        Simulator::Software(s) => s.compute(),
        Simulator::Hardware(h) => h.compute(None),
    }

    // TODO: time this
    let bytes = match renderer {
        Simulator::Software(s) => s.into_frame(),
        Simulator::Hardware(h) => h.into_frame(None),
    };

    image::save_buffer("out.png", &bytes, width, height, image::ColorType::Rgba8)
        .expect("failed to save image");

    Ok(())
}
