mod metadata;
mod generate;
mod interactive;
mod version;
mod deps;
mod update;
mod upgrade;
mod build_tool;
mod doctor;
mod search;
mod info;
mod project_deps;
mod template;
mod git;
mod docker;

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
    #[command(alias = "new")]
    Init(generate::NewArgs),
    /// List available options (boot versions, dependencies, etc.)
    List(metadata::ListArgs),
    /// Add/install dependencies to an existing project
    #[command(alias = "install")]
    Add(deps::AddArgs),
    /// Remove/uninstall dependencies from an existing project
    #[command(alias = "uninstall")]
    Remove(deps::RemoveArgs),
    /// Update project dependencies to their latest Spring Initializr snippets
    Update(update::UpdateArgs),
    /// Upgrade the springx CLI itself to the latest version
    Upgrade(upgrade::UpgradeArgs),
    /// Validate local development environment
    Doctor(doctor::DoctorArgs),
    /// Search for dependencies
    Search(search::SearchArgs),
    /// Get info about a specific dependency
    Info(info::InfoArgs),
    /// Inspect installed dependencies in the current project
    Deps(project_deps::ProjectDepsArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Init(args)) => {
            if args.project.is_none() && args.language.is_none() && args.boot.is_none() && args.name.is_none() && args.group.is_none() && args.artifact.is_none() && args.package_name.is_none() && args.packaging.is_none() && args.java.is_none() && args.config_format.is_none() && args.deps.is_empty() && args.output.is_none() && args.ide.is_none() && args.template.is_none() && !args.flat && !args.git && !args.docker {
                interactive::run_interactive().await
            } else {
                generate::run(args).await
            }
        },
        Some(Commands::List(_args)) => metadata::list().await,
        Some(Commands::Add(args)) => deps::run_add(args).await,
        Some(Commands::Remove(args)) => deps::run_remove(args).await,
        Some(Commands::Update(args)) => update::run_update(args).await,
        Some(Commands::Upgrade(args)) => upgrade::run_upgrade(args).await,
        Some(Commands::Doctor(args)) => doctor::run_doctor(args).await,
        Some(Commands::Search(args)) => search::run_search(args).await,
        Some(Commands::Info(args)) => info::run_info(args).await,
        Some(Commands::Deps(args)) => project_deps::run_deps(args).await,
        None => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            let _ = cmd.print_help();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("\x1b[31m✗ Error:\x1b[0m {e}");
        std::process::exit(1);
    }
}
