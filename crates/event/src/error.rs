use graphics::ContextError;
use thiserror::Error;
use winit::error::EventLoopError;

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    ContextCreation(#[from] ContextError),

    #[error(transparent)]
    EvenLoop(#[from] EventLoopError),
}
