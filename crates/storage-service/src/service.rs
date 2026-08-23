use std::path::{Path, PathBuf};

use crate::error::StorageError;
use crate::fs::validate_and_resolve;

/// Service for managing file storage with security-enforced path validation.
#[derive(Debug)]
pub struct StorageService {
    root: PathBuf,
}

impl StorageService {
    /// Creates a new StorageService with the given root directory.
    ///
    /// The root directory will be used as the base for all file operations.
    /// All paths passed to read/write operations must be relative to this root.
    pub fn new(root: impl AsRef<Path>) -> Self {
        StorageService {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Reads the entire contents of a file at the given relative path.
    ///
    /// The path is validated and resolved relative to the service root.
    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let resolved = validate_and_resolve(&self.root, path)?;
        tokio::fs::read(&resolved).await.map_err(StorageError::from)
    }

    /// Writes content to a file at the given relative path.
    ///
    /// Creates the file if it doesn't exist, and creates parent directories as needed.
    /// Truncates the file if it already exists.
    /// The path is validated and resolved relative to the service root.
    pub async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), StorageError> {
        let resolved = validate_and_resolve(&self.root, path)?;

        // Create parent directories if they don't exist
        if let Some(parent) = resolved.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // Write the file
        tokio::fs::write(&resolved, content).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_and_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let service = StorageService::new(temp_dir.path());

        let content = b"Hello, Storage!";
        service.write_file("test.txt", content).await.unwrap();

        let read_content = service.read_file("test.txt").await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_write_file_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let service = StorageService::new(temp_dir.path());

        let content = b"Nested file";
        service
            .write_file("folder/subfolder/file.txt", content)
            .await
            .unwrap();

        let read_content = service
            .read_file("folder/subfolder/file.txt")
            .await
            .unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let service = StorageService::new(temp_dir.path());

        let result = service.read_file("nonexistent.txt").await;
        assert!(matches!(result, Err(StorageError::NotFound)));
    }

    #[tokio::test]
    async fn test_reject_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let service = StorageService::new(temp_dir.path());

        let result = service.read_file("/etc/passwd").await;
        assert!(matches!(result, Err(StorageError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn test_reject_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let service = StorageService::new(temp_dir.path());

        let result = service.read_file("../../../etc/passwd").await;
        assert!(matches!(result, Err(StorageError::PathTraversal)));
    }

    #[tokio::test]
    async fn test_overwrite_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let service = StorageService::new(temp_dir.path());

        service.write_file("test.txt", b"First").await.unwrap();
        service.write_file("test.txt", b"Second").await.unwrap();

        let content = service.read_file("test.txt").await.unwrap();
        assert_eq!(content, b"Second");
    }
}
