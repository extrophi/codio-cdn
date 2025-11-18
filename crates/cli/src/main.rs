use clap::{Parser, Subcommand};
use codio_content_id::ContentId;
use codio_dht::{DhtConfig, DhtEvent, DhtNode};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "codio-cdn")]
#[command(about = "Decentralized content delivery CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Publish content and get CID
    Publish {
        /// File or directory to publish
        path: PathBuf,

        /// Announce to DHT
        #[arg(long, default_value_t = true)]
        announce: bool,
    },

    /// Retrieve content by CID
    Get {
        /// Content ID (CID) to retrieve
        cid: String,

        /// Output path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show CID for content without publishing
    Hash {
        /// File to hash
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    match cli.command {
        Commands::Publish { path, announce } => {
            publish_content(path, announce).await?;
        }
        Commands::Get { cid, output } => {
            get_content(&cid, output).await?;
        }
        Commands::Hash { path } => {
            hash_content(path)?;
        }
    }

    Ok(())
}

async fn publish_content(path: PathBuf, announce: bool) -> anyhow::Result<()> {
    println!("{}", "Publishing content...".cyan());

    // Read content
    let content = fs::read(&path)?;
    println!("  {} Read {} bytes", "✓".green(), content.len());

    // Generate CID
    let cid = ContentId::new(&content);
    println!(
        "  {} Generated CID: {}",
        "✓".green(),
        cid.to_string().bright_blue()
    );

    // Announce to DHT if requested
    if announce {
        println!("{}", "Announcing to DHT...".cyan());

        let config = DhtConfig::default();
        let (mut node, _event_rx) = DhtNode::new(config).await?;

        // Listen on random port
        let listen_addr = "/ip4/0.0.0.0/tcp/0".parse()?;
        node.listen(listen_addr).await?;

        // Announce content
        node.provide(cid.clone()).await?;
        println!("  {} Content announced", "✓".green());

        // Wait briefly for DHT propagation
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    println!("\n{}", "Success!".bright_green().bold());
    println!("CID: {}", cid.to_string().bright_blue());

    Ok(())
}

async fn get_content(cid_str: &str, _output: Option<PathBuf>) -> anyhow::Result<()> {
    println!("{} {}", "Retrieving:".cyan(), cid_str.bright_blue());

    // Parse CID
    let cid = ContentId::from_str(cid_str)?;
    println!("  {} CID parsed", "✓".green());

    // Create DHT node
    println!("{}", "Searching DHT...".cyan());
    let config = DhtConfig::default();
    let (mut node, mut event_rx) = DhtNode::new(config).await?;

    // Listen
    let listen_addr = "/ip4/0.0.0.0/tcp/0".parse()?;
    node.listen(listen_addr).await?;

    // Find providers
    node.find_providers(cid.clone()).await?;

    // Wait for providers
    tokio::select! {
        Some(event) = event_rx.recv() => {
            match event {
                DhtEvent::ProvidersFound { cid: _, providers } => {
                    println!("  {} Found {} providers", "✓".green(), providers.len());

                    // TODO: Actually download from providers
                    println!("\n{}", "Note: Content download not yet implemented".yellow());
                    println!("Providers found: {:?}", providers);
                }
                _ => {}
            }
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            println!("  {} Timeout: No providers found", "✗".red());
        }
    }

    Ok(())
}

fn hash_content(path: PathBuf) -> anyhow::Result<()> {
    println!("{} {}", "Hashing:".cyan(), path.display());

    let content = fs::read(&path)?;
    let cid = ContentId::new(&content);

    println!("\nCID: {}", cid.to_string().bright_blue());
    println!("Size: {} bytes", content.len());

    Ok(())
}
