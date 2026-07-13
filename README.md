# Fiber Probe

**Payment Readiness SDK for Fiber Network.** Pre-flight and diagnose Fiber payments before they can fail.

> Built for [Gone in 60ms: Fiber Network Infrastructure Hackathon](https://talk.nervos.org/t/gone-in-60ms-fiber-network-infrastructure-hackathon-announcement/10418) (Jul 2026). Category 2: Node, Routing, and Diagnostics Infrastructure.

## What it does

When a Fiber payment fails today, developers get a raw protocol error:

```
"Send payment error: Failed to build route, Insufficient balance: max outbound liquidity 10000000000 is insufficient, required amount: 200000000000"
```

No pre-flight signal, no category, no fix. A wallet has to string-match Fiber's message text to react at all.

Fiber Probe wraps the Fiber JSON-RPC and adds three tools on top:

1. **`check`**: simulate payment routability *before* attempting.
2. **`diagnose`**: classify a raw error into a stable category with a suggested fix.
3. **`status`**: channel-level summary of your node's usable capacity.

All commands emit `--json` for programmatic use.

**Before / after** on the same failure:

```bash
$ fiber-probe check --to 036e3df4...8587 --amount 20000000000
✗ Payment likely to fail
  reason: need 200 CKB to send, have 101 CKB available
```

Zero API calls to Fiber. The wallet knows before rendering the Send button.

## Quickstart

Rust 1.80+ required. Fiber node optional (SDK example runs standalone).

```bash
git clone https://github.com/CECILIA-MULANDI/fiber-probe.git
cd fiber-probe
cargo build --release

# Against your Fiber node (default: http://127.0.0.1:8227)
./target/release/fiber-probe status
./target/release/fiber-probe check --to <peer-pubkey> --amount <shannons>
./target/release/fiber-probe diagnose <payment-hash>

# Without a Fiber node: SDK integration example
cargo run --example wallet_gate -p fiber-probe-core
```

Fiber node setup: see [fiber.world](https://www.fiber.world/).

## Commands

### `status`

```
$ fiber-probe status
Fiber node status
  version:  0.9.0-rc7
  pubkey:   02bae0aa...231a5b1
  peers:    1
  channels: 1 open, 0 pending, 1 routable now
  CKB liquidity (send-side): 101 CKB

Channels:
  - 0xc19bdc7c...b35053
      peer:    036e3df4...8587
      state:   ChannelReady
      asset:   CKB
      balance: 101 CKB local / 0 CKB remote
```

### `check`

```
$ fiber-probe check --to 036e3df4...8587 --amount 5000000000
✓ Payment likely to succeed via direct channel to 036e3df4...
```

Flags: `--to <pubkey>` (required), `--amount <shannons>` (required), `--asset ckb|udt` (default `ckb`).

### `diagnose`

```
$ fiber-probe diagnose 0x0000...0000
✗ Payment 0x0000...0000 failed
  category: InvalidInput
  fix:      Check the payment arguments (destination pubkey, amount, asset) for typos or bad encoding.
  raw:      [-32000] InvalidParameter: Payment session not found: Hash256(0x0000...)
```

### Global flags

`--rpc-url <URL>` (default `http://127.0.0.1:8227`) and `--json` on any command.

## SDK integration

`fiber-probe-core` is a standalone library. A wallet embeds it and never touches the CLI:

```rust
use fiber_probe_core::client::RpcClient;
use fiber_probe_core::preflight::{self, Asset, PreflightResult, FailReason};

let client = RpcClient::new("http://127.0.0.1:8227");
let channels = client.list_channels().await?;

match preflight::analyze(&channels, &recipient, amount_shannons, Asset::Ckb) {
    PreflightResult::LikelySuccess => enable_send_button(),
    PreflightResult::LikelyFail(reason) => show_user(translate(reason)),
    PreflightResult::Unknown(msg) => show_user_warn(msg),
}
```

Full runnable simulation: [`crates/fiber-probe-core/examples/wallet_gate.rs`](crates/fiber-probe-core/examples/wallet_gate.rs).

The pre-flight and classifier are pure functions, so downstream code can test with hand-built channel data and mock `RpcError` values. No Fiber node required for CI.

## Failure taxonomy

**Pre-flight (`FailReason`)** returned by `preflight::analyze`:

| Variant | When |
|---|---|
| `NoDirectChannel` | No channel to the destination peer |
| `AssetMismatch` | Channels exist but none carry the requested asset |
| `ChannelNotReady { state }` | Channel isn't in `ChannelReady` yet |
| `ChannelDisabled` | Ready channel exists but flagged disabled |
| `InsufficientLiquidity { available, required }` | Not enough send-side balance |

**Post-hoc (`FailureCategory`)** returned by `classifier::classify`:

| Category | Suggested fix |
|---|---|
| `LiquidityShortfall` | Reduce amount, open a larger channel, or rebalance. |
| `PeerOffline` | Wait for peer, or route through a different intermediary. |
| `NoRoute` | Open a direct channel, or add channels to intermediate nodes. |
| `AssetUnsupported` | Confirm the destination accepts this asset. |
| `ChannelNotReady` | Wait for funding confirmation. |
| `Timeout` | Retry with a longer TLC expiry delta. |
| `InvalidInput` | Check destination pubkey, amount, or asset for typos. |
| `Unknown` | Inspect the raw message and check node logs. |

## Architecture

Cargo workspace, `fiber-probe-core` (lib) + `fiber-probe` (bin). Core exposes: async `RpcClient` (reqwest + tokio); pure `preflight::analyze` and `classifier::classify`; wire types (`NodeInfo`, `Channel`, `PaymentStatus`, `NodeSummary`); envelope + error types (`RpcRequest<P>`, `RpcResponse<R>`, `Error`).

Design: I/O lives in `RpcClient`; reasoning is pure. Consumers can test preflight and classification without a runtime or a network.

```bash
cargo test -p fiber-probe-core                              # 22 unit tests
cargo test -p fiber-probe-core --lib -- --ignored --nocapture  # live tests against your Fiber node
```

## Roadmap and license

Roadmap through Phase 2 (Observability Layer) and Phase 3 (LSP): see [ROADMAP.md](ROADMAP.md).

Dual-licensed under MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

