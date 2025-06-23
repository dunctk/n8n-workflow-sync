use anyhow::Result;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct Tree {
    tree: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
}

/// Fetch the latest node versions from the n8n repository
pub async fn fetch_node_versions() -> Result<HashMap<String, u32>> {
    let client = Client::new();
    let tree_url = "https://api.github.com/repos/n8n-io/n8n/git/trees/master?recursive=1";
    let tree: Tree = client
        .get(tree_url)
        .header("User-Agent", "n8n-workflow-sync")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let file_re = Regex::new(r"^packages/nodes-base/nodes/([^/]+)/.*\.node.[jt]s$")?;
    let version_re = Regex::new(r"version:\s*(\d+)")?;
    let mut map: HashMap<String, u32> = HashMap::new();

    for entry in tree.tree {
        if entry.entry_type == "blob" {
            if let Some(caps) = file_re.captures(&entry.path) {
                let node_name = caps.get(1).unwrap().as_str().to_string();
                let raw_url = format!(
                    "https://raw.githubusercontent.com/n8n-io/n8n/master/{}",
                    entry.path
                );
                let text = client
                    .get(&raw_url)
                    .header("User-Agent", "n8n-workflow-sync")
                    .send()
                    .await?
                    .error_for_status()?
                    .text()
                    .await?;

                if let Some(v_caps) = version_re.captures(&text) {
                    if let Ok(v) = v_caps[1].parse::<u32>() {
                        map.entry(node_name)
                            .and_modify(|e| *e = (*e).max(v))
                            .or_insert(v);
                    }
                }
            }
        }
    }

    Ok(map)
}

/// Fetch node versions and save them as `node-versions.json` in the given directory
pub async fn save_node_versions<P: AsRef<Path>>(dir: P) -> Result<()> {
    let versions = fetch_node_versions().await?;
    let path = dir.as_ref().join("node-versions.json");
    let data = serde_json::to_vec_pretty(&versions)?;
    fs::write(path, data)?;
    Ok(())
}
