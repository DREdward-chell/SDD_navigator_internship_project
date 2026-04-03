use thiserror::Error;

/// All errors produced by the sdd-core library.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("IO error reading '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("YAML parse error{}: {message}",
        .line.map(|l| format!(" at line {l}")).unwrap_or_default())]
    Yaml {
        line: Option<usize>,
        message: String,
    },

    #[error("Validation error for '{id}': {message}")]
    Validation { id: String, message: String },
}

pub type Result<T> = std::result::Result<T, CoreError>;
