use reqwest;
use serde::Deserialize;
use std::fmt;

// ── Raw API types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RawMetadata {
    #[serde(rename = "type")]
    pub project_type: Option<TypeBlock>,
    pub language: Option<SimpleBlock>,
    pub boot_version: Option<SimpleBlock>,
    pub packaging: Option<SimpleBlock>,
    pub java_version: Option<SimpleBlock>,
    pub configuration_file_format: Option<SimpleBlock>,
    pub group_id: Option<DefaultOnly>,
    pub artifact_id: Option<DefaultOnly>,
    pub name: Option<DefaultOnly>,
    pub package_name: Option<DefaultOnly>,
    pub dependencies: Option<DependenciesBlock>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TypeBlock {
    pub default: Option<String>,
    pub values: Option<Vec<TypeValue>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TypeValue {
    pub id: String,
    pub name: String,
    pub action: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SimpleBlock {
    pub default: Option<String>,
    pub values: Option<Vec<SimpleValue>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SimpleValue {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct DefaultOnly {
    pub default: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DependenciesBlock {
    pub values: Option<Vec<DependencyGroup>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DependencyGroup {
    pub name: Option<String>,
    pub values: Option<Vec<DependencyValue>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DependencyValue {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub version_range: Option<String>,
}

// ── View model ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Option_ {
    pub key: String,
    pub text: String,
}

impl fmt::Display for Option_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DepOption {
    pub key: String,
    pub text: String,
    pub description: String,
    pub group: String,
    pub version_range: Option<String>,
}

impl fmt::Display for DepOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.description.is_empty() {
            write!(f, "{}", self.text)
        } else {
            write!(f, "{} — {}", self.text, self.description)
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepGroup {
    pub name: String,
    pub deps: Vec<DepOption>,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub projects: Vec<Option_>,
    pub languages: Vec<Option_>,
    pub boot_versions: Vec<Option_>,
    pub packagings: Vec<Option_>,
    pub java_versions: Vec<Option_>,
    pub config_formats: Vec<Option_>,
    pub dependency_groups: Vec<DepGroup>,
    pub all_deps: Vec<DepOption>,
    pub defaults: Defaults,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Defaults {
    pub project: String,
    pub language: String,
    pub boot: String,
    pub name: String,
    pub group: String,
    pub artifact: String,
    pub package_name: String,
    pub packaging: String,
    pub java: String,
    pub config_format: String,
}

// ── API URL ────────────────────────────────────────────────────────────

const API_URL: &str = "https://api-springboot-initializr.vercel.app/api";

// ── Fetch & map ────────────────────────────────────────────────────────

pub async fn fetch_metadata() -> Result<Metadata, String> {
    let raw: RawMetadata = reqwest::get(API_URL)
        .await
        .map_err(|e| format!("Failed to fetch metadata: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse metadata: {e}"))?;
    Ok(map_metadata(raw))
}

fn map_metadata(raw: RawMetadata) -> Metadata {
    let projects: Vec<Option_> = raw
        .project_type
        .as_ref()
        .and_then(|t| t.values.as_ref())
        .map(|vs| {
            vs.iter()
                .filter(|v| v.action.as_deref() == Some("/starter.zip"))
                .map(|v| Option_ {
                    key: v.id.clone(),
                    text: v.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let languages = map_simple(&raw.language);
    let boot_versions = map_simple(&raw.boot_version);
    let packagings = map_simple(&raw.packaging);
    let java_versions = map_simple(&raw.java_version);
    let config_formats = map_simple(&raw.configuration_file_format);

    let mut dependency_groups: Vec<DepGroup> = Vec::new();
    let mut all_deps: Vec<DepOption> = Vec::new();

    if let Some(deps_block) = &raw.dependencies {
        if let Some(groups) = &deps_block.values {
            for group in groups {
                let group_name = group
                    .name
                    .as_deref()
                    .unwrap_or("Dependencies")
                    .trim()
                    .to_string();
                let mut deps = Vec::new();
                if let Some(values) = &group.values {
                    for v in values {
                        if let Some(id) = v.id.as_ref().filter(|s| !s.trim().is_empty()) {
                            let dep = DepOption {
                                key: id.trim().to_string(),
                                text: readable_name(v.name.as_deref(), id),
                                description: v
                                    .description
                                    .as_deref()
                                    .unwrap_or("")
                                    .trim()
                                    .to_string(),
                                group: group_name.clone(),
                                version_range: v
                                    .version_range
                                    .as_ref()
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty()),
                            };
                            deps.push(dep.clone());
                            all_deps.push(dep);
                        }
                    }
                }
                if !deps.is_empty() {
                    dependency_groups.push(DepGroup {
                        name: group_name,
                        deps,
                    });
                }
            }
        }
    }

    let defaults = Defaults {
        project: raw
            .project_type
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "maven-project".into()),
        language: raw
            .language
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "java".into()),
        boot: raw
            .boot_version
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "3.4.4".into()),
        name: raw
            .name
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "demo".into()),
        group: raw
            .group_id
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "com.example".into()),
        artifact: raw
            .artifact_id
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "demo".into()),
        package_name: raw
            .package_name
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "com.example.demo".into()),
        packaging: raw
            .packaging
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "jar".into()),
        java: raw
            .java_version
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "21".into()),
        config_format: raw
            .configuration_file_format
            .as_ref()
            .and_then(|t| t.default.clone())
            .unwrap_or_else(|| "properties".into()),
    };

    // Fallback if empty
    if projects.is_empty() || languages.is_empty() || boot_versions.is_empty() {
        return fallback();
    }

    Metadata {
        projects,
        languages,
        boot_versions,
        packagings,
        java_versions,
        config_formats,
        dependency_groups,
        all_deps,
        defaults,
    }
}

fn map_simple(block: &Option<SimpleBlock>) -> Vec<Option_> {
    block
        .as_ref()
        .and_then(|b| b.values.as_ref())
        .map(|vs| {
            vs.iter()
                .map(|v| Option_ {
                    key: v.id.clone(),
                    text: v.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn readable_name(name: Option<&str>, id: &str) -> String {
    if let Some(n) = name {
        let trimmed = n.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    id.split(&['-', '_', '.'][..])
        .filter(|s| !s.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    format!("{upper}{}", chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn fallback() -> Metadata {
    Metadata {
        projects: vec![
            Option_ { key: "maven-project".into(), text: "Maven".into() },
            Option_ { key: "gradle-project".into(), text: "Gradle".into() },
        ],
        languages: vec![
            Option_ { key: "java".into(), text: "Java".into() },
            Option_ { key: "kotlin".into(), text: "Kotlin".into() },
            Option_ { key: "groovy".into(), text: "Groovy".into() },
        ],
        boot_versions: vec![
            Option_ { key: "3.4.4".into(), text: "3.4.4".into() },
            Option_ { key: "3.3.10".into(), text: "3.3.10".into() },
        ],
        packagings: vec![
            Option_ { key: "jar".into(), text: "Jar".into() },
            Option_ { key: "war".into(), text: "War".into() },
        ],
        java_versions: vec![
            Option_ { key: "21".into(), text: "21".into() },
            Option_ { key: "17".into(), text: "17".into() },
        ],
        config_formats: vec![
            Option_ { key: "properties".into(), text: "Properties".into() },
            Option_ { key: "yaml".into(), text: "YAML".into() },
        ],
        dependency_groups: vec![DepGroup {
            name: "Core".into(),
            deps: vec![
                DepOption {
                    key: "web".into(),
                    text: "Spring Web".into(),
                    description: "Build web, including RESTful applications".into(),
                    group: "Core".into(),
                    version_range: None,
                },
                DepOption {
                    key: "data-jpa".into(),
                    text: "Spring Data JPA".into(),
                    description: "Persist data in SQL stores with Java Persistence API".into(),
                    group: "Core".into(),
                    version_range: None,
                },
            ],
        }],
        all_deps: vec![
            DepOption {
                key: "web".into(),
                text: "Spring Web".into(),
                description: "Build web, including RESTful applications".into(),
                group: "Core".into(),
                version_range: None,
            },
            DepOption {
                key: "data-jpa".into(),
                text: "Spring Data JPA".into(),
                description: "Persist data in SQL stores with Java Persistence API".into(),
                group: "Core".into(),
                version_range: None,
            },
        ],
        defaults: Defaults {
            project: "maven-project".into(),
            language: "java".into(),
            boot: "3.4.4".into(),
            name: "demo".into(),
            group: "com.example".into(),
            artifact: "demo".into(),
            package_name: "com.example.demo".into(),
            packaging: "jar".into(),
            java: "21".into(),
            config_format: "properties".into(),
        },
    }
}

// ── List subcommand ────────────────────────────────────────────────────

use clap::Args;

#[derive(Args)]
pub struct ListArgs {}

pub async fn list() -> Result<(), String> {
    let meta = fetch_metadata().await?;

    println!("\n  \x1b[1;32m🍃 Dependencies ({} total)\x1b[0m", meta.all_deps.len());
    for group in &meta.dependency_groups {
        println!("\n  \x1b[1;33m── {} ──\x1b[0m", group.name);
        for dep in &group.deps {
            println!("    \x1b[2m▪\x1b[0m \x1b[36m{:<28}\x1b[0m {}", dep.key, dep.text);
        }
    }

    Ok(())
}
