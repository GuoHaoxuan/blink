use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlinkError {
    #[error("unknown error occurred")]
    Unknown,
}
