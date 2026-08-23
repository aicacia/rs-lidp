use std::fmt;
use std::io;

#[derive(Debug)]
pub enum StorageError {
    InvalidPath(String),
    PathTraversal,
    NotFound,
    IoError(io::Error),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::InvalidPath(msg) => write!(f, "invalid path: {}", msg),
            StorageError::PathTraversal => write!(f, "path traversal attempt detected"),
            StorageError::NotFound => write!(f, "file not found"),
            StorageError::IoError(err) => write!(f, "io error: {}", err),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => StorageError::NotFound,
            _ => StorageError::IoError(err),
        }
    }
}
