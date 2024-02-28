use glam::vec3;
pub use hardware_renderer::Renderer as Hardware;
pub use software_renderer::Renderer as Software;

pub enum Simulator {
    Software(Software),
    Hardware(Hardware),
}

#[derive(Clone)]
pub struct Config {
    pub samples: u32,
    pub fov: f32,
}

impl From<Config> for hardware_renderer::Config {
    fn from(value: Config) -> Self {
        let Config {
            fov,
            samples: _,
        } = value;

        hardware_renderer::Config {
            fov,
            origin: vec3(0.0, 0.2, 3.3),
            disk_radius: 8.0,
            disk_height: 3.0
        }
    }
}

impl From<Config> for software_renderer::Config {
    fn from(value: Config) -> Self {
        let Config {
            fov,
            samples,
        } = value;

        software_renderer::Config {
            fov,
            samples,
        }
    }
}
