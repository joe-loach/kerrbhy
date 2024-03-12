mod angle;
pub mod camera;
mod error;

use std::{
    path::Path,
    str::FromStr,
};

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
        const DISK = 1;
    }
}

impl FromStr for Features {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let kind = match s.to_lowercase().as_str() {
            "disk" => Features::DISK,
            _ => return Err("invalid feature"),
        };
        Ok(kind)
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
            thickness: 3.0,
            color: vec3(0.3, 0.2, 0.1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub samples: u32,
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
            samples: 1,
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
