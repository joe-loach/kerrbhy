use glam::{
    vec3,
    Vec3,
};
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
    pub pos: Vec3,
    pub disk_radius: f32,
    pub disk_height: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            samples: 1,
            fov: 90_f32.to_radians(),
            pos: vec3(0.0, 0.3, 3.3),
            disk_radius: 8.0,
            disk_height: 3.0,
        }
    }
}

impl From<Config> for hardware_renderer::Config {
    fn from(value: Config) -> Self {
        let Config {
            fov,
            samples: _,
            pos,
            disk_radius,
            disk_height,
        } = value;

        hardware_renderer::Config {
            fov,
            pos,
            disk_radius,
            disk_height,
        }
    }
}

impl From<Config> for software_renderer::Config {
    fn from(value: Config) -> Self {
        let Config { fov, samples, .. } = value;

        software_renderer::Config { fov, samples }
    }
}
