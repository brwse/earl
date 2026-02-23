use std::collections::BTreeMap;
use std::path::PathBuf;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use super::schema::{CommandMode, CommandTemplate, ProviderEnvironments};
use earl_core::with::AsPath;

#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct TemplateCatalog {
    pub entries: BTreeMap<String, TemplateCatalogEntry>,
}

impl TemplateCatalog {
    pub fn empty() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&TemplateCatalogEntry> {
        self.entries.get(key)
    }

    pub fn upsert(&mut self, key: String, entry: TemplateCatalogEntry) {
        self.entries.insert(key, entry);
    }

    pub fn values(&self) -> impl Iterator<Item = &TemplateCatalogEntry> {
        self.entries.values()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct TemplateCatalogEntry {
    pub key: String,
    pub provider: String,
    pub command: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub categories: Vec<String>,
    pub mode: CommandMode,
    pub source: TemplateSource,
    pub template: CommandTemplate,
    pub provider_environments: Option<ProviderEnvironments>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct TemplateSource {
    #[rkyv(with = AsPath)]
    pub path: PathBuf,
    pub scope: TemplateScope,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum TemplateScope {
    Local,
    Global,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    #[cfg(feature = "bash")]
    fn catalog_preserves_provider_environments_through_upsert_and_get() {
        use earl_core::schema::{CommandMode, ResultTemplate};
        use earl_protocol_bash::{BashOperationTemplate, BashScriptTemplate};

        use super::super::schema::{Annotations, CommandTemplate, OperationTemplate};

        let mut envs = BTreeMap::new();
        let mut prod_vars = BTreeMap::new();
        prod_vars.insert(
            "base_url".to_string(),
            "https://api.example.com".to_string(),
        );
        envs.insert("production".to_string(), prod_vars);

        let pe = ProviderEnvironments {
            default: Some("production".to_string()),
            secrets: vec![],
            environments: envs,
        };

        let op = OperationTemplate::Bash(BashOperationTemplate {
            bash: BashScriptTemplate {
                script: "echo hi".into(),
                env: None,
                cwd: None,
                sandbox: None,
            },
            transport: None,
            stream: false,
        });

        let cmd = CommandTemplate {
            title: "T".into(),
            summary: "S".into(),
            description: "D".into(),
            categories: vec![],
            annotations: Annotations::default(),
            params: vec![],
            operation: op,
            result: ResultTemplate {
                output: "{{ result }}".into(),
                ..Default::default()
            },
            environment_overrides: BTreeMap::new(),
        };

        let entry = TemplateCatalogEntry {
            key: "myservice.ping".to_string(),
            provider: "myservice".to_string(),
            command: "ping".to_string(),
            title: "Ping".to_string(),
            summary: "Ping the service".to_string(),
            description: "Ping.".to_string(),
            categories: vec![],
            mode: CommandMode::Read,
            source: TemplateSource {
                path: PathBuf::from("myservice.hcl"),
                scope: TemplateScope::Local,
            },
            template: cmd,
            provider_environments: Some(pe),
        };

        let mut catalog = TemplateCatalog::empty();
        catalog.upsert("myservice.ping".to_string(), entry);

        let retrieved = catalog.get("myservice.ping").unwrap();
        let envs = retrieved.provider_environments.as_ref().unwrap();
        assert_eq!(envs.default.as_deref(), Some("production"));
        assert!(envs.environments.contains_key("production"));
        assert_eq!(
            envs.environments["production"]["base_url"],
            "https://api.example.com"
        );

        // Verify missing key returns None
        assert!(catalog.get("nonexistent.cmd").is_none());
    }
}
