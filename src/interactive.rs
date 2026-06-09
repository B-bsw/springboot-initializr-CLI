use crate::generate;
use crate::metadata::{self, DepOption};
use console::style;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use std::path::PathBuf;

pub async fn run_interactive() -> Result<(), String> {
    println!();
    println!(
        "  {} {}",
        style("🍃").green(),
        style("Spring Initializr").bold().green()
    );
    println!("  {}", style("Interactive project generator").dim());
    println!();

    // Fetch metadata
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message("Loading metadata...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    let meta = metadata::fetch_metadata().await?;
    spinner.finish_and_clear();

    let theme = ColorfulTheme::default();

    // ── Project type ───────────────────────────────────────────────────
    let project_idx = select_option(
        &theme,
        "Project type",
        &meta.projects,
        &meta.defaults.project,
    )?;
    let project = &meta.projects[project_idx];

    // ── Language ────────────────────────────────────────────────────────
    let lang_idx = select_option(&theme, "Language", &meta.languages, &meta.defaults.language)?;
    let language = &meta.languages[lang_idx];

    // ── Boot version ───────────────────────────────────────────────────
    let boot_idx = select_option(
        &theme,
        "Spring Boot version",
        &meta.boot_versions,
        &meta.defaults.boot,
    )?;
    let boot = &meta.boot_versions[boot_idx];

    // ── Project metadata ───────────────────────────────────────────────
    println!();
    println!("  {}", style("Project Metadata").bold().underlined());

    let name: String = Input::with_theme(&theme)
        .with_prompt("  Name")
        .default(meta.defaults.name.clone())
        .interact_text()
        .map_err(|e| format!("Input error: {e}"))?;

    let group: String = Input::with_theme(&theme)
        .with_prompt("  Group")
        .default(meta.defaults.group.clone())
        .interact_text()
        .map_err(|e| format!("Input error: {e}"))?;

    let artifact: String = Input::with_theme(&theme)
        .with_prompt("  Artifact")
        .default(meta.defaults.artifact.clone())
        .interact_text()
        .map_err(|e| format!("Input error: {e}"))?;

    let safe_artifact = artifact.replace("-", "").replace("_", "");
    let default_package = format!("{group}.{safe_artifact}");
    let package_name: String = Input::with_theme(&theme)
        .with_prompt("  Package name")
        .default(default_package)
        .interact_text()
        .map_err(|e| format!("Input error: {e}"))?;

    // ── Packaging ──────────────────────────────────────────────────────
    let pkg_idx = select_option(
        &theme,
        "Packaging",
        &meta.packagings,
        &meta.defaults.packaging,
    )?;
    let packaging = &meta.packagings[pkg_idx];

    // ── Java version ───────────────────────────────────────────────────
    let java_idx = select_option(
        &theme,
        "Java version",
        &meta.java_versions,
        &meta.defaults.java,
    )?;
    let java = &meta.java_versions[java_idx];

    // ── Config format ──────────────────────────────────────────────────
    let cfg_idx = select_option(
        &theme,
        "Configuration format",
        &meta.config_formats,
        &meta.defaults.config_format,
    )?;
    let config_format = &meta.config_formats[cfg_idx];

    // ── Dependencies ───────────────────────────────────────────────────
    println!();
    println!("  {}", style("Dependencies").bold().underlined());
    println!(
        "  {}",
        style("Select dependencies (space to toggle, enter to confirm)").dim()
    );

    let selected_deps = select_dependencies(&theme, &meta, &boot.key, &[])?;

    // ── Output directory ───────────────────────────────────────────────
    println!();
    let default_output = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let output_str: String = Input::with_theme(&theme)
        .with_prompt("  Output directory")
        .default(default_output)
        .interact_text()
        .map_err(|e| format!("Input error: {e}"))?;
    let output_dir = PathBuf::from(shellexpand(&output_str));

    // ── IDE ────────────────────────────────────────────────────────────
    let mut ide_options = vec!["None".to_string()];
    let mut ide_commands = vec![None];

    if is_ide_available("idea") {
        ide_options.push("idea (IntelliJ IDEA)".to_string());
        ide_commands.push(Some("idea".to_string()));
    }
    if is_ide_available("code") {
        ide_options.push("code (VS Code)".to_string());
        ide_commands.push(Some("code".to_string()));
    }

    ide_options.push("Other".to_string());
    ide_commands.push(Some("other".to_string()));

    let ide_idx = Select::with_theme(&theme)
        .with_prompt("  Open in IDE after generation?")
        .items(&ide_options)
        .default(0)
        .interact()
        .map_err(|e| format!("Select error: {e}"))?;

    let ide = match ide_commands[ide_idx].as_deref() {
        None => None,
        Some("other") => {
            let custom: String = Input::with_theme(&theme)
                .with_prompt("  IDE command")
                .interact_text()
                .map_err(|e| format!("Input error: {e}"))?;
            Some(custom)
        }
        Some(cmd) => Some(cmd.to_string()),
    };

    // ── Extras (Git, Docker, Template) ─────────────────────────────────
    let use_git = dialoguer::Confirm::with_theme(&theme)
        .with_prompt("  Initialize a Git repository?")
        .default(true)
        .interact()
        .unwrap_or(false);

    let use_docker = dialoguer::Confirm::with_theme(&theme)
        .with_prompt("  Generate Dockerfile and docker-compose.yml?")
        .default(false)
        .interact()
        .unwrap_or(false);

    let template_options = vec!["None", "clean-architecture"];
    let template_idx = Select::with_theme(&theme)
        .with_prompt("  Apply a project template?")
        .items(&template_options)
        .default(0)
        .interact()
        .unwrap_or(0);
    
    let template = if template_idx == 0 {
        None
    } else {
        Some(template_options[template_idx].to_string())
    };

    // ── Generate ───────────────────────────────────────────────────────
    let dep_keys: Vec<String> = selected_deps.iter().map(|d| d.key.clone()).collect();

    let args = generate::NewArgs {
        project: Some(project.key.clone()),
        language: Some(language.key.clone()),
        boot: Some(boot.key.clone()),
        name: Some(name),
        group: Some(group),
        artifact: Some(artifact),
        package_name: Some(package_name),
        packaging: Some(packaging.key.clone()),
        java: Some(java.key.clone()),
        config_format: Some(config_format.key.clone()),
        deps: dep_keys,
        output: Some(output_dir),
        ide,
        flat: false,
        git: use_git,
        docker: use_docker,
        template,
    };

    generate::run(args).await
}

fn select_option(
    theme: &ColorfulTheme,
    prompt: &str,
    options: &[metadata::Option_],
    default_key: &str,
) -> Result<usize, String> {
    let labels: Vec<String> = options.iter().map(|o| o.text.clone()).collect();
    let default_idx = options
        .iter()
        .position(|o| o.key == default_key)
        .unwrap_or(0);

    Select::with_theme(theme)
        .with_prompt(format!("  {prompt}"))
        .items(&labels)
        .default(default_idx)
        .interact()
        .map_err(|e| format!("Selection error: {e}"))
}

const PAGE_SIZE: usize = 10;

/// A single page: a slice of deps from one group, with display title.
struct DepPage {
    title: String,
    deps: Vec<DepOption>,
}

fn build_pages(meta: &metadata::Metadata) -> Vec<DepPage> {
    let mut pages: Vec<DepPage> = Vec::new();
    for group in &meta.dependency_groups {
        if group.deps.is_empty() {
            continue;
        }

        let chunks: Vec<&[DepOption]> = group.deps.chunks(PAGE_SIZE).collect();
        let total_chunks = chunks.len();
        for (i, chunk) in chunks.iter().enumerate() {
            let title = if total_chunks == 1 {
                group.name.clone()
            } else {
                format!("{} ({}/{})", group.name, i + 1, total_chunks)
            };
            pages.push(DepPage {
                title,
                deps: chunk.to_vec(),
            });
        }
    }
    pages
}

pub fn select_dependencies(
    _theme: &ColorfulTheme,
    meta: &metadata::Metadata,
    boot_version: &str,
    pre_selected: &[String],
) -> Result<Vec<DepOption>, String> {
    use crate::version::is_boot_version_in_range;
    let pages = build_pages(meta);
    if pages.is_empty() {
        return Ok(Vec::new());
    }

    let term = console::Term::stderr();
    term.hide_cursor().ok();

    let mut selected_keys: std::collections::HashSet<String> = pre_selected.iter().cloned().collect();
    let mut page_idx: usize = 0;
    let mut cursor: usize = 0;
    let total_pages = pages.len();

    loop {
        let page = &pages[page_idx];
        let dep_count = page.deps.len();

        // Clamp cursor
        if cursor >= dep_count {
            cursor = if dep_count > 0 { dep_count - 1 } else { 0 };
        }

        // Count total selected
        let total_selected: usize = selected_keys.len();

        // ── Render ─────────────────────────────────────────────────
        term.clear_screen().ok();

        // Title
        let title = format!(
            "  {} {}",
            style("🍃").green(),
            style("Dependencies").bold().green()
        );
        let _ = term.write_line(&title);

        // Navigation hint
        let nav = format!(
            "  {}",
            style("◀ ▶ switch group  ▲ ▼ move  ␣ toggle  ⏎ done").dim()
        );
        let _ = term.write_line(&nav);
        let _ = term.write_line("");

        // Page header with pagination indicator
        let page_indicator = format!("{}/{}", page_idx + 1, total_pages);
        let group_header = format!(
            "  {} {} {}",
            style("◀").dim(),
            style(format!(" {} ", page.title)).bold().yellow(),
            style(format!("▶  {}", style(page_indicator).dim())).dim(),
        );
        let _ = term.write_line(&group_header);
        let _ = term.write_line("");

        // Dependencies list
        for (i, dep) in page.deps.iter().enumerate() {
            let is_compatible =
                is_boot_version_in_range(boot_version, dep.version_range.as_deref());
            let is_selected = selected_keys.contains(&dep.key) && is_compatible;
            let is_cursor = i == cursor;

            let checkbox = if !is_compatible {
                style("⨯").red().to_string()
            } else if is_selected {
                style("◉").green().bold().to_string()
            } else {
                style("○").dim().to_string()
            };

            let mut name_styled = if !is_compatible {
                style(&dep.text).dim().to_string()
            } else if is_cursor {
                if is_selected {
                    style(&dep.text).green().bold().to_string()
                } else {
                    style(&dep.text).white().bold().to_string()
                }
            } else if is_selected {
                style(&dep.text).green().to_string()
            } else {
                style(&dep.text).to_string()
            };

            if !is_compatible {
                let raw_range = dep.version_range.as_deref().unwrap_or("");
                let formatted_range = crate::version::format_version_range(raw_range);
                name_styled = format!(
                    "{} {}",
                    name_styled,
                    style(format!("(Requires Boot {})", formatted_range)).red()
                );
            }

            let pointer = if is_cursor {
                style("❯").cyan().to_string()
            } else {
                " ".to_string()
            };

            let _ = term.write_line(&format!("  {} {} {}", pointer, checkbox, name_styled));
        }

        // Footer: selected summary
        let _ = term.write_line("");
        if total_selected > 0 {
            let selected_names: Vec<&str> = meta
                .all_deps
                .iter()
                .filter(|d| selected_keys.contains(&d.key))
                .map(|d| d.text.as_str())
                .collect();

            let summary_text = if selected_names.len() <= 6 {
                selected_names.join(", ")
            } else {
                format!(
                    "{}, ... +{} more",
                    selected_names[..5].join(", "),
                    selected_names.len() - 5
                )
            };
            let _ = term.write_line(&format!(
                "  {} {}",
                style(format!("{} selected:", total_selected))
                    .green()
                    .bold(),
                style(summary_text).green()
            ));
        } else {
            let _ = term.write_line(&format!("  {}", style("No dependencies selected").dim()));
        }

        // ── Read key ───────────────────────────────────────────────
        let key = term
            .read_key()
            .map_err(|e| format!("Terminal error: {e}"))?;

        match key {
            console::Key::ArrowLeft => {
                page_idx = if page_idx == 0 {
                    total_pages - 1
                } else {
                    page_idx - 1
                };
                cursor = 0;
            }
            console::Key::ArrowRight => {
                page_idx = (page_idx + 1) % total_pages;
                cursor = 0;
            }
            console::Key::ArrowUp => {
                if dep_count > 0 {
                    cursor = if cursor == 0 {
                        dep_count - 1
                    } else {
                        cursor - 1
                    };
                }
            }
            console::Key::ArrowDown => {
                if dep_count > 0 {
                    cursor = (cursor + 1) % dep_count;
                }
            }
            console::Key::Char(' ') => {
                if dep_count > 0 {
                    let dep = &page.deps[cursor];
                    if is_boot_version_in_range(boot_version, dep.version_range.as_deref()) {
                        if selected_keys.contains(&dep.key) {
                            selected_keys.remove(&dep.key);
                        } else {
                            selected_keys.insert(dep.key.clone());
                        }
                    }
                }
            }
            console::Key::Enter => {
                break;
            }
            console::Key::Escape => {
                break;
            }
            _ => {}
        }
    }

    // Cleanup terminal
    term.clear_screen().ok();
    term.show_cursor().ok();

    // Collect selected deps in order
    let result: Vec<DepOption> = meta
        .all_deps
        .iter()
        .filter(|d| selected_keys.contains(&d.key))
        .cloned()
        .collect();

    // Print confirmation
    if result.is_empty() {
        println!(
            "  {} {}",
            style("◉").dim(),
            style("No dependencies selected").dim()
        );
    } else {
        println!(
            "  {} {} {}",
            style("◉").green(),
            style(format!("{} dependencies:", result.len()))
                .green()
                .bold(),
            style(
                result
                    .iter()
                    .map(|d| d.text.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .green()
        );
    }

    Ok(result)
}

fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn is_ide_available(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let exe = dir.join(cmd);
            if exe.is_file() {
                return true;
            }
            #[cfg(target_os = "windows")]
            {
                if dir.join(format!("{}.exe", cmd)).is_file()
                    || dir.join(format!("{}.cmd", cmd)).is_file()
                {
                    return true;
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        if cmd == "idea" {
            let paths = [
                "/Applications/IntelliJ IDEA.app".to_string(),
                "/Applications/IntelliJ IDEA CE.app".to_string(),
                format!(
                    "{}/Applications/JetBrains Toolbox/IntelliJ IDEA Ultimate.app",
                    home
                ),
                format!(
                    "{}/Applications/JetBrains Toolbox/IntelliJ IDEA Community Edition.app",
                    home
                ),
            ];
            for p in &paths {
                if std::path::Path::new(p).exists() {
                    return true;
                }
            }
            if let Ok(output) = std::process::Command::new("mdfind")
                .arg("kMDItemCFBundleIdentifier == 'com.jetbrains.intellij' || kMDItemCFBundleIdentifier == 'com.jetbrains.intellij.ce'")
                .output()
            {
                if !output.stdout.is_empty() && output.stdout.iter().any(|&b| !b.is_ascii_whitespace()) {
                    return true;
                }
            }
        } else if cmd == "code" {
            let paths = [
                "/Applications/Visual Studio Code.app".to_string(),
                format!("{}/Applications/Visual Studio Code.app", home),
            ];
            for p in &paths {
                if std::path::Path::new(p).exists() {
                    return true;
                }
            }
            if let Ok(output) = std::process::Command::new("mdfind")
                .arg("kMDItemCFBundleIdentifier == 'com.microsoft.VSCode'")
                .output()
            {
                if !output.stdout.is_empty()
                    && output.stdout.iter().any(|&b| !b.is_ascii_whitespace())
                {
                    return true;
                }
            }
        }
    }

    false
}
