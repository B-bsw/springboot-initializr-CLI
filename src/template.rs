use std::fs;
use std::path::Path;
use console::style;

pub trait ProjectTemplate {
    fn apply(&self, project_path: &Path, package_name: &str) -> Result<(), String>;
}

pub struct CleanArchitectureTemplate;

impl ProjectTemplate for CleanArchitectureTemplate {
    fn apply(&self, project_path: &Path, package_name: &str) -> Result<(), String> {
        let package_path = package_name.replace('.', "/");
        
        let java_dir = project_path.join("src/main/java").join(&package_path);
        let kotlin_dir = project_path.join("src/main/kotlin").join(&package_path);
        let groovy_dir = project_path.join("src/main/groovy").join(&package_path);

        // Determine which language dir exists
        let base_dir = if kotlin_dir.exists() {
            kotlin_dir
        } else if groovy_dir.exists() {
            groovy_dir
        } else {
            java_dir
        };

        let dirs = ["controller", "service", "repository", "entity", "dto", "config"];

        for dir in dirs {
            let path = base_dir.join(dir);
            fs::create_dir_all(&path).map_err(|e| format!("Failed to create directory {:?}: {}", path, e))?;
            
            // Add a .gitkeep so git tracks the empty directory
            fs::write(path.join(".gitkeep"), "").unwrap_or_default();
        }

        println!("  {} {}", style("✓").green(), style("Applied clean-architecture template.").dim());

        Ok(())
    }
}

pub fn apply_template(template_name: &str, project_path: &Path, package_name: &str) -> Result<(), String> {
    println!("  {} {}", style("🏗").cyan(), style(format!("Applying template: {}", template_name)).cyan());

    match template_name.to_lowercase().as_str() {
        "clean-architecture" => {
            let tpl = CleanArchitectureTemplate;
            tpl.apply(project_path, package_name)
        }
        _ => Err(format!("Unknown template: {}. Available templates: clean-architecture", template_name)),
    }
}
