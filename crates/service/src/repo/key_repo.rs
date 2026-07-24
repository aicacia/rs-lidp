use model::{contract::EntityType, model::Key};

use crate::repo::RepoResult;

pub trait KeyRepo {
    fn list_active(&self) -> impl Future<Output = RepoResult<Vec<Key>>>;

    fn find_by_id(&self, id: i64) -> impl Future<Output = RepoResult<Option<Key>>>;

    fn active_by_entity_type_and_id(
        &self,
        entity_type: EntityType,
        entity_id: i64,
    ) -> impl Future<Output = RepoResult<Option<Key>>>;

    fn create_key(
        &self,
        entity_type: EntityType,
        entity_id: i64,
        hardened: bool,
        name: String,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl Future<Output = RepoResult<Key>>;
}
