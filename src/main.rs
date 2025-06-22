use clap::{Parser, Subcommand};
use n8n_workflow_sync::{api, config};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List workflows from the server
    List,
    /// Create a new workflow with the given name
    New { name: String },
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
    }
    Ok(())
}
