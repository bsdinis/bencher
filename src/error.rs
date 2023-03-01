use std::collections::HashSet;

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

    #[error("IO Error: {message}")]
    IoError {
        #[source]
        source: std::io::Error,
        message: String,
    },

    #[error("Unknown experiment type: {0}")]
    UnknownExperimentType(String),

    #[error("Invalid confidence level: {0}")]
    InvalidConfidence(usize),

    #[error("Point type and error bar type do not match")]
    MismatchedBarTypes,

    #[error("Cannot have with both linear and bidimensional confidences")]
    IncompatibleBarTypes,

    #[error("No experiment type provided. Available experiments: {0}")]
    MissingExperiment(String),

    #[error("No experiment label provided")]
    MissingLabel,

    #[error("No experiment code provided")]
    MissingCode,

    #[error("The experiment code {0} exists with type {1}, cannot add idempotently with type {2}")]
    MismatchedType(String, String, String),

    #[error(
        "The experiment code {0} exists with label {1}, cannot add idempotently with label {2}"
    )]
    MismatchedLabel(String, String, String),

    #[error("No lines found for experiment type {0}")]
    NoLines(String),

    #[error("No sets found for experiment type {0}")]
    NoSets(String),

    #[error("Experiment `{0}` not found. Available experiments: {1}")]
    ExperimentNotFound(String, String),

    #[error("Deserialization Error")]
    Serde(#[from] serde_json::Error),

    #[error("Incompatible databases for view: {db} has codes which are already in other databases: {codes:?}")]
    IncompatibleDbs {
        db: std::path::PathBuf,
        codes: HashSet<String>,
    },

    #[error("Duplicate experiment: already have experiment with code {0}")]
    DuplicateExperiment(String),

    #[error("Schema error: missing table {0} in db {1}")]
    SchemaMissingTable(String, String),

    #[error("Failed to create path from prefix {}: cannot add extension {}", .0.to_string_lossy(), .1)]
    PathCreateError(std::path::PathBuf, String),
}

impl BencherError {
    pub fn io_err(source: std::io::Error, message: impl ToString) -> BencherError {
        BencherError::IoError {
            source,
            message: message.to_string(),
        }
    }
}

impl From<BencherError> for rusqlite::Error {
    fn from(error: BencherError) -> Self {
        rusqlite::Error::ModuleError(format!("{:?}", error))
    }
}

pub type BencherResult<T> = std::result::Result<T, BencherError>;
