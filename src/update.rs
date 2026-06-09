use crate::metadata;
use crate::deps;
use console::style;

#[derive(clap::Parser, Debug)]
pub struct UpdateArgs {
    pub deps: Vec<String>,
}

pub async fn run_update(args: UpdateArgs) -> Result<(), String> {
    println!("\n  {} {}", style("🍃").green(), style("Updating Dependencies...").bold().green());
    
    let (file_path, content) = deps::get_build_file()?;
    let is_maven = file_path.ends_with("pom.xml");

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message("Loading metadata...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    let meta = metadata::fetch_metadata().await?;
    spinner.finish_and_clear();

    let existing_deps = deps::detect_existing_deps(&meta, &content);

    if existing_deps.is_empty() {
        println!("  {}", style("No recognized Spring dependencies found in project.").dim());
        return Ok(());
    }

    let to_update = if args.deps.is_empty() {
        // Update all existing dependencies
        existing_deps.clone()
    } else {
        // Filter requested dependencies to only those that actually exist in the project
        let mut filtered = Vec::new();
        for d in args.deps {
            if existing_deps.contains(&d) {
                filtered.push(d);
            } else {
                println!("  {} {} is not installed, skipping.", style("⚠️").yellow(), d);
            }
        }
        filtered
    };

    if to_update.is_empty() {
        println!("  {}", style("Nothing to update.").dim());
        return Ok(());
    }

    // Call apply_changes with the same list for remove and add!
    // This will delete the old snippets and inject the latest snippets from Spring Initializr.
    deps::apply_changes(&file_path, &content, to_update.clone(), to_update.clone(), is_maven, "").await?;

    Ok(())
}
