# Fiber Probe Roadmap

Fiber Probe is Phase 1 of a three-phase infrastructure project. Each phase builds directly on the previous one.

## Phase 1: Hackathon MVP (this repo)

**Timeline:** July 2026
**Delivered:**

- `fiber-probe` CLI with three commands (`check`, `diagnose`, `status`) plus global `--json`
- `fiber-probe-core` library, embeddable by wallets and dashboards
- Pre-flight analyzer with 5-variant failure taxonomy and 6 unit tests
- Failure classifier with 8-category taxonomy and 8 unit tests, tuned against real Fiber testnet errors
- SDK integration example (`wallet_gate`) that runs without a Fiber node
- Async JSON-RPC client covering `node_info`, `list_channels`, `get_payment`, `summary`

## Phase 2: Observability Layer

**Timeline:** August to October 2026

Build on `fiber-probe-core` to expand into a continuous monitoring service:

- Peer-liveness cross-check in preflight (real bug caught during Phase 1 testing: a `ChannelReady` channel to an offline peer still routes as `LikelySuccess`)
- Multi-hop preflight using Fiber's `build_router` and `graph_channels` RPCs
- Time-series storage of channel health, payment success rate, and fee dynamics
- Alerting for unhealthy nodes, weak routes, and peer disconnects
- Web dashboard consuming `fiber-probe-core` primitives
- Multi-node aggregation for operators running node fleets

## Phase 3: Liquidity Service Provider (LSP)

**Timeline:** November 2026 onwards

Use the observability data as the decision engine for a Fiber LSP:

- Just-in-time channel opening based on observed liquidity demand
- Capital allocation across channels tuned by Phase 2 telemetry
- Wallet SDK for channel-request flows
- UDT-specific (RUSD, USDC-Fiber) LSP support tied to `Asset` type extension

## Design continuity

Each phase reuses the Phase 1 SDK. `preflight::analyze`, `classifier::classify`, and the `RpcClient` primitives are the foundation. Phase 2 adds a monitoring loop around them; Phase 3 adds an LSP decision layer on top of Phase 2's telemetry. No pivots, just depth.
