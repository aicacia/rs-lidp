use model::model::{ManagementRole, ManagementUserRole};

use crate::repo::RepoResult;

pub trait ManagementRoleRepo {
    fn list_roles(&self, offset: u32, limit: u32) -> impl Future<Output = RepoResult<Vec<ManagementRole>>>;

    fn create_role(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> impl Future<Output = RepoResult<ManagementRole>>;

    fn find_role_by_id(
        &self,
        role_id: i64,
    ) -> impl Future<Output = RepoResult<Option<ManagementRole>>>;

    fn delete_role_by_id(&self, role_id: i64) -> impl Future<Output = RepoResult<()>>;

    fn list_user_roles(
        &self,
        user_id: i64,
    ) -> impl Future<Output = RepoResult<Vec<ManagementUserRole>>>;

    fn assign_role_to_user(
        &self,
        user_id: i64,
        role_id: i64,
    ) -> impl Future<Output = RepoResult<()>>;

    fn revoke_role_from_user(
        &self,
        user_id: i64,
        role_id: i64,
    ) -> impl Future<Output = RepoResult<()>>;

    fn count_user_role_assignments(&self) -> impl Future<Output = RepoResult<u64>>;
}
