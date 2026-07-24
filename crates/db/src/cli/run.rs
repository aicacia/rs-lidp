use std::{fs::read_dir, io, sync::Arc};

use clap::Parser;
use cli::shutdown_signal;
use env_logger::Env;
use libsql::Database;
use tokio::{select, spawn};
use tokio_util::sync::CancellationToken;

use crate::{
    DatabaseConfig,
    cli::args::{CliArgs, CliMigrateCommand},
    close_database,
    migrate::{MigrationFile, down, up},
    open_database,
};

pub async fn run() -> io::Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("failed to load .env file: {}", e);
        }
    }

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = CliArgs::parse();

    let cancellation_token = CancellationToken::new();

    let database_config = DatabaseConfig {
        url: std::env::var("DATABASE_URL").unwrap_or(args.database_url),
        ..Default::default()
    };

    let database = Arc::new(open_database(&database_config).await.map_err(|e| {
        log::error!("failed to create database pool: {}", e);
        io::Error::other(e)
    })?);

    let mut migration_files = Vec::new();
    for result in read_dir(args.migrations)? {
        let entry = result?;
        let path = entry.path();

        if path.is_file() && path.extension().map(|ext| ext == "sql").unwrap_or(false) {
            migration_files.push(MigrationFile {
                name: path
                    .file_name()
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidFilename,
                            "invalid migration file name",
                        )
                    })?
                    .to_string_lossy()
                    .to_string(),
                contents: std::fs::read_to_string(&path).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("failed to read migration file {:?}: {}", path, e),
                    )
                })?,
            })
        }
    }

    let run_up = |database: Arc<Database>, migration_files: Vec<MigrationFile>| {
        spawn(async move {
            up(&database, &migration_files)
                .await
                .map_err(|e| io::Error::other(format!("failed to run database migrations: {}", e)))
        })
    };
    let run_down = |database: Arc<Database>, migration_files: Vec<MigrationFile>| {
        spawn(async move {
            down(&database, &migration_files).await.map_err(|e| {
                io::Error::other(format!("failed to revert database migrations: {}", e))
            })
        })
    };

    let command_handle = match args.command {
        #[cfg(feature = "completions")]
        Some(CliMigrateCommand::Completions { shell }) => {
            spawn(async move { cli::run_completions(shell).await })
        }
        Some(CliMigrateCommand::Up) => run_up(database.clone(), migration_files),
        Some(CliMigrateCommand::Down) => run_down(database.clone(), migration_files),
        None => run_up(database.clone(), migration_files),
    };

    select! {
        _ = shutdown_signal(cancellation_token.clone()) => {
            log::info!("received shutdown signal, cancelling command");
            cancellation_token.cancel();
        }
        res = command_handle => match res {
            Ok(Ok(_)) => log::debug!("command completed successfully"),
            Ok(Err(e)) => log::error!("command error: {}", e),
            Err(e) => log::error!("join error: {}", e),
        }
    }

    close_database(&database).await.map_err(|e| {
        log::error!("failed to close database pool: {}", e);
        io::Error::other(e)
    })?;

    Ok(())
}
