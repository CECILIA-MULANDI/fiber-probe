use anyhow::Result;
use clap::{Parser, Subcommand};
use fiber_probe_core::client::RpcClient;

/// Fiber network payment diagnostics
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Fiber node RPC endpoint, e.g. http://
    #[arg(long, global = true, default_value = "http://127.0.0.1:8227")]
    rpc_url: String,

    /// Output - some machine readable JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Shows connected node status: version, peers, channels
    Status,

    /// Pre-flight a payment: can this route succeed?
    Check {
        /// Destination node pubkey(should be 33byte hex)
        #[arg(long)]
        to: String,

        /// Amount in shannons
        #[arg(long)]
        amount: u64,
    },
    /// Classify a payment failure into an actionable category.
    Diagnose {
        /// Payment hash to look up.
        payment_hash: String,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = RpcClient::new(&cli.rpc_url);

    match cli.command {
        Command::Status => {
            let info = client.node_info().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("Fiber node status");
                println!("  version:  {}", info.version);
                println!("  pubkey:   {}", info.pubkey);
                println!(
                    "  name:     {}",
                    info.node_name.as_deref().unwrap_or("<unset>")
                );
                println!("  chain:    {}", info.chain_hash);
                println!("  peers:    {}", info.peers_count);
                println!(
                    "  channels: {} open, {} pending",
                    info.channel_count, info.pending_channel_count
                );
            }
            Ok(())
        }
        Command::Check { .. } => todo!("check"),
        Command::Diagnose { .. } => todo!("diagnose"),
    }
}
