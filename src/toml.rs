use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkflowsToml {
    pub n8n: N8nConfig,
    pub workflows: HashMap<String, WorkflowConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct N8nConfig {
    pub host: String,
    pub base_url: String,
    pub r#type: String,
    pub api: ApiConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiConfig {
    pub enabled: bool,
    pub base_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub id: String,
    pub description: String,
    pub workflow_url: String,
    pub webhook_path: String,
    pub used_by: Vec<String>,
    pub environment_var: String,
    pub active: bool,
}

impl WorkflowsToml {
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: WorkflowsToml = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn active_workflows(&self) -> Vec<&WorkflowConfig> {
        self.workflows
            .values()
            .filter(|w| w.active)
            .collect()
    }

    pub fn get_n8n_host(&self) -> &str {
        &self.n8n.host
    }

    pub fn get_api_base_url(&self) -> String {
        format!("{}{}", self.n8n.base_url, self.n8n.api.base_path)
    }
}