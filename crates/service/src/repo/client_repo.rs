use model::{contract::ClientRegistration, model::Client};

use crate::repo::RepoResult;

pub trait ClientRepo {
    fn find_client_by_client_id(
        &self,
        client_id: &str,
    ) -> impl Future<Output = RepoResult<Option<Client>>>;

    fn create_client(&self, client: ClientRegistration)
    -> impl Future<Output = RepoResult<Client>>;

    fn update_client(&self, client: Client) -> impl Future<Output = RepoResult<Client>>;

    fn delete_client_by_client_id(&self, client_id: &str) -> impl Future<Output = RepoResult<()>>;
}
