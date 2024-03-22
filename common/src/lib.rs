mod angle;
pub mod camera;
mod error;

use std::path::Path;

pub use angle::{
    Degree,
    Radians,
};
use camera::OrbitCamera;
use glam::{
    vec3,
    Affine3A,
    Vec3,
};
use serde::{
    Deserialize,
    Serialize,
};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Features: u32 {
        const DISK_SDF      = 1 << 0;
        const DISK_VOL      = 1 << 1;
        const SKY_PROC      = 1 << 2;
        const AA            = 1 << 3;
        const RK4           = 1 << 4;
        const ADAPTIVE  = 1 << 5;
        const BLOOM         = 1 << 6;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Disk {
    pub radius: f32,
    pub thickness: f32,
    pub color: Vec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Camera {
    Orbit(OrbitCamera),
}

impl Camera {
    pub fn view(&self) -> Affine3A {
        match self {
            Camera::Orbit(cam) => cam.view(),
        }
    }

    pub fn fov(&self) -> Radians {
        match self {
            Camera::Orbit(cam) => cam.fov,
        }
    }

    pub fn fov_mut(&mut self) -> &mut Radians {
        match self {
            Camera::Orbit(cam) => &mut cam.fov,
        }
    }
}

impl Default for Disk {
    fn default() -> Self {
        Self {
            radius: 8.0,
            thickness: 0.1,
            color: vec3(0.3, 0.2, 0.1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub features: Features,
    pub camera: Camera,
    pub disk: Disk,
}

impl Config {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, error::ConfigError> {
        let path = path.as_ref();

        let contents = std::fs::read_to_string(path)?;

        Self::load(&contents)
    }

    pub fn load(s: &str) -> Result<Self, error::ConfigError> {
        Ok(toml::from_str(s)?)
    }

    pub fn save(&self, writer: &mut impl std::io::Write) -> Result<(), error::ConfigError> {
        let toml = toml::to_string_pretty(self)?;

        write!(writer, "{}", toml)?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            features: Features::empty(),
            camera: Camera::Orbit(OrbitCamera::new(
                // 90 degree FOV
                angle::Degree(90.0),
                // start at distance 3.3
                3.3,
                // bounds for the orbit
                0.5..=3.5,
                // target
                Vec3::ZERO,
            )),
            disk: Default::default(),
        }
    }
}
