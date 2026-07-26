use chrono::{DateTime, Utc};
use key::DerivedKey;
use model::{contract::EntityType, model::Key};

use crate::repo::{KeyRepo, PrivateKeyKeyringRepo, PrivateKeyRepo, RepoResult};

pub struct KeyService<R> {
    key_repo: R,
    private_key_repo: PrivateKeyKeyringRepo,
    namespace: String,
}

impl<R> KeyService<R>
where
    R: KeyRepo,
{
    pub fn new(
        key_repo: R,
        private_key_repo: PrivateKeyKeyringRepo,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            key_repo,
            private_key_repo,
            namespace: namespace.into(),
        }
    }

    pub fn key_repo(&self) -> &R {
        &self.key_repo
    }

    pub fn private_key_repo(&self) -> &PrivateKeyKeyringRepo {
        &self.private_key_repo
    }

    pub async fn create_key(
        &self,
        parent_id: Option<u32>,
        entity_type: EntityType,
        entity_id: i64,
        hardened: bool,
        name: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> RepoResult<(Key, DerivedKey)> {
        let key = self
            .key_repo
            .create_key(
                parent_id,
                entity_type,
                entity_id,
                hardened,
                name,
                expires_at,
            )
            .await?;

        let private_key = self
            .private_key_repo
            .ensure_derivation_path(&self.namespace, key.derivation_path()?)?;

        Ok((key, private_key))
    }
}
