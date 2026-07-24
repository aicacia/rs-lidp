use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

use key::MasterKey;
use keyring_core::Entry;

use crate::repo::{MasterKeyRepo, RepoResult};

static SET_CREDENTIAL_STORE: AtomicBool = AtomicBool::new(false);

pub struct MasterKeyKeyringRepo {
    service_name: String,
}

impl MasterKeyKeyringRepo {
    pub fn new(service_name: impl Into<String>) -> Self {
        init_credential_store().expect("Failed to initialize credential store");

        Self {
            service_name: service_name.into(),
        }
    }
}

impl MasterKeyRepo for MasterKeyKeyringRepo {
    async fn load(&self, name: &str) -> RepoResult<Option<MasterKey>> {
        let entry = Entry::new(&self.service_name, name)?;

        match entry.get_secret() {
            Ok(secret_bytes) => {
                let master_key = MasterKey::from_entropy(secret_bytes)?;
                Ok(Some(master_key))
            }
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn save<T>(&self, name: &str, seed: T) -> RepoResult<()>
    where
        T: AsRef<[u8]>,
    {
        let entry = create_entry(&self.service_name, name)?;
        entry.set_secret(seed.as_ref())?;
        Ok(())
    }

    async fn delete(&self, name: &str) -> RepoResult<()> {
        let entry = Entry::new(&self.service_name, name)?;
        entry.delete_credential()?;
        Ok(())
    }
}

fn create_entry(service: &str, user: &str) -> keyring_core::Result<Entry> {
    let mut modifiers = HashMap::new();
    modifiers.insert("target", "OIDC Master Key");
    let entry = Entry::new_with_modifiers(service, user, &modifiers)?;
    Ok(entry)
}

fn init_credential_store() -> keyring_core::Result<()> {
    if SET_CREDENTIAL_STORE.compare_exchange(false, true, Ordering::Release, Ordering::Acquire)
        == Ok(false)
    {
        set_credential_store()?;
    }
    Ok(())
}

fn set_credential_store() -> keyring_core::Result<()> {
    #[cfg(target_os = "macos")]
    let store = apple_native_keyring_store::keychain::Store::new()?;
    #[cfg(target_os = "windows")]
    let store = windows_native_keyring_store::Store::new()?;
    #[cfg(all(target_os = "linux", feature = "headless"))]
    let store = linux_keyutils_keyring_store::Store::new()?;
    #[cfg(all(target_os = "linux", not(feature = "headless")))]
    let store = dbus_secret_service_keyring_store::Store::new()?;
    #[cfg(target_os = "android")]
    let store = android_native_keyring_store::Store::new()?;
    keyring_core::set_default_store(store);
    Ok(())
}
