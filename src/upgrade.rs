use std::process::Command;
use console::style;

#[derive(clap::Parser, Debug)]
pub struct UpgradeArgs {}

pub async fn run_upgrade(_args: UpgradeArgs) -> Result<(), String> {
    println!("  {} {}", style("🍃").green(), style("Upgrading springx CLI to the latest version...").bold().green());
    
    let status = Command::new("sh")
        .arg("-c")
        .arg("curl -sSL https://raw.githubusercontent.com/B-bsw/springboot-initializr-CLI/main/install.sh | bash")
        .status()
        .map_err(|e| format!("Failed to run upgrade script: {}", e))?;

    if status.success() {
        println!("\n  {} {}", style("✔").green(), style("Successfully upgraded springx CLI.").bold());
        Ok(())
    } else {
        Err("Upgrade failed. Please check your network connection or try manually.".to_string())
    }
}
