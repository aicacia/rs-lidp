use clap::Parser;
#[cfg(feature = "completions")]
use cli::Shell;

#[derive(Parser, Debug)]
#[clap(version, about, author)]
pub struct CliArgs {
    #[arg(
        long,
        env = "MIGRATION_DIR",
        short = 'm',
        default_value = "./migrations"
    )]
    pub migrations: String,
    #[arg(long = "database-url", short = 'd', default_value = ":memory:")]
    pub database_url: String,
    #[clap(subcommand)]
    pub command: Option<CliMigrateCommand>,
}

#[derive(Parser, Debug)]
pub enum CliMigrateCommand {
    Up,
    Down,
    #[cfg(feature = "completions")]
    Completions {
        shell: Shell,
    },
}
