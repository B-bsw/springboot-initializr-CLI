use crate::metadata;
use console::style;

#[derive(clap::Parser, Debug)]
pub struct InfoArgs {
    pub dependency_id: String,
}

pub async fn run_info(args: InfoArgs) -> Result<(), String> {
    let meta = metadata::fetch_metadata().await?;
    let query = args.dependency_id.to_lowercase();

    // Find the dependency exactly, or fuzzily if not exact
    let dep = meta.all_deps.iter()
        .find(|d| d.key == query)
        .or_else(|| meta.all_deps.iter().find(|d| d.key.contains(&query)))
        .ok_or_else(|| format!("Dependency '{}' not found", query))?;

    println!();
    println!("{} {}", style("Name:").bold(), style(&dep.text).cyan());
    println!();
    println!("{} {}", style("ID:").bold(), style(&dep.key).green());
    println!();
    println!("{}", style("Description:").bold());
    if dep.description.is_empty() {
        println!("{}", style("(No description available)").dim());
    } else {
        println!("{}", dep.description);
    }
    println!();
    println!("{}", style("Starter:").bold());
    println!("org.springframework.boot:spring-boot-starter-{}", dep.key);
    println!();

    Ok(())
}
