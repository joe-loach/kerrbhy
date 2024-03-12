use serde::{
    Deserialize,
    Serialize,
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Degree(pub f32);

impl Degree {
    pub fn as_f32(&self) -> f32 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Radians(pub f32);

impl Radians {
    pub fn as_f32(&self) -> f32 {
        self.0
    }
}

impl From<Degree> for Radians {
    fn from(value: Degree) -> Self {
        Radians(value.0.to_radians())
    }
}

impl From<Radians> for Degree {
    fn from(value: Radians) -> Self {
        Degree(value.0.to_degrees())
    }
}
