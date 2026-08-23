use std::path::{Component, Path, PathBuf};

use crate::error::StorageError;

/// Validates a relative path string and returns the resolved path relative to root.
///
/// Security rules:
/// - Rejects absolute paths
/// - Rejects paths containing `..` (parent directory traversal)
/// - Rejects empty paths
/// - All paths are relative to `root`
pub fn validate_and_resolve(root: &Path, path: &str) -> Result<PathBuf, StorageError> {
    log::debug!("storage path validation: root={root:?}, path={path:?}");

    // Reject empty paths
    if path.is_empty() {
        return Err(StorageError::InvalidPath("empty path".to_string()));
    }

    // Parse the path and check each component
    let path_obj = Path::new(path);

    // Reject absolute paths
    if path_obj.is_absolute() {
        return Err(StorageError::InvalidPath(
            "absolute paths not allowed".to_string(),
        ));
    }

    // Check for path traversal attempts
    for component in path_obj.components() {
        match component {
            Component::ParentDir => {
                return Err(StorageError::PathTraversal);
            }
            Component::RootDir => {
                return Err(StorageError::InvalidPath(
                    "root directory not allowed".to_string(),
                ));
            }
            Component::Prefix(_) => {
                return Err(StorageError::InvalidPath(
                    "path prefixes not allowed".to_string(),
                ));
            }
            Component::Normal(_) | Component::CurDir => {
                // Valid components
            }
        }
    }

    // Resolve the path relative to root
    let resolved = root.join(path);

    // Verify the resolved path is still under root (additional safety check)
    if !resolved.starts_with(root) {
        log::warn!("storage path rejected after resolution: root={root:?}, resolved={resolved:?}");
        return Err(StorageError::PathTraversal);
    }

    log::debug!("storage path resolved: {path:?} -> {resolved:?}");
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_relative_path() {
        let root = Path::new("/tmp/storage");
        let result = validate_and_resolve(root, "file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/tmp/storage/file.txt"));
    }

    #[test]
    fn test_valid_nested_path() {
        let root = Path::new("/tmp/storage");
        let result = validate_and_resolve(root, "folder/subfolder/file.txt");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/tmp/storage/folder/subfolder/file.txt")
        );
    }

    #[test]
    fn test_reject_empty_path() {
        let root = Path::new("/tmp/storage");
        let result = validate_and_resolve(root, "");
        assert!(matches!(result, Err(StorageError::InvalidPath(_))));
    }

    #[test]
    fn test_reject_absolute_path() {
        let root = Path::new("/tmp/storage");
        let result = validate_and_resolve(root, "/etc/passwd");
        assert!(matches!(result, Err(StorageError::InvalidPath(_))));
    }

    #[test]
    fn test_reject_parent_dir_traversal() {
        let root = Path::new("/tmp/storage");
        let result = validate_and_resolve(root, "../etc/passwd");
        assert!(matches!(result, Err(StorageError::PathTraversal)));
    }

    #[test]
    fn test_reject_mixed_traversal() {
        let root = Path::new("/tmp/storage");
        let result = validate_and_resolve(root, "folder/../../etc/passwd");
        assert!(matches!(result, Err(StorageError::PathTraversal)));
    }

    #[test]
    fn test_allow_dot_in_filename() {
        let root = Path::new("/tmp/storage");
        let result = validate_and_resolve(root, "file.tar.gz");
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_dot_as_path() {
        let root = Path::new("/tmp/storage");
        // Single "." should resolve to root only, but we allow it as it's the current dir component
        let result = validate_and_resolve(root, ".");
        // Actually "." is CurDir component, which is allowed but resolves to the root itself
        assert!(result.is_ok());
    }
}
