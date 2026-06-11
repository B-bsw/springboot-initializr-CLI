use std::path::Path;
use std::fs;

pub trait BuildFile {
    fn file_name(&self) -> &str;
    fn is_maven(&self) -> bool;
    fn content(&self) -> &str;
    fn set_content(&mut self, content: String);
    fn build_tool_name(&self) -> &str;

    fn extract_artifact_ids(&self) -> Vec<String>;
    fn extract_boot_version(&self) -> Option<String>;

    fn add_dependencies(&mut self, lines: &str);
    fn remove_dependency_line(&mut self, artifact_id: &str) -> bool;

    fn add_bom(&mut self, downloaded_text: &str);
    fn remove_bom(&mut self, downloaded_text: &str) -> bool;
    fn add_properties(&mut self, downloaded_text: &str);
    fn remove_properties(&mut self, downloaded_text: &str) -> bool;
    fn add_plugins(&mut self, downloaded_text: &str);

    fn cleanup_empty_blocks(&mut self);

    fn save(&self) -> Result<(), String> {
        fs::write(self.file_name(), self.content())
            .map_err(|e| format!("Failed to update build file: {}", e))
    }
}

// ── Gradle ─────────────────────────────────────────

pub struct GradleBuildFile {
    file_name: String,
    content: String,
}

impl GradleBuildFile {
    pub fn new(file_name: String, content: String) -> Self {
        Self { file_name, content }
    }
}

impl BuildFile for GradleBuildFile {
    fn file_name(&self) -> &str { &self.file_name }
    fn is_maven(&self) -> bool { false }
    fn content(&self) -> &str { &self.content }
    fn set_content(&mut self, content: String) { self.content = content; }
    fn build_tool_name(&self) -> &str {
        if self.file_name.ends_with(".kts") { "Gradle Kotlin DSL" } else { "Gradle Groovy DSL" }
    }

    fn extract_artifact_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for line in self.content.lines() {
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
        ids
    }

    fn extract_boot_version(&self) -> Option<String> {
        if let Some(idx) = self.content.find("org.springframework.boot") {
            let rest = &self.content[idx..];
            if let Some(version_idx) = rest.find("version") {
                let ver_str = &rest[version_idx + 7..];
                let mut in_quote = false;
                let mut quote_char = ' ';
                let mut extracted = String::new();
                for c in ver_str.chars() {
                    if in_quote {
                        if c == quote_char { break; }
                        extracted.push(c);
                    } else if c == '\'' || c == '"' {
                        in_quote = true;
                        quote_char = c;
                    } else if c == '\n' {
                        break;
                    }
                }
                if extracted.starts_with('2') || extracted.starts_with('3') || extracted.starts_with('4') {
                    return Some(extracted.trim().to_string());
                }
            }
        }
        None
    }

    fn add_dependencies(&mut self, lines: &str) {
        if let Some(insert_pos) = self.content.rfind("dependencies {") {
            if let Some(brace_end) = self.content[insert_pos..].find('}') {
                self.content.insert_str(insert_pos + brace_end, lines);
            }
        }
    }

    fn remove_dependency_line(&mut self, artifact_id: &str) -> bool {
        if let Some(idx) = self.content.find(artifact_id) {
            if let Some(start) = self.content[..idx].rfind('\n') {
                if let Some(end) = self.content[idx..].find('\n') {
                    self.content.replace_range(start..idx + end, "");
                    return true;
                }
            }
        }
        false
    }

    fn add_bom(&mut self, downloaded_text: &str) {
        // Extract ext block
        if let Some(ext_start) = downloaded_text.find("ext {") {
            if let Some(ext_end) = downloaded_text[ext_start..].find("}") {
                let ext_block = &downloaded_text[ext_start..ext_start + ext_end];
                let mut to_add = String::new();
                for line in ext_block.lines() {
                    if line.trim().starts_with("set(") {
                        if !self.content.contains(line.trim()) {
                            to_add.push_str("\t");
                            to_add.push_str(line.trim());
                            to_add.push('\n');
                        }
                    }
                }
                if !to_add.is_empty() {
                    if let Some(target_ext) = self.content.find("ext {") {
                        if let Some(brace_end) = self.content[target_ext..].find('}') {
                            self.content.insert_str(target_ext + brace_end, &format!("{}", to_add));
                        }
                    } else {
                        if let Some(dep_start) = self.content.find("dependencies {") {
                            self.content.insert_str(dep_start, &format!("ext {{\n{}}}\n\n", to_add));
                        } else {
                            self.content.push_str(&format!("\next {{\n{}}}\n", to_add));
                        }
                    }
                }
            }
        }
        // Extract mavenBom lines
        let mut bom_lines = String::new();
        for line in downloaded_text.lines() {
            if line.trim().starts_with("mavenBom ") {
                if !self.content.contains(line.trim()) {
                    bom_lines.push_str("\t\t");
                    bom_lines.push_str(line.trim());
                    bom_lines.push('\n');
                }
            }
        }
        if !bom_lines.is_empty() {
            if let Some(target_dm) = self.content.find("dependencyManagement {") {
                if let Some(imports_start) = self.content[target_dm..].find("imports {") {
                    if let Some(imports_end) = self.content[target_dm + imports_start..].find('}') {
                        self.content.insert_str(target_dm + imports_start + imports_end, &format!("{}", bom_lines));
                    }
                }
            } else {
                self.content.push_str(&format!("\ndependencyManagement {{\n\timports {{\n{}\t}}\n}}\n", bom_lines));
            }
        }
    }

    fn remove_bom(&mut self, downloaded_text: &str) -> bool {
        let mut changed = false;
        // Remove ext lines
        if let Some(ext_start) = downloaded_text.find("ext {") {
            if let Some(ext_end) = downloaded_text[ext_start..].find("}") {
                let ext_block = &downloaded_text[ext_start..ext_start + ext_end];
                for line in ext_block.lines() {
                    let line_t = line.trim();
                    if line_t.starts_with("set(") {
                        if let Some(idx) = self.content.find(line_t) {
                            if let Some(start) = self.content[..idx].rfind('\n') {
                                if let Some(end) = self.content[idx..].find('\n') {
                                    self.content.replace_range(start..idx + end, "");
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        // Remove mavenBom lines
        for line in downloaded_text.lines() {
            let line_t = line.trim();
            if line_t.starts_with("mavenBom ") {
                if let Some(idx) = self.content.find(line_t) {
                    if let Some(start) = self.content[..idx].rfind('\n') {
                        if let Some(end) = self.content[idx..].find('\n') {
                            self.content.replace_range(start..idx + end, "");
                            changed = true;
                        }
                    }
                }
            }
        }
        changed
    }

    fn add_properties(&mut self, _downloaded_text: &str) {
        // Gradle uses ext block, handled in add_bom
    }

    fn remove_properties(&mut self, _downloaded_text: &str) -> bool {
        // Gradle uses ext block, handled in remove_bom
        false
    }

    fn add_plugins(&mut self, downloaded_text: &str) {
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
                    if let Some(target_plugins_start) = self.content.find("plugins {") {
                        if let Some(target_plugins_end) = self.content[target_plugins_start..].find("}") {
                            self.content.insert_str(target_plugins_start + target_plugins_end, &format!("{}\n", plugins_to_add));
                        }
                    }
                }
            }
        }
        // generateJava block
        if let Some(gen_start) = downloaded_text.find("generateJava {") {
            if let Some(gen_end) = downloaded_text[gen_start..].find("}") {
                let gen_block = &downloaded_text[gen_start..gen_start + gen_end + 1];
                println!("  {} {} (Config Block)", console::style("+").green(), console::style("generateJava").bold());
                if !self.content.contains("generateJava {") {
                    self.content.push_str("\n");
                    self.content.push_str(gen_block);
                    self.content.push_str("\n");
                }
            }
        }
    }

    fn cleanup_empty_blocks(&mut self) {
        self.content = self.content.replace("ext {\n}\n", "");
        self.content = self.content.replace("dependencyManagement {\n\timports {\n\t}\n}\n", "");
    }
}

// ── Maven ──────────────────────────────────────────

pub struct MavenBuildFile {
    content: String,
}

impl MavenBuildFile {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

impl BuildFile for MavenBuildFile {
    fn file_name(&self) -> &str { "pom.xml" }
    fn is_maven(&self) -> bool { true }
    fn content(&self) -> &str { &self.content }
    fn set_content(&mut self, content: String) { self.content = content; }
    fn build_tool_name(&self) -> &str { "Maven" }

    fn extract_artifact_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        let mut current_idx = 0;
        while let Some(start) = self.content[current_idx..].find("<artifactId>") {
            let actual_start = current_idx + start + 12;
            if let Some(end) = self.content[actual_start..].find("</artifactId>") {
                let artifact_id = &self.content[actual_start..actual_start + end];
                ids.push(artifact_id.trim().to_string());
                current_idx = actual_start + end + 13;
            } else {
                break;
            }
        }
        ids
    }

    fn extract_boot_version(&self) -> Option<String> {
        if let Some(idx) = self.content.find("<artifactId>spring-boot-starter-parent</artifactId>") {
            let rest = &self.content[idx..];
            if let Some(v_start) = rest.find("<version>") {
                if let Some(v_end) = rest[v_start..].find("</version>") {
                    let ver = &rest[v_start + 9..v_start + v_end];
                    if ver.starts_with('2') || ver.starts_with('3') || ver.starts_with('4') {
                        return Some(ver.trim().to_string());
                    }
                }
            }
        }
        if let Some(idx) = self.content.find("<version>") {
            if let Some(end) = self.content[idx..].find("</version>") {
                let ver = &self.content[idx + 9..idx + end];
                if ver.starts_with('2') || ver.starts_with('3') || ver.starts_with('4') {
                    return Some(ver.trim().to_string());
                }
            }
        }
        None
    }

    fn add_dependencies(&mut self, lines: &str) {
        if let Some(insert_pos) = self.content.rfind("</dependencies>") {
            self.content.insert_str(insert_pos, &format!("{}\n", lines));
        }
    }

    fn remove_dependency_line(&mut self, artifact_id: &str) -> bool {
        let search = format!("<artifactId>{}</artifactId>", artifact_id);
        if let Some(idx) = self.content.find(&search) {
            if let Some(start) = self.content[..idx].rfind("<dependency>") {
                if let Some(end) = self.content[idx..].find("</dependency>") {
                    self.content.replace_range(start..idx + end + 13, "");
                    return true;
                }
            }
        }
        false
    }

    fn add_bom(&mut self, downloaded_text: &str) {
        if let Some(dm_start) = downloaded_text.find("<dependencyManagement>") {
            if let Some(dm_end) = downloaded_text[dm_start..].find("</dependencyManagement>") {
                let dm_block = &downloaded_text[dm_start..dm_start + dm_end + 23];
                if let Some(deps_start) = dm_block.find("<dependencies>") {
                    if let Some(deps_end) = dm_block[deps_start..].find("</dependencies>") {
                        let inner_deps = &dm_block[deps_start+14..deps_start+deps_end];
                        let mut current_idx = 0;
                        let mut bom_to_add = String::new();
                        while let Some(d_start) = inner_deps[current_idx..].find("<dependency>") {
                            if let Some(d_end) = inner_deps[current_idx + d_start..].find("</dependency>") {
                                let block = &inner_deps[current_idx + d_start .. current_idx + d_start + d_end + 13];
                                let mut found = false;
                                if let Some(a_start) = block.find("<artifactId>") {
                                    if let Some(a_end) = block[a_start..].find("</artifactId>") {
                                        let a_id = &block[a_start + 12..a_start + a_end];
                                        if self.content.contains(a_id) {
                                            found = true;
                                        }
                                    }
                                }
                                if !found {
                                    bom_to_add.push_str("            ");
                                    bom_to_add.push_str(block.trim().replace("\n", "\n            ").as_str());
                                    bom_to_add.push('\n');
                                }
                                current_idx = current_idx + d_start + d_end + 13;
                            } else {
                                break;
                            }
                        }
                        if !bom_to_add.is_empty() {
                            if let Some(target_dm) = self.content.find("<dependencyManagement>") {
                                if let Some(target_deps) = self.content[target_dm..].find("</dependencies>") {
                                    self.content.insert_str(target_dm + target_deps, &format!("{}", bom_to_add));
                                }
                            } else {
                                if let Some(build_start) = self.content.find("<build>") {
                                    self.content.insert_str(build_start, &format!("    <dependencyManagement>\n        <dependencies>\n{}        </dependencies>\n    </dependencyManagement>\n\n", bom_to_add));
                                } else if let Some(dep_end) = self.content.find("</dependencies>") {
                                    self.content.insert_str(dep_end + 15, &format!("\n    <dependencyManagement>\n        <dependencies>\n{}        </dependencies>\n    </dependencyManagement>\n", bom_to_add));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn remove_bom(&mut self, downloaded_text: &str) -> bool {
        let mut changed = false;
        // Remove BOM artifacts from dependencyManagement
        let mut current_idx = 0;
        while let Some(d_start) = downloaded_text[current_idx..].find("<dependency>") {
            if let Some(d_end) = downloaded_text[current_idx + d_start..].find("</dependency>") {
                let block = &downloaded_text[current_idx + d_start .. current_idx + d_start + d_end + 13];
                if let Some(a_start) = block.find("<artifactId>") {
                    if let Some(a_end) = block[a_start..].find("</artifactId>") {
                        let a_id = &block[a_start + 12..a_start + a_end];
                        if a_id != "spring-boot-starter" && a_id != "spring-boot-starter-test" && a_id != "junit-platform-launcher" {
                            // Try to remove from dependencyManagement section
                            let search = format!("<artifactId>{}</artifactId>", a_id);
                            if let Some(dm_start) = self.content.find("<dependencyManagement>") {
                                if let Some(idx) = self.content[dm_start..].find(&search) {
                                    let actual_idx = dm_start + idx;
                                    if let Some(start) = self.content[..actual_idx].rfind("<dependency>") {
                                        if let Some(end) = self.content[actual_idx..].find("</dependency>") {
                                            self.content.replace_range(start..actual_idx + end + 13, "");
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                current_idx = current_idx + d_start + d_end + 13;
            } else {
                break;
            }
        }
        changed
    }

    fn add_properties(&mut self, downloaded_text: &str) {
        if let Some(prop_start) = downloaded_text.find("<properties>") {
            if let Some(prop_end) = downloaded_text[prop_start..].find("</properties>") {
                let prop_block = &downloaded_text[prop_start+12..prop_start + prop_end];
                let mut to_add = String::new();
                for line in prop_block.lines() {
                    let line_t = line.trim();
                    if !line_t.is_empty() && !line_t.contains("<java.version>") {
                        if !self.content.contains(line_t) {
                            to_add.push_str("        ");
                            to_add.push_str(line_t);
                            to_add.push('\n');
                        }
                    }
                }
                if !to_add.is_empty() {
                    if let Some(target_prop) = self.content.find("</properties>") {
                        self.content.insert_str(target_prop, &format!("{}", to_add));
                    } else {
                        if let Some(dep_start) = self.content.find("<dependencies>") {
                            self.content.insert_str(dep_start, &format!("    <properties>\n{}    </properties>\n\n", to_add));
                        }
                    }
                }
            }
        }
    }

    fn remove_properties(&mut self, downloaded_text: &str) -> bool {
        let mut changed = false;
        if let Some(prop_start) = downloaded_text.find("<properties>") {
            if let Some(prop_end) = downloaded_text[prop_start..].find("</properties>") {
                let prop_block = &downloaded_text[prop_start+12..prop_start + prop_end];
                for line in prop_block.lines() {
                    let line_t = line.trim();
                    if !line_t.is_empty() && !line_t.contains("<java.version>") {
                        if let Some(idx) = self.content.find(line_t) {
                            if let Some(start) = self.content[..idx].rfind('\n') {
                                if let Some(end) = self.content[idx..].find('\n') {
                                    self.content.replace_range(start..idx + end, "");
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        changed
    }

    fn add_plugins(&mut self, downloaded_text: &str) {
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
                        if let Some(b_start) = self.content.find("<build>") {
                            if let Some(p_start) = self.content[b_start..].find("<plugins>") {
                                self.content.insert_str(b_start + p_start + 9, &format!("\n{}\n", plugins_to_add));
                            } else {
                                self.content.insert_str(b_start + 7, &format!("\n        <plugins>\n{}\n        </plugins>\n", plugins_to_add));
                            }
                        }
                    }
                }
            }
        }
    }

    fn cleanup_empty_blocks(&mut self) {
        self.content = self.content.replace("    <properties>\n    </properties>\n", "");
        self.content = self.content.replace("    <dependencyManagement>\n        <dependencies>\n        </dependencies>\n    </dependencyManagement>\n", "");
    }
}
