use clap::{Parser, Subcommand};
use dialoguer::Confirm;
use git2::{Repository, Signature};
use n8n_workflow_sync::{api, config, nodes};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;

/// Convert a workflow name into a filesystem-friendly slug
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Attempt to locate a default workflow JSON file in the current directory.
///
/// Preference is given to a file named `workflow.json`. If exactly one other
/// `.json` file exists, that is returned. Otherwise an error is produced.
fn default_json_path() -> anyhow::Result<PathBuf> {
    let preferred = PathBuf::from("workflow.json");
    if preferred.exists() {
        return Ok(preferred);
    }

    let mut json_files = vec![];
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|ext| ext == "json").unwrap_or(false) {
            json_files.push(path);
        }
    }

    if json_files.len() == 1 {
        Ok(json_files.remove(0))
    } else if json_files.is_empty() {
        Err(anyhow::anyhow!("No JSON files found"))
    } else {
        Err(anyhow::anyhow!(
            "Multiple JSON files found. Please specify which one to push"
        ))
    }
}

/// Remove fields not accepted by the Public API when updating a workflow.
fn sanitize_for_update(json: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Map, Value};

    let allowed = [
        "name",
        "nodes",
        "connections",
        "settings",
        "staticData",
        "tags",
        "active",
    ];

    let mut obj = Map::new();
    for key in allowed.iter() {
        if let Some(v) = json.get(*key) {
            obj.insert((*key).to_string(), v.clone());
        }
    }
    Value::Object(obj)
}

#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = "Pull, edit and push n8n workflows using Git. \n\
Set the N8N_HOST and N8N_API_KEY environment variables to authenticate with your n8n instance.\n\n\
Examples:\n  \
n8n-workflow-sync list\n  \
n8n-workflow-sync new \"My New Workflow\"\n  \
n8n-workflow-sync pull 123 workflow.json\n  \
n8n-workflow-sync push 123 workflow.json",
    after_help = "ENVIRONMENT VARIABLES:\n    N8N_HOST     Base URL of the n8n instance (e.g., https://your-n8n.example.com)\n    N8N_API_KEY  API key for authentication",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all workflows from the n8n server
    List,
    /// Create a new workflow with the given name and download it as JSON
    ///
    /// This command creates a new workflow on the n8n server, downloads the workflow
    /// JSON, creates a directory with the workflow name, and initializes a git repository.
    New {
        /// Name for the newly created workflow (required)
        ///
        /// Example: "My New Workflow" or "data-processing-pipeline"
        name: String,
    },
    /// Download a workflow JSON file from the server
    Pull {
        /// ID of the workflow to download
        id: String,
        /// Optional path to save the workflow JSON. Can be a directory
        /// or a file. Defaults to a directory named after the workflow.
        path: Option<PathBuf>,
    },
    /// Upload a modified workflow JSON file to the server
    ///
    /// If no ID or path is provided, the command will attempt to
    /// locate a single JSON file in the current directory and read
    /// the `id` field from it.
    Push {
        /// ID of the workflow to update. If omitted, the ID will be
        /// read from the JSON file.
        id: Option<String>,
        /// Path to the workflow JSON file to upload. Defaults to
        /// `workflow.json` or the only JSON file in the current
        /// directory.
        path: Option<PathBuf>,
    },
    /// Download and replace the binary with the latest release from GitHub
    Upgrade,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Validate environment variables early with helpful error messages
    let cfg = config::N8nConfig::from_env().with_context(|| {
        "Failed to load configuration. Please ensure N8N_HOST and N8N_API_KEY environment variables are set.\n\
        Example:\n  \
        export N8N_HOST=https://your-n8n.example.com\n  \
        export N8N_API_KEY=your-api-key-here"
    })?;

    match cli.command {
        Commands::List => {
            println!("Fetching workflows from {}...", cfg.host);
            let workflows = api::list_workflows(&cfg).await.with_context(
                || "Failed to list workflows. Please check your N8N_HOST and N8N_API_KEY",
            )?;

            if workflows.is_empty() {
                println!("No workflows found on the server.");
            } else {
                println!("Found {} workflows:", workflows.len());
                for wf in workflows {
                    println!("  {}: {}", wf.id, wf.name);
                }
            }
        }
        Commands::New { name } => {
            if name.trim().is_empty() {
                return Err(anyhow::anyhow!("Workflow name cannot be empty"));
            }

            println!("Creating new workflow: \"{}\"", name);
            let wf = api::create_workflow(&cfg, &name)
                .await
                .with_context(|| format!("Failed to create workflow \"{}\"", name))?;

            println!("Created workflow with ID: {}", wf.id);

            let wf_json = api::get_workflow(&cfg, &wf.id)
                .await
                .with_context(|| format!("Failed to download workflow {}", wf.id))?;

            let slug = slugify(&wf.name);
            let dir = PathBuf::from(&slug);

            fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create directory {}", dir.display()))?;

            let json_path = dir.join("workflow.json");
            let data = serde_json::to_vec_pretty(&wf_json)?;
            fs::write(&json_path, data)
                .with_context(|| format!("Failed to write workflow to {}", json_path.display()))?;

            nodes::save_node_versions(&dir)
                .await
                .with_context(|| "Failed to fetch node versions")?;

            // Initialize git repository
            let repo = Repository::init(&dir).with_context(|| {
                format!("Failed to initialize git repository in {}", dir.display())
            })?;
            let mut index = repo.index()?;
            index.add_path(Path::new("workflow.json"))?;
            index.write()?;
            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            let sig = Signature::now("n8n-workflow-sync", "n8n@localhost")?;
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("feat: sync from n8n (workflow {})", wf.id),
                &tree,
                &[],
            )?;

            println!(
                "✓ Created workflow {} in directory: {}",
                wf.id,
                dir.display()
            );
            println!("✓ Initialized git repository with initial commit");
        }
        Commands::Pull { id, path } => {
            let wf_json = api::get_workflow(&cfg, &id)
                .await
                .with_context(|| format!("Failed to download workflow {}", id))?;

            // Determine directory and file path
            let mut dir = PathBuf::new();
            let json_path = if let Some(p) = path {
                if p.is_dir() || p.extension().is_none() {
                    dir = p.clone();
                    dir.join("workflow.json")
                } else {
                    if let Some(parent) = p.parent() {
                        dir = parent.to_path_buf();
                    }
                    p
                }
            } else {
                let name = wf_json.get("name").and_then(|v| v.as_str()).unwrap_or(&id);
                dir = PathBuf::from(slugify(name));
                dir.join("workflow.json")
            };

            if !dir.exists() {
                fs::create_dir_all(&dir)
                    .with_context(|| format!("Failed to create directory {}", dir.display()))?;
            }

            if json_path.exists() {
                if !Confirm::new()
                    .with_prompt(format!("Overwrite {}?", json_path.display()))
                    .default(false)
                    .interact()?
                {
                    println!("Aborted");
                    return Ok(());
                }
            }

            let data = serde_json::to_vec_pretty(&wf_json)?;
            fs::write(&json_path, data)
                .with_context(|| format!("Failed to write to {}", json_path.display()))?;

            nodes::save_node_versions(&dir)
                .await
                .with_context(|| "Failed to fetch node versions")?;

            // Initialise git repo if none exists
            let repo = match Repository::open(&dir) {
                Ok(r) => r,
                Err(_) => {
                    println!("Initializing git repository in {}...", dir.display());
                    Repository::init(&dir).with_context(|| {
                        format!("Failed to initialize git repository in {}", dir.display())
                    })?
                }
            };

            // Commit the workflow.json file
            let mut index = repo.index()?;
            let rel = json_path.strip_prefix(&dir).unwrap_or(&json_path);
            index.add_path(rel)?;
            index.write()?;
            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            let sig = Signature::now("n8n-workflow-sync", "n8n@localhost")?;
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("feat: sync from n8n (workflow {})", id),
                &tree,
                &[],
            )?;

            println!("✓ Downloaded workflow {} to {}", id, json_path.display());
        }
        Commands::Push { id, path } => {
            // Determine the path to use. If none provided, try common defaults.
            let path = match path {
                Some(p) => p,
                None => default_json_path().with_context(
                    || "Unable to determine workflow JSON file. Please specify a path.",
                )?,
            };

            let data = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let json: serde_json::Value = serde_json::from_str(&data)
                .with_context(|| format!("Failed to parse JSON in {}", path.display()))?;

            // Determine workflow ID. Command line argument overrides JSON field.
            let id = match id {
                Some(v) => v,
                None => json
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Workflow ID not provided and not found in JSON")
                    })?,
            };

            println!("Uploading {} to workflow {}...", path.display(), id);

            let body = sanitize_for_update(&json);
            let wf = api::update_workflow(&cfg, &id, &body)
                .await
                .with_context(|| format!("Failed to update workflow {}", id))?;
            println!("✓ Updated workflow {}: {}", wf.id, wf.name);
        }
        Commands::Upgrade => {
            println!("Checking for updates...");
            self_update::backends::github::Update::configure()
                .repo_owner("dunctk")
                .repo_name("n8n-workflow-sync")
                .bin_name("n8n-workflow-sync")
                .show_download_progress(true)
                .current_version(env!("CARGO_PKG_VERSION"))
                .build()?
                .update()
                .with_context(|| "Failed to upgrade to latest release")?;
            println!("✓ Updated to latest version");
        }
    }
    Ok(())
}
