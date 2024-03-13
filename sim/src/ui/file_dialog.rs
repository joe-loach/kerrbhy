use std::{
    fs,
    io::Write,
};

use anyhow::Context as _;
use common::Config;
use egui::Context;
use egui_file::{
    DialogType,
    FileDialog,
};

pub fn show(
    ctx: &Context,
    dialog: Option<&mut FileDialog>,
    config: &mut Config,
) -> anyhow::Result<()> {
    if let Some(dialog) = dialog {
        if dialog.show(ctx).selected() {
            match dialog.dialog_type() {
                DialogType::OpenFile => {
                    if let Some(path) = dialog.path() {
                        let contents = fs::read_to_string(path).with_context(|| {
                            format!("failed to read config file at {}", path.display())
                        })?;

                        if let Ok(cfg) = Config::load(&contents) {
                            log::info!("loaded new config from {}", path.display());

                            *config = cfg;
                        } else {
                            log::error!("failed to load config from {}", path.display());
                        }
                    }
                }
                DialogType::SaveFile => {
                    if let Some(path) = dialog.path() {
                        let mut file = fs::File::options()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(path)
                            .with_context(|| format!("failed to open file {}", path.display()))?;

                        config
                            .save(&mut file)
                            .context("failed to save config to file")?;
                        file.flush()?;

                        log::info!("saved config to {}", path.display());
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}
