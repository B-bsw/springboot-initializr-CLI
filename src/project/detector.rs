use std::path::Path;
use std::fs;
use super::{BuildFile, GradleBuildFile, MavenBuildFile};

pub struct ProjectDetector;

impl ProjectDetector {
    pub fn detect() -> Result<Box<dyn BuildFile>, String> {
        let paths = [
            ("build.gradle.kts", false),
            ("build.gradle", false),
            ("pom.xml", true),
        ];

        for (path, is_maven) in paths.iter() {
            if Path::new(path).exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if *is_maven {
                        return Ok(Box::new(MavenBuildFile::new(content)));
                    } else {
                        return Ok(Box::new(GradleBuildFile::new(path.to_string(), content)));
                    }
                }
            }
        }

        Err("Could not find build.gradle.kts, build.gradle, or pom.xml in the current directory. Are you in a Spring Boot project?".to_string())
    }
}
