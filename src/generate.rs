use crate::metadata::{self, Metadata};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;
use zip::ZipArchive;

#[derive(Args)]
pub struct NewArgs {
    /// Project type (e.g. maven-project, gradle-project)
    #[arg(short = 't', long, alias = "type")]
    pub project: Option<String>,

    /// Programming language (java, kotlin, groovy)
    #[arg(short, long)]
    pub language: Option<String>,

    /// Spring Boot version
    #[arg(short, long)]
    pub boot: Option<String>,

    /// Project name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Group ID (e.g. com.example)
    #[arg(short, long)]
    pub group: Option<String>,

    /// Artifact ID
    #[arg(short, long)]
    pub artifact: Option<String>,

    /// Package name (e.g. com.example.demo)
    #[arg(long)]
    pub package_name: Option<String>,

    /// Packaging (jar, war)
    #[arg(short, long)]
    pub packaging: Option<String>,

    /// Java version (e.g. 21, 17)
    #[arg(short, long)]
    pub java: Option<String>,

    /// Configuration file format (properties, yaml)
    #[arg(short = 'f', long, alias = "format")]
    pub config_format: Option<String>,

    /// Comma-separated dependency IDs (e.g. web,data-jpa,validation)
    #[arg(short, long, value_delimiter = ',')]
    pub deps: Vec<String>,

    /// Output directory (defaults to current directory)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Initialize a git repository after generation
    #[arg(long)]
    pub git: bool,

    /// Generate Dockerfile and docker-compose.yml
    #[arg(long)]
    pub docker: bool,

    /// Apply a project template (e.g. clean-architecture)
    #[arg(long)]
    pub template: Option<String>,

    /// Open in IDE after generation (e.g. code, idea)
    #[arg(long)]
    pub ide: Option<String>,

    /// Extract to output directory without the wrapper folder
    #[arg(long)]
    pub flat: bool,
}

pub async fn run(args: NewArgs) -> Result<(), String> {
    // Fetch metadata
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message("Fetching metadata from start.spring.io...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let meta = metadata::fetch_metadata().await?;
    spinner.finish_and_clear();

    // Resolve values with defaults
    let project = resolve_or_default(&args.project, &meta.defaults.project);
    let language = resolve_or_default(&args.language, &meta.defaults.language);
    let boot = resolve_or_default(&args.boot, &meta.defaults.boot);
    let name = resolve_or_default(&args.name, &meta.defaults.name);
    let group = resolve_or_default(&args.group, &meta.defaults.group);
    let artifact = resolve_or_default(&args.artifact, &meta.defaults.artifact);
    let packaging = resolve_or_default(&args.packaging, &meta.defaults.packaging);
    let java = resolve_or_default(&args.java, &meta.defaults.java);
    let config_format = resolve_or_default(&args.config_format, &meta.defaults.config_format);
    let safe_artifact = artifact.replace("-", "").replace("_", "");
    let package_name = args
        .package_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("{group}.{safe_artifact}"));

    // Validate
    validate_option(&project, &meta.projects, "project type")?;
    validate_option(&language, &meta.languages, "language")?;
    validate_option(&boot, &meta.boot_versions, "boot version")?;
    validate_option(&packaging, &meta.packagings, "packaging")?;
    validate_option(&java, &meta.java_versions, "java version")?;
    validate_option(&config_format, &meta.config_formats, "config format")?;
    validate_deps(&args.deps, &meta, &boot)?;

    let output_dir = args.output.unwrap_or_else(|| PathBuf::from("."));

    // Print summary
    println!();
    println!("\x1b[1;32m🍃 Spring Initializr\x1b[0m");
    println!();
    println!("  \x1b[1mProject\x1b[0m      {project}");
    println!("  \x1b[1mLanguage\x1b[0m     {language}");
    println!("  \x1b[1mBoot\x1b[0m         {boot}");
    println!("  \x1b[1mName\x1b[0m         {name}");
    println!("  \x1b[1mGroup\x1b[0m        {group}");
    println!("  \x1b[1mArtifact\x1b[0m     {artifact}");
    println!("  \x1b[1mPackage\x1b[0m      {package_name}");
    println!("  \x1b[1mPackaging\x1b[0m    {packaging}");
    println!("  \x1b[1mJava\x1b[0m         {java}");
    println!("  \x1b[1mConfig\x1b[0m       {config_format}");
    if !args.deps.is_empty() {
        println!(
            "  \x1b[1mDeps\x1b[0m         {}",
            args.deps.join(", ")
        );
    }
    println!(
        "  \x1b[1mOutput\x1b[0m       {}",
        output_dir.canonicalize().unwrap_or(output_dir.clone()).display()
    );
    println!();

    // Download
    let zip_bytes = download_zip(
        &project,
        &language,
        &boot,
        &name,
        &group,
        &artifact,
        &package_name,
        &packaging,
        &java,
        &config_format,
        &args.deps,
    )
    .await?;

    // Extract
    let target = extract_zip(&zip_bytes, &output_dir, &name, args.flat)?;

    println!(
        "\x1b[1;32m✓ Project generated at\x1b[0m {}",
        target.display()
    );

    if let Some(tpl) = &args.template {
        if let Err(e) = crate::template::apply_template(tpl, &target, &package_name) {
            println!("  \x1b[33m! {}\x1b[0m", e);
        }
    }

    if args.docker {
        if let Err(e) = crate::docker::add_docker_support(&target, &artifact) {
            println!("  \x1b[33m! Failed to add Docker support: {}\x1b[0m", e);
        }
    }

    if args.git {
        crate::git::init_git(&target);
    }

    // Open IDE
    if let Some(ide) = &args.ide {
        println!("\x1b[2mOpening in {ide}...\x1b[0m");
        
        let spawn_result = std::process::Command::new(ide)
            .arg(&target)
            .spawn();
            
        let mut success = spawn_result.is_ok();
        
        if !success && ide == "idea" {
            #[cfg(target_os = "macos")]
            {
                if let Ok(status) = std::process::Command::new("open")
                    .arg("-a")
                    .arg("IntelliJ IDEA")
                    .arg(&target)
                    .status()
                {
                    success = status.success();
                    if !success {
                        if let Ok(status_ce) = std::process::Command::new("open")
                            .arg("-a")
                            .arg("IntelliJ IDEA CE")
                            .arg(&target)
                            .status()
                        {
                            success = status_ce.success();
                        }
                    }
                }
            }
        }
        
        if !success {
            // Fallback to opening the folder if IDE fails
            let _ = open::that_in_background(target.to_str().unwrap_or("."));
        }
    }

    Ok(())
}

fn resolve_or_default(value: &Option<String>, default: &str) -> String {
    value
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| default.to_string())
}

fn validate_option(
    value: &str,
    options: &[metadata::Option_],
    label: &str,
) -> Result<(), String> {
    if options.iter().any(|o| o.key == value) {
        Ok(())
    } else {
        let available: Vec<&str> = options.iter().map(|o| o.key.as_str()).collect();
        Err(format!(
            "Invalid {label}: '{value}'. Available: {}",
            available.join(", ")
        ))
    }
}

fn validate_deps(deps: &[String], meta: &Metadata, boot_version: &str) -> Result<(), String> {
    use crate::version::{is_boot_version_in_range, format_version_range};
    for dep in deps {
        if let Some(found_dep) = meta.all_deps.iter().find(|d| d.key == *dep) {
            // Found it, now check version range
            if !is_boot_version_in_range(boot_version, found_dep.version_range.as_deref()) {
                let raw_range = found_dep.version_range.as_deref().unwrap_or("");
                let formatted = format_version_range(raw_range);
                return Err(format!(
                    "Dependency '{}' is not compatible with Spring Boot {}. Required: {}",
                    dep, boot_version, formatted
                ));
            }
        } else {
            // Not found at all, fuzzy suggestion
            let suggestions: Vec<&str> = meta
                .all_deps
                .iter()
                .filter(|d| {
                    (d.key.contains(dep.as_str())
                        || d.text.to_lowercase().contains(&dep.to_lowercase()))
                        && is_boot_version_in_range(boot_version, d.version_range.as_deref())
                })
                .take(5)
                .map(|d| d.key.as_str())
                .collect();
            let hint = if suggestions.is_empty() {
                String::new()
            } else {
                format!(" Did you mean: {}?", suggestions.join(", "))
            };
            return Err(format!("Unknown dependency: '{dep}'.{hint}"));
        }
    }
    Ok(())
}

pub async fn download_zip(
    project: &str,
    language: &str,
    boot: &str,
    base_dir: &str,
    group: &str,
    artifact: &str,
    package_name: &str,
    packaging: &str,
    java: &str,
    config_format: &str,
    deps: &[String],
) -> Result<Vec<u8>, String> {
    let mut params = vec![
        ("type", project),
        ("language", language),
        ("bootVersion", boot),
        ("baseDir", base_dir),
        ("groupId", group),
        ("artifactId", artifact),
        ("packageName", package_name),
        ("packaging", packaging),
        ("javaVersion", java),
        ("configurationFileFormat", config_format),
    ];
    let deps_str = deps.join(",");
    if !deps.is_empty() {
        params.push(("dependencies", deps_str.as_str()));
    }

    let url = format!(
        "https://start.spring.io/starter.zip?{}",
        params
            .iter()
            .map(|(k, v)| format!(
                "{k}={}",
                urlencoding(v)
            ))
            .collect::<Vec<_>>()
            .join("&")
    );

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Downloading project zip...");
    pb.enable_steady_tick(Duration::from_millis(80));

    let response = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/zip")
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !response.status().is_success() {
        pb.finish_and_clear();
        return Err(format!(
            "Generate failed (HTTP {})",
            response.status().as_u16()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    pb.finish_and_clear();
    println!(
        "\x1b[2m  Downloaded {} bytes\x1b[0m",
        bytes.len()
    );

    Ok(bytes.to_vec())
}

fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for c in s.bytes() {
        match c {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(c as char);
            }
            _ => {
                result.push_str(&format!("%{c:02X}"));
            }
        }
    }
    result
}

pub fn extract_zip(
    zip_bytes: &[u8],
    output_dir: &Path,
    project_name: &str,
    flat: bool,
) -> Result<PathBuf, String> {
    let cursor = Cursor::new(zip_bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to open zip: {e}"))?;

    // Determine target directory
    let folder_name = normalize_folder_name(project_name);
    let target_dir = if flat {
        output_dir.to_path_buf()
    } else {
        output_dir.join(&folder_name)
    };

    if target_dir.exists() && !flat {
        return Err(format!(
            "Target folder already exists: {}",
            target_dir.display()
        ));
    }

    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create directory: {e}"))?;

    // Find the common prefix (the baseDir folder inside the zip)
    let prefix = find_zip_prefix(&mut archive);

    let pb = ProgressBar::new(archive.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  \x1b[32m{bar:30}\x1b[0m {pos}/{len} files")
            .unwrap()
            .progress_chars("━━╸"),
    );

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Zip read error: {e}"))?;

        let name = file.name().to_string();

        // Strip the prefix
        let relative = if let Some(ref p) = prefix {
            match name.strip_prefix(p.as_str()) {
                Some(rest) => rest.to_string(),
                None => name.clone(),
            }
        } else {
            name.clone()
        };

        if relative.is_empty() || relative == "/" {
            pb.inc(1);
            continue;
        }

        let out_path = target_dir.join(&relative);

        if name.ends_with('/') {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create dir {}: {e}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create dir: {e}"))?;
            }
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| format!("Failed to read file from zip: {e}"))?;
            fs::write(&out_path, &buf)
                .map_err(|e| format!("Failed to write {}: {e}", out_path.display()))?;
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    // List extracted structure
    println!("\x1b[2m  Extracted files:\x1b[0m");
    list_tree(&target_dir, &target_dir, 0, 12);

    Ok(target_dir)
}

fn normalize_folder_name(name: &str) -> String {
    let normalized: String = name
        .trim()
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect();
    if normalized.is_empty() {
        "project".to_string()
    } else {
        normalized
    }
}

fn find_zip_prefix(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Option<String> {
    if archive.len() == 0 {
        return None;
    }
    // Get the first entry's name without holding the borrow
    let first_name = {
        let first = archive.by_index(0).ok()?;
        first.name().to_string()
    };

    let prefix = if let Some(slash_pos) = first_name.find('/') {
        first_name[..=slash_pos].to_string()
    } else {
        return None;
    };

    for i in 1..archive.len() {
        if let Ok(f) = archive.by_index(i) {
            if !f.name().starts_with(&prefix) {
                return None;
            }
        }
    }

    Some(prefix)
}

fn list_tree(root: &Path, current: &Path, depth: usize, max_files: usize) {
    let mut entries: Vec<_> = match fs::read_dir(current) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };
    entries.sort_by_key(|e| e.file_name());

    let mut shown = 0;
    for entry in &entries {
        if shown >= max_files && depth == 0 {
            let remaining = entries.len() - shown;
            if remaining > 0 {
                println!(
                    "  \x1b[2m{}  ... and {remaining} more\x1b[0m",
                    "  ".repeat(depth)
                );
            }
            break;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = path.is_dir();

        if is_dir {
            println!(
                "  \x1b[34m{}📁 {}/\x1b[0m",
                "  ".repeat(depth),
                name
            );
            if depth < 2 {
                list_tree(root, &path, depth + 1, 8);
            }
        } else {
            println!(
                "  \x1b[2m{}   {}\x1b[0m",
                "  ".repeat(depth),
                name
            );
        }
        shown += 1;
    }
}
