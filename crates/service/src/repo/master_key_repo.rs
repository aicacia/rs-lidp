use key::MasterKey;

use crate::repo::RepoResult;

pub trait MasterKeyRepo {
    fn load(&self, name: &str) -> impl Future<Output = RepoResult<Option<MasterKey>>>;

    fn save<T>(&self, name: &str, seed: T) -> impl Future<Output = RepoResult<()>>
    where
        T: AsRef<[u8]>;

    fn delete(&self, name: &str) -> impl Future<Output = RepoResult<()>>;

    fn create_or_load(&self, name: &str) -> impl Future<Output = RepoResult<MasterKey>> {
        let this = self;
        async move {
            match this.load(name).await? {
                Some(master_key) => {
                    log::debug!("Master key loaded");
                    Ok(master_key)
                }
                None => {
                    log::debug!("Master key not found in repository, creating new one");
                    let master_key_entropy = MasterKey::entropy()?;
                    let master_key = MasterKey::from_entropy(&master_key_entropy)?;
                    this.save(name, &master_key_entropy).await?;
                    Ok(master_key)
                }
            }
        }
    }
}
