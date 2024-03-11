pub mod camera;

use std::str::FromStr;

use glam::{
    vec3,
    Affine3A,
    Vec3,
};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Disk {
    pub radius: f32,
    pub thickness: f32,
    pub color: Vec3,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub samples: u32,
    pub features: Features,
    pub fov: f32,
    pub view: Affine3A,
    pub disk: Disk,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            samples: 1,
            features: Features::empty(),
            fov: 90_f32.to_radians(),
            view: Affine3A::IDENTITY,
            disk: Default::default(),
        }
    }
}
