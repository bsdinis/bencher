use rusqlite;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BencherError {
    #[error("Config file not found")]
    NotFound,

    #[error("Datapoint is missing x value")]
    MissingXValue,

    #[error("Datapoint is missing y value")]
    MissingYValue,

    #[error("SQLite error")]
    Database(#[from] rusqlite::Error),

    #[error("IO Error")]
    IoError(#[from] std::io::Error),

    #[error("Invalid confidence level: {0}")]
    InvalidConfidence(usize),

    #[error("Point type and error bar type do not match")]
    MismatchedTypes,
}

impl From<BencherError> for rusqlite::Error {
    fn from(error: BencherError) -> Self {
        rusqlite::Error::ModuleError(format!("{:?}", error))
    }
}
