mod schema;

use anyhow::Result;
use clap::{Args, Command, FromArgMatches as _, Subcommand};

#[derive(Subcommand)]
enum Commands {
    /// Generate a JSON schema for `.asm-lsp.toml` files
    Schema(Schema),
}

#[derive(Args)]
struct Schema;

#[allow(clippy::missing_errors_doc)]
pub fn main() -> Result<()> {
    let cli = Command::new("xtask")
        .subcommand_required(true)
        .arg_required_else_help(true);

    let command = Commands::from_arg_matches(&Commands::augment_subcommands(cli).get_matches())?;

    match command {
        Commands::Schema(_) => Schema::run()?,
    }

    Ok(())
}
