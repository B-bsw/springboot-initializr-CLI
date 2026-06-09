use std::path::Path;
use std::fs;

#[derive(Debug, PartialEq, Clone)]
pub enum BuildTool {
    GradleKotlin,
    GradleGroovy,
    Maven,
}

impl BuildTool {
    pub fn file_name(&self) -> &'static str {
        match self {
            BuildTool::GradleKotlin => "build.gradle.kts",
            BuildTool::GradleGroovy => "build.gradle",
            BuildTool::Maven => "pom.xml",
        }
    }

    pub fn is_maven(&self) -> bool {
        matches!(self, BuildTool::Maven)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BuildTool::GradleKotlin => "Gradle Kotlin DSL",
            BuildTool::GradleGroovy => "Gradle Groovy DSL",
            BuildTool::Maven => "Maven",
        }
    }
}

pub fn detect_build_tool() -> Result<(BuildTool, String), String> {
    // Priority: build.gradle.kts -> build.gradle -> pom.xml
    let paths = [
        (BuildTool::GradleKotlin, "build.gradle.kts"),
        (BuildTool::GradleGroovy, "build.gradle"),
        (BuildTool::Maven, "pom.xml"),
    ];

    for (tool, path) in paths.iter() {
        if Path::new(path).exists() {
            if let Ok(content) = fs::read_to_string(path) {
                return Ok((tool.clone(), content));
            }
        }
    }

    Err("Could not find build.gradle.kts, build.gradle, or pom.xml in the current directory. Are you in a Spring Boot project?".to_string())
}
