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
    /// List of runtime features for Renderers.
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
    /// Radius of the disk
    pub radius: f32,
    /// Thickness (height) of the disk
    pub thickness: f32,
    /// The apparent color of the disk
    pub color: Vec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// The camera used to control perspective of the rays fired from it.
pub enum Camera {
    Orbit(OrbitCamera),
}

impl Camera {
    /// The view matrix for the [`Cameras`](Camera) perspective.
    pub fn view(&self) -> Affine3A {
        match self {
            Camera::Orbit(cam) => cam.view(),
        }
    }

    /// The field of view of the [`Camera`] in [`Radians`].
    pub fn fov(&self) -> Radians {
        match self {
            Camera::Orbit(cam) => cam.fov,
        }
    }

    /// A mutable view to the [`Cameras`](Camera) field of view.
    /// Allows you to change the fov at runtime.
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
    /// Load a config from a file.
    /// 
    /// Fails if the file cannot be read or parsed.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, error::ConfigError> {
        let path = path.as_ref();

        let contents = std::fs::read_to_string(path)?;

        Self::load(&contents)
    }

    /// Loads a config file from a string.
    pub fn load(s: &str) -> Result<Self, error::ConfigError> {
        Ok(toml::from_str(s)?)
    }

    /// Saves a config file to disk.
    /// 
    /// Fails if the toml couldn't be generated, or the contents couldn't be written.
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
                // the center (where the black hole is)
                Vec3::ZERO,
            )),
            disk: Default::default(),
        }
    }
}
