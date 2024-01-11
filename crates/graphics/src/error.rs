use thiserror::Error;
use wgpu::{CreateSurfaceError, RequestDeviceError};
use winit::error::{EventLoopError, OsError};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    EventLoopError(#[from] EventLoopError),

    #[error(transparent)]
    OsError(#[from] OsError),

    #[error(transparent)]
    SurfaceCreationError(#[from] CreateSurfaceError),

    #[error(transparent)]
    RequestDeviceError(#[from] RequestDeviceError),

    #[error("Limits requested couldn't be fulfilled")]
    LimitsSurpassed,

    #[error("Failed to find an appropriate adapter")]
    AdapterCreationError,
}
