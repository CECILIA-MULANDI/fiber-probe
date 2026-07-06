use anyhow::Result;
use clap::{Parser, Subcommand};
use fiber_probe_core::client::RpcClient;
use fiber_probe_core::preflight::{self, Asset, FailReason, PreflightResult};

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
        /// Asset to send. `ckb` for native CKB, `udt` for any UDT.
        #[arg(long, default_value = "ckb")]
        asset: String,
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
        Command::Check { to, amount, asset } => {
            let asset = match asset.to_lowercase().as_str() {
                "ckb" => Asset::Ckb,
                "udt" => Asset::Udt,
                other => anyhow::bail!("unknown asset: {other}. Try `ckb` or `udt`."),
            };

            let channels = client.list_channels().await?;
            let result = preflight::analyze(&channels, &to, amount, asset);

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                match &result {
                    PreflightResult::LikelySuccess => {
                        println!("✓ Payment likely to succeed via direct channel to {to}");
                    }
                    PreflightResult::LikelyFail(reason) => {
                        println!("✗ Payment likely to fail");
                        match reason {
                            FailReason::NoDirectChannel => {
                                println!("  reason: no channel to peer {to}");
                            }
                            FailReason::AssetMismatch => {
                                println!("  reason: no channel with matching asset to peer {to}");
                            }
                            FailReason::ChannelNotReady { state } => {
                                println!("  reason: channel is in state {state}, not ChannelReady");
                            }
                            FailReason::ChannelDisabled => {
                                println!("  reason: channel is disabled");
                            }
                            FailReason::InsufficientLiquidity {
                                available,
                                required,
                            } => {
                                println!(
                                    "  reason: need {required} shannons, have {available} available"
                                );
                            }
                        }
                    }
                    PreflightResult::Unknown(msg) => {
                        println!("? Cannot determine: {msg}");
                    }
                }
            }
            Ok(())
        }

        Command::Diagnose { .. } => todo!("diagnose"),
    }
}
