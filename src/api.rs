use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::N8nConfig;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
}


#[derive(Deserialize)]
struct WorkflowList {
    data: Vec<Workflow>,
}

pub async fn list_workflows(config: &N8nConfig) -> Result<Vec<Workflow>> {
    let client = Client::new();
    let url = config.endpoint("workflows");
    let resp = client
        .get(url)
        .header("X-N8N-API-KEY", &config.api_key)
        .send()
        .await?
        .error_for_status()?;
    let list: WorkflowList = resp.json().await?;
    Ok(list.data)
}

pub async fn create_workflow(config: &N8nConfig, name: &str) -> Result<Workflow> {
    let client = Client::new();
    let url = config.endpoint("workflows");
    let body = json!({
        "name": name,
        "nodes": [],
        "connections": {},
        "settings": {}
    });
    let resp = client
        .post(url)
        .header("X-N8N-API-KEY", &config.api_key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    let wf: Workflow = resp.json().await?;
    Ok(wf)
}
