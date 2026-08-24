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

    /// Builds a session-scoped storage service rooted under the JWT subject.
    pub fn for_session(&self, sub: &str) -> Result<Self, StorageError> {
        if sub.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "empty session subject".to_string(),
            ));
        }

        let resolved = validate_and_resolve(&self.root, sub)?;
        Ok(Self::new(resolved))
    }

    /// Reads the entire contents of a file at the given relative path.
    ///
    /// The path is validated and resolved relative to the service root.
    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let resolved = validate_and_resolve(&self.root, path)?;
        tokio::fs::read(&resolved).await.map_err(StorageError::from)
    }

    /// Lists the entries in a directory at the given relative path.
    pub async fn list_dir(&self, path: &str) -> Result<Vec<String>, StorageError> {
        let resolved = validate_and_resolve(&self.root, path)?;
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&resolved).await?;

        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry.file_name().to_string_lossy().into_owned());
        }

        entries.sort();
        Ok(entries)
    }

    /// Creates a directory at the given relative path.
    pub async fn create_dir(&self, path: &str) -> Result<(), StorageError> {
        let resolved = validate_and_resolve(&self.root, path)?;
        tokio::fs::create_dir_all(&resolved)
            .await
            .map_err(StorageError::from)
    }

    /// Removes a file or directory at the given relative path.
    pub async fn delete_path(&self, path: &str) -> Result<(), StorageError> {
        let resolved = validate_and_resolve(&self.root, path)?;
        let metadata = tokio::fs::metadata(&resolved).await?;

        if metadata.is_dir() {
            tokio::fs::remove_dir_all(&resolved).await?;
        } else {
            tokio::fs::remove_file(&resolved).await?;
        }

        Ok(())
    }

    /// Renames a file or directory.
    pub async fn rename_path(&self, from: &str, to: &str) -> Result<(), StorageError> {
        let from_resolved = validate_and_resolve(&self.root, from)?;
        let to_resolved = validate_and_resolve(&self.root, to)?;

        if let Some(parent) = to_resolved.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        tokio::fs::rename(&from_resolved, &to_resolved)
            .await
            .map_err(StorageError::from)
    }

    /// Checks whether a path exists.
    pub async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        let resolved = validate_and_resolve(&self.root, path)?;
        tokio::fs::try_exists(&resolved)
            .await
            .map_err(StorageError::from)
    }

    /// Writes content to a file at the given relative path.
    ///
    /// Creates the file if it doesn't exist, and creates parent directories as needed.
    /// Truncates the file if it already exists.
    /// The path is validated and resolved relative to the service root.
    pub async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), StorageError> {
        log::info!(
            "storage write requested: path={path:?}, bytes={}",
            content.len()
        );
        let resolved = validate_and_resolve(&self.root, path)?;

        // Create parent directories if they don't exist
        if let Some(parent) = resolved.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // Write the file
        tokio::fs::write(&resolved, content).await?;
        log::info!("storage write completed: path={resolved:?}");
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

    #[tokio::test]
    async fn test_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        let service = StorageService::new(temp_dir.path());

        service.write_file("b.txt", b"b").await.unwrap();
        service.write_file("a.txt", b"a").await.unwrap();

        let entries = service.list_dir(".").await.unwrap();
        assert_eq!(entries, vec!["a.txt".to_string(), "b.txt".to_string()]);
    }

    #[tokio::test]
    async fn test_create_delete_and_rename_path() {
        let temp_dir = TempDir::new().unwrap();
        let service = StorageService::new(temp_dir.path());

        service.create_dir("folder").await.unwrap();
        assert!(service.exists("folder").await.unwrap());

        service
            .write_file("folder/file.txt", b"hello")
            .await
            .unwrap();
        service
            .rename_path("folder/file.txt", "folder/renamed.txt")
            .await
            .unwrap();

        assert!(service.exists("folder/renamed.txt").await.unwrap());
        assert!(!service.exists("folder/file.txt").await.unwrap());

        service.delete_path("folder").await.unwrap();
        assert!(!service.exists("folder").await.unwrap());
    }
}
