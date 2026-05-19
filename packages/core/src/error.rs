use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("synthesis failed: {0}")]
    Synthesis(String),

    #[error("codegen failed: {0}")]
    Codegen(String),

    #[error("harness error: {0}")]
    Harness(String),
}

pub type Result<T> = std::result::Result<T, Error>;
