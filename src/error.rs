use anyhow::anyhow;

/// Custom error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A generically presented error.
    #[error("Error: {0}")]
    General(#[from] anyhow::Error),
    /// An error that shows the user the usage information.
    #[error("Usage: midi <file>")]
    Usage,
}

impl Error {
    /// Constructs a new instance of `Error::General` with the given message.
    pub fn general(message: &str) -> Self {
        Self::General(anyhow!("{}", message))
    }
}
