use crate::metadata::Metadata;

const OVERRIDES: &[(&str, &str)] = &[
    ("transformation", "tanzu-scg-transformation"),
    ("spring-cloud-starter-gateway-server-webflux", "cloud-gateway"),
    ("spring-rabbit-stream", "amqp-streams"),
    ("jcc", "db2"),
    ("spring-cloud-azure-starter", "azure-support"),
    ("spring-cloud-azure-starter-active-directory", "azure-active-directory"),
    ("spring-cloud-azure-starter-cosmos", "azure-cosmos-db"),
    ("spring-cloud-azure-starter-keyvault", "azure-keyvault"),
    ("spring-cloud-azure-starter-storage", "azure-storage"),
    ("spring-pulsar-reactive-spring-boot-starter", "pulsar-reactive"),
    ("solace-spring-boot-starter", "solace"),
    ("camel-spring-boot-starter", "camel"),
    ("spring-data-rest-hal-explorer", "data-rest-explorer"),
    ("graphql-dgs-spring-graphql-starter", "netflix-dgs"),
    ("com.netflix.dgs.codegen", "dgs-codegen"),
];

pub struct ArtifactMapper;

impl ArtifactMapper {
    pub fn artifact_to_key(artifact_id: &str, meta: &Metadata) -> Option<String> {
        // Check overrides first
        for (artifact, key) in OVERRIDES {
            if *artifact == artifact_id {
                return Some(key.to_string());
            }
        }

        // Fuzzy matching against metadata
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
        best_match
    }

    pub fn detect_existing(artifact_ids: &[String], meta: &Metadata) -> Vec<String> {
        let mut existing = std::collections::HashSet::new();
        for artifact_id in artifact_ids {
            if let Some(key) = Self::artifact_to_key(artifact_id, meta) {
                existing.insert(key);
            }
        }
        let mut res: Vec<String> = existing.into_iter().collect();
        res.sort();
        res
    }

    pub fn resolve_artifacts_for_removal(to_remove: &[String], artifact_ids: &[String], meta: &Metadata) -> std::collections::HashSet<String> {
        let mut artifacts_to_remove = std::collections::HashSet::new();
        for artifact_id in artifact_ids {
            if let Some(key) = Self::artifact_to_key(artifact_id, meta) {
                if to_remove.contains(&key) {
                    artifacts_to_remove.insert(artifact_id.clone());
                }
            }
        }
        artifacts_to_remove
    }
}
