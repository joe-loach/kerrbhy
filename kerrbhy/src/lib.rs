use glam::vec3;
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

impl From<Config> for hardware_renderer::Params {
    fn from(value: Config) -> Self {
        let Config {
            width,
            height,
            fov,
            samples: _,
        } = value;

        hardware_renderer::Params {
            width,
            height,
            fov,
            origin: vec3(0.0, 0.2, 3.3),
            disk_radius: 8.0,
            disk_height: 3.0
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
