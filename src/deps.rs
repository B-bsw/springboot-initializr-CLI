use crate::metadata::{self, Metadata};
use crate::interactive;
use dialoguer::theme::ColorfulTheme;
use std::fs;
use console::style;

#[derive(clap::Parser, Debug)]
pub struct AddArgs {
    #[arg(value_delimiter = ',')]
    pub deps: Vec<String>,
}

#[derive(clap::Parser, Debug)]
pub struct RemoveArgs {
    #[arg(value_delimiter = ',')]
    pub deps: Vec<String>,
}

pub async fn run_add(args: AddArgs) -> Result<(), String> {
    process_deps(args.deps, true).await
}

pub async fn run_remove(args: RemoveArgs) -> Result<(), String> {
    process_deps(args.deps, false).await
}

async fn process_deps(mut input_deps: Vec<String>, _is_add: bool) -> Result<(), String> {
    let (tool, content) = crate::build_tool::detect_build_tool()?;
    let file_path = tool.file_name().to_string();
    let is_maven = tool.is_maven();

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message("Loading metadata...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    let meta = metadata::fetch_metadata().await?;
    spinner.finish_and_clear();

    let existing_deps = detect_existing_deps(&meta, &content);

    // Get the boot version from build file if possible, or use default
    let boot_version = extract_boot_version(&content).unwrap_or_else(|| meta.defaults.boot.clone());

    if input_deps.is_empty() {
        // Interactive mode
        println!();
        let theme = ColorfulTheme::default();
        
        if !_is_add {
            if existing_deps.is_empty() {
                println!("  {}", style("No recognized Spring dependencies found to remove.").dim());
                return Ok(());
            }
            
            let mut installed_deps = Vec::new();
            for dep_key in &existing_deps {
                let text = meta.all_deps.iter().find(|d| d.key == *dep_key).map(|d| d.text.clone()).unwrap_or_else(|| dep_key.clone());
                installed_deps.push(metadata::DepOption {
                    key: dep_key.clone(),
                    text,
                    description: "Select to REMOVE this dependency".to_string(),
                    group: "Installed".to_string(),
                    version_range: None,
                });
            }
            
            let custom_meta = metadata::Metadata {
                dependency_groups: vec![metadata::DepGroup {
                    name: "Installed Dependencies".to_string(),
                    deps: installed_deps.clone(),
                }],
                all_deps: installed_deps,
                projects: vec![],
                languages: vec![],
                boot_versions: vec![],
                java_versions: vec![],
                packagings: vec![],
                config_formats: vec![],
                defaults: meta.defaults.clone(),
            };

            println!("  {} {}", style("🗑").red(), style("Select dependencies to remove").bold().red());
            let selected = interactive::select_dependencies(&theme, &custom_meta, &boot_version, &[])?;
            
            if selected.is_empty() {
                println!("  {}", style("No dependencies selected for removal.").dim());
                return Ok(());
            }

            let to_remove: Vec<_> = selected.into_iter().map(|d| d.key).collect();
            
            apply_changes(&file_path, &content, vec![], to_remove, is_maven, &boot_version).await?;
            return Ok(());
        }

        println!("  {} {}", style("🍃").green(), style("Manage Dependencies").bold().green());
        let selected = interactive::select_dependencies(&theme, &meta, &boot_version, &existing_deps)?;
        let selected_keys: std::collections::HashSet<_> = selected.into_iter().map(|d| d.key).collect();
        let existing_set: std::collections::HashSet<_> = existing_deps.into_iter().collect();

        let to_add: Vec<_> = selected_keys.difference(&existing_set).cloned().collect();
        let to_remove: Vec<_> = existing_set.difference(&selected_keys).cloned().collect();

        if to_add.is_empty() && to_remove.is_empty() {
            println!("  {}", style("No changes to dependencies.").dim());
            return Ok(());
        }

        apply_changes(&file_path, &content, to_add, to_remove, is_maven, &boot_version).await?;
    } else {
        // CLI mode (no interactive prompt)
        // Check if dependencies exist in metadata
        let mut resolved_deps = Vec::new();
        for d in &input_deps {
            if let Some(matched) = meta.all_deps.iter().find(|dep| dep.key == *d) {
                resolved_deps.push(matched.key.clone());
            } else if let Some(matched) = meta.all_deps.iter().find(|dep| dep.key.contains(d)) {
                println!("  {} Resolving '{}' to '{}'", style("ℹ").cyan(), d, matched.key);
                resolved_deps.push(matched.key.clone());
            } else {
                return Err(format!("Unknown dependency ID: {}", d));
            }
        }
        input_deps = resolved_deps;
        
        // Remove duplicates
        input_deps.sort();
        input_deps.dedup();

        if _is_add {
            let to_add: Vec<_> = input_deps.into_iter().filter(|d| !existing_deps.contains(d)).collect();
            if to_add.is_empty() {
                println!("  {}", style("All specified dependencies are already installed.").dim());
                return Ok(());
            }
            apply_changes(&file_path, &content, to_add, vec![], is_maven, &boot_version).await?;
        } else {
            let to_remove: Vec<_> = input_deps.into_iter().filter(|d| existing_deps.contains(d)).collect();
            if to_remove.is_empty() {
                println!("  {}", style("None of the specified dependencies are installed.").dim());
                return Ok(());
            }
            apply_changes(&file_path, &content, vec![], to_remove, is_maven, &boot_version).await?;
        }
    }

    Ok(())
}



pub fn detect_existing_deps(meta: &Metadata, content: &str) -> Vec<String> {
    let mut existing = Vec::new();
    for dep in &meta.all_deps {
        let key = &dep.key;
        let s1 = format!("starter-{}", key);
        let s2 = format!(">{}<", key);
        let s3 = format!(":{}", key);
        let s4 = format!("'{}'", key);
        let s5 = format!("\"{}\"", key);
        // This heuristic handles most cases
        if content.contains(&s1) || content.contains(&s2) || content.contains(&s3) || content.contains(&s4) || content.contains(&s5) {
            existing.push(key.clone());
        }
    }
    existing
}

fn extract_boot_version(content: &str) -> Option<String> {
    // Very basic extraction logic
    if let Some(idx) = content.find("<version>") {
        if let Some(end) = content[idx..].find("</version>") {
            let ver = &content[idx + 9..idx + end];
            if ver.starts_with('2') || ver.starts_with('3') || ver.starts_with('4') {
                return Some(ver.trim().to_string());
            }
        }
    }
    None
}

pub async fn apply_changes(file_path: &str, original_content: &str, to_add: Vec<String>, to_remove: Vec<String>, is_maven: bool, _boot_version: &str) -> Result<(), String> {
    let mut new_content = original_content.to_string();
    let mut changed = false;
    
    println!();
    
    if !to_remove.is_empty() {
        for dep in &to_remove {
            println!("  {} {}", style("-").red(), style(dep).bold());
            if is_maven {
                // simple greedy remove for maven
                let search = format!("<artifactId>spring-boot-starter-{}</artifactId>", dep);
                if let Some(idx) = new_content.find(&search).or_else(|| new_content.find(&format!("<artifactId>{}</artifactId>", dep))) {
                    if let Some(start) = new_content[..idx].rfind("<dependency>") {
                        if let Some(end) = new_content[idx..].find("</dependency>") {
                            new_content.replace_range(start..idx + end + 13, "");
                            changed = true;
                        }
                    }
                }
            } else {
                // simple remove for gradle
                let search = format!("spring-boot-starter-{}", dep);
                if let Some(idx) = new_content.find(&search).or_else(|| new_content.find(dep)) {
                    if let Some(line_start) = new_content[..idx].rfind('\n') {
                        if let Some(line_end) = new_content[idx..].find('\n') {
                            new_content.replace_range(line_start..idx + line_end, "");
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    if !to_add.is_empty() {
        let joined = to_add.join(",");
        let url = format!("https://start.spring.io/pom.xml?dependencies={}", joined);
        if let Ok(resp) = reqwest::get(&url).await {
            if let Ok(pom) = resp.text().await {
                if let Some(start) = pom.find("<dependencies>") {
                    if let Some(end) = pom.find("</dependencies>") {
                        let deps_block = &pom[start + 14..end];
                        let mut blocks_to_add = String::new();

                        for dep in &to_add {
                            println!("  {} {}", style("+").green(), style(dep).bold());
                            if let Some(d_start) = deps_block.find(&format!("<artifactId>spring-boot-starter-{}</artifactId>", dep)).or_else(|| deps_block.find(&format!("<artifactId>{}</artifactId>", dep))) {
                                let section = &deps_block[..d_start + 20];
                                if let Some(tag_start) = section.rfind("<dependency>") {
                                    if let Some(d_end) = deps_block[d_start..].find("</dependency>") {
                                        blocks_to_add.push_str(&deps_block[tag_start..d_start + d_end + 13]);
                                        blocks_to_add.push('\n');
                                    }
                                }
                            }
                        }

                        if !blocks_to_add.is_empty() {
                            if is_maven {
                                if let Some(insert_pos) = new_content.rfind("</dependencies>") {
                                    new_content.insert_str(insert_pos, &format!("{}\n    ", blocks_to_add));
                                    changed = true;
                                }
                            } else {
                                // For Gradle, just append implementation strings roughly
                                let mut gradle_deps = String::new();
                                for dep in &to_add {
                                    gradle_deps.push_str(&format!("    implementation 'org.springframework.boot:spring-boot-starter-{}'\n", dep));
                                }
                                if let Some(insert_pos) = new_content.rfind("dependencies {") {
                                    if let Some(brace_end) = new_content[insert_pos..].find('}') {
                                        new_content.insert_str(insert_pos + brace_end, &format!("\n{}\n", gradle_deps));
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    if changed {
        fs::write(file_path, new_content).map_err(|e| format!("Failed to update build file: {}", e))?;
        println!("\n  {} Successfully updated {}", style("✔").green(), file_path);
    } else {
        println!("\n  {} Could not automatically modify {}. Please update manually.", style("⚠️").yellow(), file_path);
    }

    Ok(())
}
