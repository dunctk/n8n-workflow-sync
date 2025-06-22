use clap::{Parser, Subcommand};
use n8n_workflow_sync::{api, config};
use std::fs;
use std::path::PathBuf;

use anyhow::Context;

#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = "Pull, edit and push n8n workflows using Git. \n\
Set the N8N_HOST and N8N_API_KEY environment variables to authenticate with your n8n instance.",
    after_help = "ENVIRONMENT VARIABLES:\n    N8N_HOST     Base URL of the n8n instance\n    N8N_API_KEY  API key for authentication",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List workflows from the server
    List,
    /// Create a new workflow with the given name
    New {
        /// Name for the newly created workflow
        name: String,
    },
    /// Download a workflow JSON to the given file
    Pull {
        /// ID of the workflow to download
        id: String,
        /// Path to save the workflow JSON file
        path: PathBuf,
    },
    /// Upload a modified workflow JSON from a file
    Push {
        /// ID of the workflow to update
        id: String,
        /// Path containing the modified workflow JSON
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => {
            let cfg = config::N8nConfig::from_env()?;
            let workflows = api::list_workflows(&cfg).await?;
            for wf in workflows {
                println!("{}: {}", wf.id, wf.name);
            }
        }
        Commands::New { name } => {
            let cfg = config::N8nConfig::from_env()?;
            let wf = api::create_workflow(&cfg, &name).await?;
            println!("Created workflow {}: {}", wf.id, wf.name);
        }
        Commands::Pull { id, path } => {
            let cfg = config::N8nConfig::from_env()?;
            let wf_json = api::get_workflow(&cfg, &id).await?;
            let data = serde_json::to_vec_pretty(&wf_json)?;
            fs::write(&path, data).with_context(|| format!("write {}", path.display()))?;
            println!("Downloaded workflow {} to {}", id, path.display());
        }
        Commands::Push { id, path } => {
            let cfg = config::N8nConfig::from_env()?;
            let data = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
            let json: serde_json::Value = serde_json::from_str(&data)?;
            let wf = api::update_workflow(&cfg, &id, &json).await?;
            println!("Updated workflow {}: {}", wf.id, wf.name);
        }
    }
    Ok(())
}
