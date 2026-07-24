use std::{fs::create_dir_all, path::Path};

use libsql::OpenFlags;
use url::Url;

use crate::DatabaseConfig;

pub async fn open_database(database_config: &DatabaseConfig) -> libsql::Result<libsql::Database> {
    if database_config.url == ":memory:" {
        return libsql::Builder::new_local(":memory:").build().await;
    }
    let database_url =
        Url::parse(&database_config.url).map_err(|e| libsql::Error::Misuse(e.to_string()))?;

    log::info!("initializing sqlite database: {}", database_url);
    match database_url.scheme() {
        "file" | "sqlite" => {
            let path = Path::new(database_url.path());
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
                && !parent.exists()
            {
                log::info!("Creating database directory: {:?}", parent);
                match create_dir_all(parent) {
                    Ok(_) => (),
                    Err(e) => {
                        return Err(libsql::Error::Misuse(e.to_string()));
                    }
                }
            }

            libsql::Builder::new_local(database_url.as_str())
                .flags(OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE)
                .build()
                .await
        }
        #[cfg(feature = "remote")]
        "libsql" => {
            libsql::Builder::new_remote(
                database_url.to_string(),
                database_config.auth_token.clone().unwrap_or_default(),
            )
            .build()
            .await
        }
        _ => Err(libsql::Error::Misuse(format!(
            "unsupported database scheme: {}",
            database_url.scheme()
        ))),
    }
}

pub async fn close_database(database: &libsql::Database) -> libsql::Result<()> {
    database
        .connect()?
        .execute_batch("PRAGMA analysis_limit=400; PRAGMA optimize;")
        .await?;
    Ok(())
}
