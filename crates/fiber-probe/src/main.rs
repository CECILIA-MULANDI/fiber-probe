use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use fiber_probe_core::classifier::{self, Classification};
use fiber_probe_core::client::RpcClient;
use fiber_probe_core::error::Error as CoreError;
use fiber_probe_core::preflight::{self, Asset, FailReason, PreflightResult};
use fiber_probe_core::rpc::RpcError;

const SHANNONS_PER_CKB: u64 = 100_000_000;

/// Render a shannons amount as CKB, trimming trailing zeros from the fraction.
fn format_ckb(shannons: u64) -> String {
    let whole = shannons / SHANNONS_PER_CKB;
    let frac = shannons % SHANNONS_PER_CKB;
    if frac == 0 {
        format!("{whole} CKB")
    } else {
        let frac_str = format!("{frac:08}");
        let trimmed = frac_str.trim_end_matches('0');
        format!("{whole}.{trimmed} CKB")
    }
}

/// Truncate a pubkey (66 chars) to `prefix...suffix` for terminal skim.
/// Not used on channel_ids — those we always print in full since users may
/// want to feed them back to Fiber for follow-up calls (diagnose, etc.).
fn short_pubkey(pk: &str) -> String {
    if pk.len() > 12 {
        format!("{}...{}", &pk[..8], &pk[pk.len() - 4..])
    } else {
        pk.to_string()
    }
}

/// Render a Classification either as pretty terminal output or as JSON.
fn print_classification(as_json: bool, payment_hash: &str, c: &Classification) -> Result<()> {
    if as_json {
        println!("{}", serde_json::to_string_pretty(c)?);
    } else {
        println!("{} Payment {payment_hash} failed", "✗".red().bold());
        println!("  category: {:?}", c.category);
        println!("  fix:      {}", c.suggested_fix);
        println!("  raw:      [{}] {}", c.raw_code, c.raw_message);
    }
    Ok(())
}

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
            let summary = client.summary().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                let node = &summary.node;
                println!("Fiber node status");
                println!("  version:  {}", node.version);
                println!("  pubkey:   {}", node.pubkey);
                println!(
                    "  name:     {}",
                    node.node_name.as_deref().unwrap_or("<unset>")
                );
                println!("  chain:    {}", node.chain_hash);
                println!("  peers:    {}", node.peers_count);
                println!(
                    "  channels: {} open, {} pending, {} routable now",
                    node.channel_count,
                    node.pending_channel_count,
                    summary.routable_channel_count()
                );
                println!(
                    "  CKB liquidity (send-side): {}",
                    format_ckb(summary.total_ckb_local_balance())
                );

                if summary.channels.is_empty() {
                    println!("\nNo channels.");
                } else {
                    println!("\nChannels:");
                    for c in &summary.channels {
                        let asset = if c.funding_udt_type_script.is_some() {
                            "UDT"
                        } else {
                            "CKB"
                        };
                        let state_colored = if c.state.state_name == "ChannelReady" {
                            c.state.state_name.green()
                        } else {
                            c.state.state_name.yellow()
                        };
                        let disabled_marker = if c.enabled {
                            "".normal()
                        } else {
                            " (disabled)".red()
                        };
                        // Full channel_id — hashes are useless when truncated
                        // (can't reverse-lookup); users may feed them to `diagnose`.
                        println!("  - {}", c.channel_id);
                        println!("      peer:    {}", short_pubkey(&c.pubkey));
                        println!("      state:   {state_colored}{disabled_marker}");
                        println!("      asset:   {asset}");
                        println!(
                            "      balance: {} local / {} remote",
                            format_ckb(c.local_balance),
                            format_ckb(c.remote_balance)
                        );
                    }
                }
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
                        println!(
                            "{} Payment likely to succeed via direct channel to {to}",
                            "✓".green().bold()
                        );
                    }
                    PreflightResult::LikelyFail(reason) => {
                        println!("{} Payment likely to fail", "✗".red().bold());
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
                                    "  reason: need {} to send, have {} available",
                                    format_ckb(*required),
                                    format_ckb(*available)
                                );
                            }
                        }
                    }
                    PreflightResult::Unknown(msg) => {
                        println!("{} Cannot determine: {msg}", "?".yellow().bold());
                    }
                }
            }
            Ok(())
        }

        Command::Diagnose { payment_hash } => {
            match client.get_payment(&payment_hash).await {
                Ok(status) => match status.status.as_str() {
                    "Success" => {
                        if cli.json {
                            println!("{}", serde_json::to_string_pretty(&status)?);
                        } else {
                            println!(
                                "{} Payment {} succeeded",
                                "✓".green().bold(),
                                status.payment_hash
                            );
                            println!("  fee paid: {}", format_ckb(status.fee));
                            println!("  nothing to diagnose.");
                        }
                    }
                    "Failed" => {
                        let raw = status
                            .failed_error
                            .as_deref()
                            .unwrap_or("payment failed with no error message");
                        // Classifier takes an RpcError; wrap the raw string.
                        // Code 0 is a sentinel: "not an RPC-layer failure."
                        let synthetic = RpcError {
                            code: 0,
                            message: raw.to_string(),
                            data: None,
                        };
                        let classification = classifier::classify(&synthetic);
                        print_classification(cli.json, &status.payment_hash, &classification)?;
                    }
                    other => {
                        if cli.json {
                            println!("{}", serde_json::to_string_pretty(&status)?);
                        } else {
                            println!(
                                "{} Payment {} is {other} — still routing, no diagnosis yet.",
                                "?".yellow().bold(),
                                status.payment_hash
                            );
                        }
                    }
                },
                Err(CoreError::Rpc(rpc_error)) => {
                    // Lookup itself failed. Most common cause: hash doesn't
                    // match a known payment session. Classify Fiber's error.
                    let classification = classifier::classify(&rpc_error);
                    print_classification(cli.json, &payment_hash, &classification)?;
                }
                Err(other) => return Err(other.into()),
            }
            Ok(())
        }
    }
}
