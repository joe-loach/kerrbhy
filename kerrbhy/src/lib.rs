pub use hardware_renderer::Renderer as Hardware;
pub use software_renderer::Renderer as Software;

pub enum Simulator {
    Software(Software),
    Hardware(Hardware),
}

#[derive(Clone)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub fov: f32,
    pub samples: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fov: 90.0_f32.to_radians(),
            samples: 1,
        }
    }
}

impl From<hardware_renderer::Params> for Config {
    fn from(value: hardware_renderer::Params) -> Self {
        let hardware_renderer::Params { width, height, fov } = value;

        Config {
            width,
            height,
            fov,
            samples: 1,
        }
    }
}

impl From<Config> for hardware_renderer::Params {
    fn from(value: Config) -> Self {
        let Config {
            width,
            height,
            fov,
            samples: _,
        } = value;

        hardware_renderer::Params { width, height, fov }
    }
}

impl From<software_renderer::Params> for Config {
    fn from(value: software_renderer::Params) -> Self {
        let software_renderer::Params {
            width,
            height,
            fov,
            samples,
        } = value;

        Config {
            width,
            height,
            fov,
            samples,
        }
    }
}

impl From<Config> for software_renderer::Params {
    fn from(value: Config) -> Self {
        let Config {
            width,
            height,
            fov,
            samples,
        } = value;

        software_renderer::Params {
            width,
            height,
            fov,
            samples,
        }
    }
}
