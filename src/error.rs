use rusqlite;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BencherError {
    #[error("Config file not found")]
    NotFound,

    #[error("Value cannot be empty")]
    EmptyValue,

    #[error("SQLite error")]
    Database(#[from] rusqlite::Error),

    #[error("IO Error")]
    IoError(#[from] std::io::Error),

    #[error("Invalid confidence level: {0}")]
    InvalidConfidence(usize),

    #[error("Point type and error bar type do not match")]
    MismatchedTypes,

    #[error("No experiment type provided. Available experiments: {0}")]
    MissingExperiment(String),

    #[error("No experiment label provided")]
    MissingLabel,

    #[error("No experiment code provided")]
    MissingCode,

    #[error("No lines found for experiment type {0}")]
    NoLines(String),

    #[error("Experiment `{0}` not found. Available experiments: {1}")]
    ExperimentNotFound(String, String),
}

impl From<BencherError> for rusqlite::Error {
    fn from(error: BencherError) -> Self {
        rusqlite::Error::ModuleError(format!("{:?}", error))
    }
}
