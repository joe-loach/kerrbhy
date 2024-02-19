use graphics::ContextBuildError;
use thiserror::Error;
use winit::error::EventLoopError;

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    ContextCreation(#[from] ContextBuildError),

    #[error(transparent)]
    EvenLoop(#[from] EventLoopError),
}
