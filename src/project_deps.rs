use crate::build_tool;
use crate::deps;
use crate::metadata;
use console::style;

#[derive(clap::Parser, Debug)]
pub struct ProjectDepsArgs {}

pub async fn run_deps(_args: ProjectDepsArgs) -> Result<(), String> {
    let (tool, content) = build_tool::detect_build_tool()?;
    
    println!("\nDetected build tool: {}\n", style(tool.as_str()).cyan());

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message("Loading metadata...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    let meta = metadata::fetch_metadata().await?;
    spinner.finish_and_clear();

    let existing_deps = deps::detect_existing_deps(&meta, &content);

    if existing_deps.is_empty() {
        println!("Installed dependencies:\n");
        println!("  {}", style("(No Spring Boot starter dependencies recognized)").dim());
        return Ok(());
    }

    println!("Installed dependencies:\n");
    for dep in existing_deps {
        println!("{} {}", style("✓").green(), dep);
    }
    println!();

    Ok(())
}
