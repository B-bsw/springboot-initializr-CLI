use std::process::Command;
use console::style;

pub fn init_git(project_path: &std::path::Path) {
    println!("  {} {}", style("📦").cyan(), style("Initializing Git repository...").cyan());
    
    // Check if git is available
    if Command::new("git").arg("--version").output().is_err() {
        println!("  {} {}", style("!").yellow(), style("Git not found, skipping initialization.").yellow());
        return;
    }

    if project_path.join(".git").exists() {
        println!("  {} {}", style("✓").green(), style("Git repository already exists.").dim());
        return;
    }

    let init = Command::new("git").arg("init").current_dir(project_path).output();
    if let Ok(out) = init {
        if !out.status.success() {
            println!("  {} {}", style("!").yellow(), style("Failed to initialize git repository.").yellow());
            return;
        }
    }

    let add = Command::new("git").arg("add").arg(".").current_dir(project_path).output();
    if let Ok(out) = add {
        if !out.status.success() {
            println!("  {} {}", style("!").yellow(), style("Failed to add files to git.").yellow());
            return;
        }
    }

    let commit = Command::new("git").arg("commit").arg("-m").arg("Initial commit").current_dir(project_path).output();
    if let Ok(out) = commit {
        if out.status.success() {
            println!("  {} {}", style("✓").green(), style("Git repository initialized and initial commit created.").green());
        } else {
            println!("  {} {}", style("!").yellow(), style("Failed to create initial commit.").yellow());
        }
    }
}
