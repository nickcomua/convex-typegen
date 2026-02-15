use std::fmt;

/// Errors that can occur during the type generation process.
#[derive(Debug)]
pub enum ConvexTypeGeneratorError
{
    /// The schema file could not be found at the specified path
    MissingSchemaFile,

    /// The Bun extractor process failed or returned invalid output
    ExtractionFailed
    {
        /// Details about the extraction failure
        details: String,
    },

    /// The provided path doesn't have a valid file name component
    InvalidPath(String),

    /// The file name contains invalid Unicode characters
    InvalidUnicode(String),

    /// Failed to serialize data to JSON
    SerializationFailed(serde_json::Error),

    /// An IO error occurred while reading or writing files
    IOError
    {
        /// Path to the file where the error occurred
        file: String,
        /// The underlying IO error
        error: std::io::Error,
    },

    /// The schema file has invalid structure or content
    InvalidSchema
    {
        /// Context where the invalid schema was found
        context: String,
        /// Details about why the schema is invalid
        details: String,
    },
}

impl fmt::Display for ConvexTypeGeneratorError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::MissingSchemaFile => write!(f, "Schema file not found"),
            Self::ExtractionFailed { details } => {
                write!(f, "Type extraction failed: {}", details)
            }
            Self::InvalidPath(path) => {
                write!(f, "Invalid path: {}", path)
            }
            Self::InvalidUnicode(path) => {
                write!(f, "Path contains invalid Unicode: {}", path)
            }
            Self::SerializationFailed(err) => {
                write!(f, "Failed to serialize: {}", err)
            }
            Self::IOError { file, error } => {
                write!(f, "IO error while reading '{}': {}", file, error)
            }
            Self::InvalidSchema { context, details } => {
                write!(f, "Invalid schema at {}: {}", context, details)
            }
        }
    }
}

impl From<std::io::Error> for ConvexTypeGeneratorError
{
    fn from(error: std::io::Error) -> Self
    {
        ConvexTypeGeneratorError::IOError {
            file: String::new(),
            error,
        }
    }
}

impl std::error::Error for ConvexTypeGeneratorError {}

impl ConvexTypeGeneratorError
{
    /// Adds file context to an IO error
    pub fn with_file_context(self, file: impl Into<String>) -> Self
    {
        match self {
            Self::IOError { error, .. } => Self::IOError {
                file: file.into(),
                error,
            },
            other => other,
        }
    }
}
