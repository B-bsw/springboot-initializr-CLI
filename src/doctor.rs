use console::style;
use std::process::Command;

#[derive(clap::Parser, Debug)]
pub struct DoctorArgs {}

pub async fn run_doctor(_args: DoctorArgs) -> Result<(), String> {
    println!("  {} {}\n", style("🩺").cyan(), style("springx doctor - Environment Validation").bold().cyan());

    let mut all_good = true;

    // Check Java
    match run_cmd("java", &["-version"]) {
        Ok(out) => {
            // Extract version from first line
            let line = out.lines().next().unwrap_or("Java (unknown version)");
            println!("  {} {}", style("✓").green(), line);
        }
        Err(_) => {
            println!("  {} {}", style("✗").red(), style("Java not found (critical)").red());
            all_good = false;
        }
    }

    // Check Git
    match run_cmd("git", &["--version"]) {
        Ok(out) => println!("  {} {}", style("✓").green(), out.lines().next().unwrap_or("Git").trim()),
        Err(_) => println!("  {} {}", style("✗").red(), style("Git not found").red()),
    }

    // Check Maven
    match run_cmd("mvn", &["-v"]) {
        Ok(out) => {
            let line = out.lines().next().unwrap_or("Maven");
            println!("  {} {}", style("✓").green(), line);
        }
        Err(_) => println!("  {} {}", style("!").yellow(), style("Maven not found").yellow()),
    }

    // Check Gradle
    match run_cmd("gradle", &["-v"]) {
        Ok(out) => {
            let line = out.lines().find(|l| l.starts_with("Gradle ")).unwrap_or("Gradle");
            println!("  {} {}", style("✓").green(), line);
        }
        Err(_) => println!("  {} {}", style("!").yellow(), style("Gradle not found").yellow()),
    }

    // Check Docker
    match run_cmd("docker", &["-v"]) {
        Ok(out) => println!("  {} {}", style("✓").green(), out.trim()),
        Err(_) => println!("  {} {}", style("✗").red(), style("Docker not found").red()),
    }

    // Check IntelliJ IDEA
    if check_cmd("idea", &["--version"]) || check_cmd("intellij", &["--version"]) || is_mac_app("IntelliJ IDEA.app") || is_mac_app("IntelliJ IDEA CE.app") {
        println!("  {} {}", style("✓").green(), "IntelliJ IDEA");
    } else {
        println!("  {} {}", style("!").yellow(), style("IntelliJ IDEA not found in PATH or standard locations").yellow());
    }

    // Check VS Code
    match run_cmd("code", &["-v"]) {
        Ok(_) => println!("  {} {}", style("✓").green(), "VS Code"),
        Err(_) => println!("  {} {}", style("!").yellow(), style("VS Code not found").yellow()),
    }

    if !all_good {
        std::process::exit(1);
    }
    
    Ok(())
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, ()> {
    let output = Command::new(cmd).args(args).output().map_err(|_| ())?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // Some tools print version to stderr (e.g. java)
        if !stdout.is_empty() {
            Ok(stdout)
        } else {
            Ok(stderr)
        }
    } else {
        Err(())
    }
}

fn check_cmd(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).output().is_ok()
}

fn is_mac_app(name: &str) -> bool {
    if cfg!(target_os = "macos") {
        std::path::Path::new(&format!("/Applications/{}", name)).exists()
            || std::path::Path::new(&format!("{}/Applications/{}", std::env::var("HOME").unwrap_or_default(), name)).exists()
    } else {
        false
    }
}
