use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Interpreter inner error: {0}")]
    InterpreterError(String),
}
