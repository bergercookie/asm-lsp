mod regnerate_docs;
mod schema;

use anyhow::Result;
use clap::{Args, Command, FromArgMatches as _, Subcommand};

#[derive(Subcommand)]
enum Commands {
    /// Generate a JSON schema for `.asm-lsp.toml` files
    Schema(Schema),
    /// Regenerate the serialized document stores for asm-lsp
    #[clap(alias = "regen")]
    RegenerateDocs(RegenerateDocs),
}

#[derive(Args)]
struct Schema;

#[derive(Args)]
struct RegenerateDocs;

#[allow(clippy::missing_errors_doc)]
pub fn main() -> Result<()> {
    let cli = Command::new("xtask")
        .subcommand_required(true)
        .arg_required_else_help(true);

    let command = Commands::from_arg_matches(&Commands::augment_subcommands(cli).get_matches())?;

    match command {
        Commands::Schema(_) => Schema::run()?,
        Commands::RegenerateDocs(_) => RegenerateDocs::run()?,
    }

    Ok(())
}
