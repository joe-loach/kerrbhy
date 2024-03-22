mod app;
mod gui;
mod input;
mod ui;

use std::sync::mpsc;

use graphics::wgpu;
use time::format_description::well_known::Rfc3339;
use winit::{
    dpi::PhysicalSize,
    window::WindowBuilder,
};

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

    event::run(event_loop, cb, |el, ctx| app::App::new(el, ctx, error_logs))?;

    Ok(())
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
