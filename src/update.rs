use crate::metadata;
use crate::deps;
use console::style;

#[derive(clap::Parser, Debug)]
pub struct UpdateArgs {
    #[arg(value_delimiter = ',')]
    pub deps: Vec<String>,
}

pub async fn run_update(args: UpdateArgs) -> Result<(), String> {
    println!("\n  {} {}", style("🍃").green(), style("Updating Dependencies...").bold().green());
    
    let (tool, content) = crate::build_tool::detect_build_tool()?;
    let file_path = tool.file_name().to_string();
    let is_maven = tool.is_maven();

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
            let mut resolved = d.clone();
            if let Some(matched) = meta.all_deps.iter().find(|dep| dep.key == d) {
                resolved = matched.key.clone();
            } else if let Some(matched) = meta.all_deps.iter().find(|dep| dep.key.contains(&d)) {
                println!("  {} Resolving '{}' to '{}'", style("ℹ").cyan(), d, matched.key);
                resolved = matched.key.clone();
            }
            
            if existing_deps.contains(&resolved) {
                filtered.push(resolved);
            } else {
                println!("  {} {} is not installed, skipping.", style("⚠️").yellow(), resolved);
            }
        }
        filtered
    };

    if to_update.is_empty() {
        println!("  {}", style("Nothing to update.").dim());
        return Ok(());
    }

    let boot_version = deps::extract_boot_version(&content).unwrap_or_else(|| meta.defaults.boot.clone());
    // Call apply_changes with the same list for remove and add!
    // It will remove existing versions and add the latest snippets!
    deps::apply_changes(&file_path, &content, to_update.clone(), to_update.clone(), is_maven, &meta, &boot_version).await?;

    Ok(())
}
