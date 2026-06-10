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
            let selected = interactive::select_dependencies(&theme, &custom_meta, &boot_version, &existing_deps, false)?;
            
            if selected.is_empty() {
                println!("  {}", style("No dependencies selected for removal.").dim());
                return Ok(());
            }

            let to_remove: Vec<_> = selected.into_iter().map(|d| d.key).collect();
            
            apply_changes(&file_path, &content, vec![], to_remove, is_maven, &meta, &boot_version).await?;
            return Ok(());
        }

        println!("  {} {}", style("🍃").green(), style("Add Dependencies").bold().green());
        let selected = interactive::select_dependencies(&theme, &meta, &boot_version, &existing_deps, true)?;
        let selected_keys: std::collections::HashSet<_> = selected.into_iter().map(|d| d.key).collect();

        // Everything selected in add menu is to be added
        let to_add: Vec<_> = selected_keys.into_iter().collect();
        let to_remove: Vec<_> = vec![];

        if to_add.is_empty() {
            println!("  {}", style("No dependencies selected to add.").dim());
            return Ok(());
        }

        apply_changes(&file_path, &content, to_add, to_remove, is_maven, &meta, &boot_version).await?;
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
            apply_changes(&file_path, &content, to_add, vec![], is_maven, &meta, &boot_version).await?;
        } else {
            let to_remove: Vec<_> = input_deps.into_iter().filter(|d| existing_deps.contains(d)).collect();
            if to_remove.is_empty() {
                println!("  {}", style("None of the specified dependencies are installed.").dim());
                return Ok(());
            }
            apply_changes(&file_path, &content, vec![], to_remove, is_maven, &meta, &boot_version).await?;
        }
    }

    Ok(())
}




fn extract_artifact_ids_from_content(content: &str) -> Vec<String> {
    let mut ids = Vec::new();

    if content.contains("<project") {
        let mut current_idx = 0;
        while let Some(start) = content[current_idx..].find("<artifactId>") {
            let actual_start = current_idx + start + 12;
            if let Some(end) = content[actual_start..].find("</artifactId>") {
                let artifact_id = &content[actual_start..actual_start + end];
                ids.push(artifact_id.trim().to_string());
                current_idx = actual_start + end + 13;
            } else {
                break;
            }
        }
    } else {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("implementation") || line.starts_with("compileOnly") 
                || line.starts_with("developmentOnly") || line.starts_with("annotationProcessor")
                || line.starts_with("testImplementation") || line.starts_with("testCompileOnly")
                || line.starts_with("testRuntimeOnly") || line.starts_with("testAnnotationProcessor")
                || line.starts_with("runtimeOnly") || line.starts_with("api") || line.starts_with("id ") {
                
                let mut in_quote = false;
                let mut quote_char = ' ';
                let mut current_str = String::new();
                
                for c in line.chars() {
                    if in_quote {
                        if c == quote_char {
                            in_quote = false;
                            if current_str.contains(':') {
                                let parts: Vec<&str> = current_str.split(':').collect();
                                if parts.len() >= 2 {
                                    ids.push(parts[1].trim().to_string());
                                }
                            } else {
                                if !current_str.contains(' ') {
                                    ids.push(current_str.clone());
                                }
                            }
                            current_str.clear();
                        } else {
                            current_str.push(c);
                        }
                    } else if c == '\'' || c == '"' {
                        in_quote = true;
                        quote_char = c;
                    }
                }
            }
        }
    }
    ids
}

pub fn detect_existing_deps(meta: &Metadata, content: &str) -> Vec<String> {
    let ids = extract_artifact_ids_from_content(content);

    let mut existing = std::collections::HashSet::new();
    for artifact_id in ids {
        // Manual overrides for dependencies whose artifactIds completely diverge from their Spring Initializr IDs
        match artifact_id.as_str() {
            "spring-rabbit-stream" => { existing.insert("amqp-streams".to_string()); continue; },
            "jcc" => { existing.insert("db2".to_string()); continue; },
            "spring-cloud-azure-starter" => { existing.insert("azure-support".to_string()); continue; },
            "spring-cloud-azure-starter-active-directory" => { existing.insert("azure-active-directory".to_string()); continue; },
            "spring-cloud-azure-starter-cosmos" => { existing.insert("azure-cosmos-db".to_string()); continue; },
            "spring-cloud-azure-starter-keyvault" => { existing.insert("azure-keyvault".to_string()); continue; },
            "spring-cloud-azure-starter-storage" => { existing.insert("azure-storage".to_string()); continue; },
            "spring-pulsar-reactive-spring-boot-starter" => { existing.insert("pulsar-reactive".to_string()); continue; },
            "solace-spring-boot-starter" => { existing.insert("solace".to_string()); continue; },
            "camel-spring-boot-starter" => { existing.insert("camel".to_string()); continue; },
            "spring-data-rest-hal-explorer" => { existing.insert("data-rest-explorer".to_string()); continue; },
            "graphql-dgs-spring-graphql-starter" => { existing.insert("netflix-dgs".to_string()); continue; },
            "com.netflix.dgs.codegen" => { existing.insert("dgs-codegen".to_string()); continue; },
            _ => {}
        }
        
        let mut best_match: Option<String> = None;
        let mut best_len = 0;

        for dep in &meta.all_deps {
            let key = &dep.key;
            let key_no_hyphen = key.replace("-", "");
            
            let mut variants = vec![key.clone(), key_no_hyphen.clone()];
            if key.starts_with("spring-") {
                let stripped = key.replace("spring-", "");
                variants.push(stripped.clone());
                variants.push(stripped.replace("-", ""));
            }

            for v in variants {
                if artifact_id.contains(&v) {
                    if v.len() > best_len {
                        best_len = v.len();
                        best_match = Some(key.clone());
                    }
                }
            }
        }
        
        if let Some(m) = best_match {
            existing.insert(m);
        }
    }

    let mut res: Vec<String> = existing.into_iter().collect();
    res.sort();
    res
}

pub fn extract_boot_version(content: &str) -> Option<String> {
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

pub async fn apply_changes(file_path: &str, original_content: &str, to_add: Vec<String>, to_remove: Vec<String>, is_maven: bool, meta: &crate::metadata::Metadata, boot_version: &str) -> Result<(), String> {
    let mut new_content = original_content.to_string();
    let mut changed = false;
    
    println!();
    
    if !to_remove.is_empty() {
        let ids = extract_artifact_ids_from_content(&new_content);
        let mut artifacts_to_remove = std::collections::HashSet::new();
        
        for artifact_id in ids {
            let mut best_match: Option<String> = None;
            
            match artifact_id.as_str() {
                "spring-rabbit-stream" => { best_match = Some("amqp-streams".to_string()); },
                "jcc" => { best_match = Some("db2".to_string()); },
                "spring-cloud-azure-starter" => { best_match = Some("azure-support".to_string()); },
                "spring-cloud-azure-starter-active-directory" => { best_match = Some("azure-active-directory".to_string()); },
                "spring-cloud-azure-starter-cosmos" => { best_match = Some("azure-cosmos-db".to_string()); },
                "spring-cloud-azure-starter-keyvault" => { best_match = Some("azure-keyvault".to_string()); },
                "spring-cloud-azure-starter-storage" => { best_match = Some("azure-storage".to_string()); },
                "spring-pulsar-reactive-spring-boot-starter" => { best_match = Some("pulsar-reactive".to_string()); },
                "solace-spring-boot-starter" => { best_match = Some("solace".to_string()); },
                "camel-spring-boot-starter" => { best_match = Some("camel".to_string()); },
                "spring-data-rest-hal-explorer" => { best_match = Some("data-rest-explorer".to_string()); },
                "graphql-dgs-spring-graphql-starter" => { best_match = Some("netflix-dgs".to_string()); },
                "com.netflix.dgs.codegen" => { best_match = Some("dgs-codegen".to_string()); },
                _ => {}
            }
            
            if best_match.is_none() {
            for dep in &meta.all_deps {
                let key = &dep.key;
                let key_no_hyphen = key.replace("-", "");
                let mut variants = vec![key.clone(), key_no_hyphen.clone()];
                if key.starts_with("spring-") {
                    let stripped = key.replace("spring-", "");
                    variants.push(stripped.clone());
                    variants.push(stripped.replace("-", ""));
                }
                for v in variants {
                    if artifact_id.contains(&v) {
                        let current_len = best_match.as_ref().map_or(0, |m: &String| m.len());
                        if v.len() > current_len {
                            best_match = Some(key.clone());
                        }
                    }
                }
                }
            }
            if let Some(m) = best_match {
                if to_remove.contains(&m) {
                    artifacts_to_remove.insert(artifact_id);
                }
            }
        }

        for dep in &to_remove {
            println!("  {} {}", console::style("-").red(), console::style(dep).bold());
        }

        if is_maven {
            let mut current_idx = 0;
            while current_idx < new_content.len() {
                let mut found_idx = None;
                let mut best_rel_idx = None;
                for artifact_id in &artifacts_to_remove {
                    let search = format!("<artifactId>{}</artifactId>", artifact_id);
                    if let Some(rel_idx) = new_content[current_idx..].find(&search) {
                        if best_rel_idx.map_or(true, |best| rel_idx < best) {
                            best_rel_idx = Some(rel_idx);
                        }
                    }
                }
                if let Some(rel_idx) = best_rel_idx {
                    found_idx = Some(current_idx + rel_idx);
                }

                if let Some(idx) = found_idx {
                    if let Some(start) = new_content[..idx].rfind("<dependency>") {
                        if let Some(end) = new_content[idx..].find("</dependency>") {
                            new_content.replace_range(start..idx + end + 13, "");
                            changed = true;
                            current_idx = start;
                            continue;
                        }
                    }
                    current_idx = idx + 1;
                } else {
                    break;
                }
            }
        } else {
            let mut current_idx = 0;
            while current_idx < new_content.len() {
                let mut found_idx = None;
                let mut best_rel_idx = None;
                for artifact_id in &artifacts_to_remove {
                    if let Some(rel_idx) = new_content[current_idx..].find(artifact_id) {
                        if best_rel_idx.map_or(true, |best| rel_idx < best) {
                            best_rel_idx = Some(rel_idx);
                        }
                    }
                }
                if let Some(rel_idx) = best_rel_idx {
                    found_idx = Some(current_idx + rel_idx);
                }

                if let Some(idx) = found_idx {
                    if let Some(start) = new_content[..idx].rfind('\n') {
                        if let Some(end) = new_content[idx..].find('\n') {
                            new_content.replace_range(start..idx + end, "");
                            changed = true;
                            current_idx = start;
                            continue;
                        }
                    }
                    current_idx = idx + 1;
                } else {
                    break;
                }
            }
        }
    }


    if !to_add.is_empty() {
        let joined = to_add.join(",");
        let url = if is_maven {
            format!("https://start.spring.io/pom.xml?dependencies={}&bootVersion={}", joined, boot_version)
        } else {
            format!("https://start.spring.io/build.gradle?dependencies={}&bootVersion={}", joined, boot_version)
        };
        
        if let Ok(resp) = reqwest::get(&url).await {
            if let Ok(downloaded_text) = resp.text().await {
                let mut lines_to_add = String::new();
                
                if is_maven {
                    if let Some(start) = downloaded_text.find("<dependencies>") {
                        if let Some(end) = downloaded_text.find("</dependencies>") {
                            let deps_block = &downloaded_text[start..end];
                            let mut current_idx = 0;
                            while let Some(d_start) = deps_block[current_idx..].find("<dependency>") {
                                let actual_start = current_idx + d_start;
                                if let Some(d_end) = deps_block[actual_start..].find("</dependency>") {
                                    let block = &deps_block[actual_start..actual_start + d_end + 13];
                                    
                                    // Extract artifactId
                                    if let Some(a_start) = block.find("<artifactId>") {
                                        if let Some(a_end) = block[a_start..].find("</artifactId>") {
                                            let artifact_id = &block[a_start + 12..a_start + a_end];
                                            
                                            if artifact_id == "spring-boot-starter" 
                                                || (artifact_id == "spring-boot-starter-test" && !to_add.contains(&"test".to_string()))
                                                || (artifact_id == "junit-platform-launcher" && !to_add.contains(&"test".to_string())) {
                                                // skip default dependencies
                                            } else {
                                                println!("  {} {}", console::style("+").green(), console::style(artifact_id).bold());
                                                lines_to_add.push_str(block);
                                                lines_to_add.push('\n');
                                            }
                                        }
                                    }
                                    current_idx = actual_start + d_end + 13;
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    // Gradle
                    if let Some(start) = downloaded_text.find("dependencies {") {
                        if let Some(end) = downloaded_text[start..].find("}") {
                            let deps_block = &downloaded_text[start..start + end];
                            for line in deps_block.lines() {
                                let line_t = line.trim();
                                if line_t.starts_with("implementation") || line_t.starts_with("compileOnly") 
                                    || line_t.starts_with("developmentOnly") || line_t.starts_with("annotationProcessor")
                                    || line_t.starts_with("testImplementation") || line_t.starts_with("testCompileOnly")
                                    || line_t.starts_with("testRuntimeOnly") || line_t.starts_with("testAnnotationProcessor")
                                    || line_t.starts_with("runtimeOnly") || line_t.starts_with("api") || line_t.starts_with("id ") {
                                    
                                    // Extract artifact ID
                                    let mut in_quote = false;
                                    let mut quote_char = ' ';
                                    let mut current_str = String::new();
                                    let mut extracted_id = String::new();
                                    
                                    for c in line_t.chars() {
                                        if in_quote {
                                            if c == quote_char {
                                                in_quote = false;
                                                if current_str.contains(':') {
                                                    let parts: Vec<&str> = current_str.split(':').collect();
                                                    if parts.len() >= 2 {
                                                        extracted_id = parts[1].trim().to_string();
                                                    }
                                                } else {
                                                    if !current_str.contains(' ') {
                                                        extracted_id = current_str.clone();
                                                    }
                                                }
                                                current_str.clear();
                                            } else {
                                                current_str.push(c);
                                            }
                                        } else if c == '\'' || c == '"' {
                                            in_quote = true;
                                            quote_char = c;
                                        }
                                    }
                                    
                                    if !extracted_id.is_empty() {
                                        if extracted_id == "spring-boot-starter" 
                                            || (extracted_id == "spring-boot-starter-test" && !to_add.contains(&"test".to_string()))
                                            || (extracted_id == "junit-platform-launcher" && !to_add.contains(&"test".to_string())) {
                                            // skip
                                        } else {
                                            println!("  {} {}", console::style("+").green(), console::style(extracted_id.as_str()).bold());
                                            lines_to_add.push_str("    ");
                                            lines_to_add.push_str(line_t);
                                            lines_to_add.push('\n');
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !lines_to_add.is_empty() {
                    if is_maven {
                        if let Some(insert_pos) = new_content.rfind("</dependencies>") {
                            new_content.insert_str(insert_pos, &format!("    {}\n", lines_to_add));
                            changed = true;
                        }
                    } else {
                        if let Some(insert_pos) = new_content.rfind("dependencies {") {
                            if let Some(brace_end) = new_content[insert_pos..].find('}') {
                                new_content.insert_str(insert_pos + brace_end, &format!("\n{}\n", lines_to_add));
                                changed = true;
                            }
                        }
                    }
                }

                // --- PLUGIN INJECTION LOGIC ---
                if is_maven {
                    // Maven plugin extraction
                    if let Some(build_start) = downloaded_text.find("<build>") {
                        if let Some(plugins_start) = downloaded_text[build_start..].find("<plugins>") {
                            let actual_plugins_start = build_start + plugins_start;
                            if let Some(plugins_end) = downloaded_text[actual_plugins_start..].find("</plugins>") {
                                let plugins_block = &downloaded_text[actual_plugins_start..actual_plugins_start + plugins_end];
                                
                                let mut current_idx = 0;
                                let mut plugins_to_add = String::new();
                                while let Some(p_start) = plugins_block[current_idx..].find("<plugin>") {
                                    let actual_p_start = current_idx + p_start;
                                    if let Some(p_end) = plugins_block[actual_p_start..].find("</plugin>") {
                                        let block = &plugins_block[actual_p_start..actual_p_start + p_end + 9];
                                        if !block.contains("<artifactId>spring-boot-maven-plugin</artifactId>") {
                                            // Extract artifact id for printing
                                            if let Some(a_start) = block.find("<artifactId>") {
                                                if let Some(a_end) = block[a_start..].find("</artifactId>") {
                                                    let a_id = &block[a_start + 12..a_start + a_end];
                                                    println!("  {} {} (Plugin)", console::style("+").green(), console::style(a_id).bold());
                                                }
                                            }
                                            plugins_to_add.push_str("            ");
                                            plugins_to_add.push_str(block.trim());
                                            plugins_to_add.push_str("\n");
                                        }
                                        current_idx = actual_p_start + p_end + 9;
                                    } else {
                                        break;
                                    }
                                }
                                
                                if !plugins_to_add.is_empty() {
                                    if let Some(b_start) = new_content.find("<build>") {
                                        if let Some(p_start) = new_content[b_start..].find("<plugins>") {
                                            new_content.insert_str(b_start + p_start + 9, &format!("\n{}\n", plugins_to_add));
                                            changed = true;
                                        } else {
                                            new_content.insert_str(b_start + 7, &format!("\n        <plugins>\n{}\n        </plugins>\n", plugins_to_add));
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Gradle plugin extraction
                    if let Some(plugins_start) = downloaded_text.find("plugins {") {
                        if let Some(plugins_end) = downloaded_text[plugins_start..].find("}") {
                            let plugins_block = &downloaded_text[plugins_start..plugins_start + plugins_end];
                            let mut plugins_to_add = String::new();
                            for line in plugins_block.lines() {
                                let line_t = line.trim();
                                if line_t.starts_with("id ") && !line_t.contains("id 'java'") && !line_t.contains("id 'org.springframework.boot'") 
                                    && !line_t.contains("id 'io.spring.dependency-management'") && !line_t.contains("id 'org.jetbrains.kotlin") 
                                    && !line_t.contains("id 'groovy'") {
                                    println!("  {} {} (Plugin)", console::style("+").green(), console::style(line_t).bold());
                                    plugins_to_add.push_str("    ");
                                    plugins_to_add.push_str(line_t);
                                    plugins_to_add.push('\n');
                                }
                            }
                            
                            if !plugins_to_add.is_empty() {
                                if let Some(target_plugins_start) = new_content.find("plugins {") {
                                    if let Some(target_plugins_end) = new_content[target_plugins_start..].find("}") {
                                        new_content.insert_str(target_plugins_start + target_plugins_end, &format!("{}\n", plugins_to_add));
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                    
                    if let Some(gen_start) = downloaded_text.find("generateJava {") {
                        if let Some(gen_end) = downloaded_text[gen_start..].find("}") {
                            let gen_block = &downloaded_text[gen_start..gen_start + gen_end + 1];
                            println!("  {} {} (Config Block)", console::style("+").green(), console::style("generateJava").bold());
                            if !new_content.contains("generateJava {") {
                                new_content.push_str("\n");
                                new_content.push_str(gen_block);
                                new_content.push_str("\n");
                                changed = true;
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
