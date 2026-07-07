//! Demonstrates how a wallet embeds `fiber-probe-core` to gate the Send
//! button on a preflight check — without ever spawning the CLI as a
//! subprocess.
//!
//! Run with:
//!     cargo run --example wallet_gate -p fiber-probe-core
//!
//! In a real wallet the `channels` vec comes from `RpcClient::list_channels()`.
//! Here we hand-build one so the example is deterministic and doesn't need
//! a running Fiber node.

use fiber_probe_core::channel::{Channel, ChannelState};
use fiber_probe_core::preflight::{self, Asset, FailReason, PreflightResult};

const SHANNONS_PER_CKB: u64 = 100_000_000;

fn main() {
    let channels = vec![
        example_channel("alice_pubkey", "ChannelReady", true, 50 * SHANNONS_PER_CKB, false),
        example_channel("bob_pubkey", "ChannelReady", true, 2 * SHANNONS_PER_CKB, false),
        example_channel("carol_pubkey", "NegotiatingFunding", true, 100 * SHANNONS_PER_CKB, false),
    ];

    println!("=== Wallet integration example ===\n");
    println!("Wallet has {} known channels.\n", channels.len());

    // Three send attempts a wallet might see. Preflight gates each one.
    try_send(&channels, "alice_pubkey", 10 * SHANNONS_PER_CKB);
    try_send(&channels, "bob_pubkey", 10 * SHANNONS_PER_CKB);
    try_send(&channels, "carol_pubkey", 5 * SHANNONS_PER_CKB);
    try_send(&channels, "dave_pubkey", SHANNONS_PER_CKB);
}

/// Simulates what a wallet's Send button handler does before making
/// the actual payment RPC call.
fn try_send(channels: &[Channel], to: &str, amount_shannons: u64) {
    let amount_ckb = amount_shannons / SHANNONS_PER_CKB;
    println!("User clicks Send: {amount_ckb} CKB → {to}");

    match preflight::analyze(channels, to, amount_shannons, Asset::Ckb) {
        PreflightResult::LikelySuccess => {
            println!("  ✓ [send button enabled] — proceed to Fiber send_payment RPC\n");
        }
        PreflightResult::LikelyFail(reason) => {
            let message = user_friendly_message(&reason, to, amount_shannons);
            println!("  ✗ [send button disabled] {message}\n");
        }
        PreflightResult::Unknown(msg) => {
            println!("  ? [send button in warn state] {msg}\n");
        }
    }
}

/// Wallet-owned translation of `FailReason` → the string the user actually sees.
///
/// This function lives in the *wallet*, not in `fiber-probe-core`. The library
/// exposes a stable `FailReason` enum; the wallet decides the wording, locale,
/// and any next-action prompts.
fn user_friendly_message(reason: &FailReason, to: &str, amount_shannons: u64) -> String {
    match reason {
        FailReason::NoDirectChannel => {
            format!("You don't have a channel to {to}. Open one or route through a peer.")
        }
        FailReason::AssetMismatch => {
            format!("No matching-asset channel to {to}.")
        }
        FailReason::ChannelNotReady { state } => {
            format!(
                "The channel to {to} is still opening (currently: {state}). Try again in a moment."
            )
        }
        FailReason::ChannelDisabled => {
            format!("The channel to {to} is temporarily disabled by the network.")
        }
        FailReason::InsufficientLiquidity {
            available,
            required,
        } => {
            let want = required / SHANNONS_PER_CKB;
            let have = available / SHANNONS_PER_CKB;
            let _ = amount_shannons; // caller already knows the amount
            format!("You need {want} CKB to send but the channel to {to} only has {have} CKB available.")
        }
    }
}

fn example_channel(
    peer: &str,
    state: &str,
    enabled: bool,
    local_balance: u64,
    is_udt: bool,
) -> Channel {
    Channel {
        channel_id: format!("0x{peer}_channel"),
        pubkey: peer.to_string(),
        state: ChannelState {
            state_name: state.to_string(),
        },
        enabled,
        local_balance,
        remote_balance: 0,
        funding_udt_type_script: if is_udt {
            Some(serde_json::Value::Null)
        } else {
            None
        },
    }
}
