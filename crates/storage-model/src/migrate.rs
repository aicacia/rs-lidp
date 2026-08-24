use alloc::{string::ToString, vec::Vec};

use include_dir::{Dir, include_dir};

use db::migrate::MigrationFile;

static MIGRATIONS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

pub async fn up(database: &libsql::Database) -> libsql::Result<()> {
    db::migrate::up(database, &migration_files()).await
}

pub async fn down(database: &libsql::Database) -> libsql::Result<()> {
    db::migrate::down(database, &migration_files()).await
}

fn migration_files() -> Vec<MigrationFile> {
    MIGRATIONS
        .files()
        .map(|file| MigrationFile {
            name: file.path().to_string_lossy().into_owned(),
            contents: file
                .contents_utf8()
                .expect("storage migration must be UTF-8")
                .to_string(),
        })
        .collect()
}
