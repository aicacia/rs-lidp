use std::path::{Path, PathBuf};

use crate::{StorageError, fs::validate_and_resolve};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Read,
    Write,
    List,
    Metadata,
    Delete,
    Rename,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderGrant {
    pub root: PathBuf,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Default)]
pub struct StoragePolicy {
    grants: Vec<FolderGrant>,
}

impl StoragePolicy {
    pub fn new(grants: impl IntoIterator<Item = FolderGrant>) -> Self {
        Self {
            grants: grants.into_iter().collect(),
        }
    }

    pub fn authorize(
        &self,
        storage_root: &Path,
        path: &str,
        operation: Operation,
    ) -> Result<PathBuf, StorageError> {
        let resolved = validate_and_resolve(storage_root, path)?;
        let canonical = match resolved.canonicalize() {
            Ok(path) => path,
            Err(_) => {
                let parent = resolved
                    .parent()
                    .ok_or_else(|| StorageError::InvalidPath("missing parent".into()))?
                    .canonicalize()
                    .map_err(StorageError::from)?;
                parent.join(resolved.file_name().unwrap_or_default())
            }
        };

        if self.grants.iter().any(|grant| {
            canonical.starts_with(&grant.root)
                && grant.operations.contains(&operation)
                && (canonical == grant.root || canonical.starts_with(grant.root.join("")))
        }) {
            Ok(canonical)
        } else {
            Err(StorageError::InvalidPath(
                "storage capability denied".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn denies_by_default_and_prevents_prefix_confusion() {
        let temp = TempDir::new().unwrap();
        let allowed = temp.path().join("a/b");
        let private = temp.path().join("a/b-private");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&private).unwrap();
        let policy = StoragePolicy::new([FolderGrant {
            root: allowed.clone(),
            operations: vec![Operation::Read],
        }]);
        assert!(
            policy
                .authorize(temp.path(), "a/b/file", Operation::Read)
                .is_ok()
        );
        assert!(
            policy
                .authorize(temp.path(), "a/b-private/file", Operation::Read)
                .is_err()
        );
    }
}
