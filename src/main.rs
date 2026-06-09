mod metadata;
mod generate;
mod interactive;
mod version;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "springx",
    version,
    about = "🍃 Spring Initializr CLI — generate Spring Boot projects from the terminal",
    long_about = "A fast CLI client for start.spring.io.\nGenerate Spring Boot projects interactively or via flags, right from your terminal."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Spring Boot project (interactive wizard)
    New(generate::NewArgs),
    /// List available options (boot versions, dependencies, etc.)
    List(metadata::ListArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::New(args)) => generate::run(args).await,
        Some(Commands::List(args)) => metadata::list(args).await,
        None => interactive::run_interactive().await,
    };

    if let Err(e) = result {
        eprintln!("\x1b[31m✗ Error:\x1b[0m {e}");
        std::process::exit(1);
    }
}
