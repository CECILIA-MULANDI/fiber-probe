use serde::Serialize;

use crate::rpc::RpcError;

/// High-level failure category derived from a raw JSON-RPC error.
///
/// Used by `diagnose` to turn a wire-level error into an actionable
/// category the user can reason about. The mapping is heuristic — we
/// match on standard JSON-RPC codes plus keyword patterns in the
/// error message.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum FailureCategory {
    LiquidityShortfall,
    PeerOffline,
    NoRoute,
    AssetUnsupported,
    ChannelNotReady,
    Timeout,
    InvalidInput,
    Unknown,
}

/// A classified failure; the raw error kept alongside the bucket
/// and a suggested next action.
#[derive(Debug, Serialize)]
pub struct Classification {
    pub category: FailureCategory,
    pub suggested_fix: &'static str,
    pub raw_code: i64,
    pub raw_message: String,
}

/// Map a raw JSON-RPC error to a human-readable failure category.
pub fn classify(err: &RpcError) -> Classification {
    let category = category_from(err);
    let suggested_fix = suggested_fix(&category);
    Classification {
        category,
        suggested_fix,
        raw_code: err.code,
        raw_message: err.message.clone(),
    }
}

fn category_from(err: &RpcError) -> FailureCategory {
    // Standard JSON-RPC 2.0 codes we can bucket without reading the message.
    match err.code {
        -32700 | -32600 | -32601 | -32602 => return FailureCategory::InvalidInput,
        _ => {}
    }

    let msg = err.message.to_lowercase();

    // Keyword patterns, ordered specific → generic so the most
    // informative match wins.
    if msg.contains("insufficient")
        || msg.contains("not enough")
        || msg.contains("balance")
        || msg.contains("liquidity")
    {
        FailureCategory::LiquidityShortfall
    } else if msg.contains("offline") || msg.contains("unreachable") || msg.contains("disconnected")
    {
        FailureCategory::PeerOffline
    } else if msg.contains("no route")
        || msg.contains("no path")
        || msg.contains("unreachable destination")
    {
        FailureCategory::NoRoute
    } else if msg.contains("udt") || msg.contains("asset") {
        FailureCategory::AssetUnsupported
    } else if msg.contains("channel") && (msg.contains("not ready") || msg.contains("state")) {
        FailureCategory::ChannelNotReady
    } else if msg.contains("timeout") || msg.contains("expired") || msg.contains("timed out") {
        FailureCategory::Timeout
    } else {
        FailureCategory::Unknown
    }
}

fn suggested_fix(category: &FailureCategory) -> &'static str {
    match category {
        FailureCategory::LiquidityShortfall => {
            "Reduce the amount, open a larger channel, or rebalance the existing channel."
        }
        FailureCategory::PeerOffline => {
            "Wait for the peer to come back online, or route through a different intermediary."
        }
        FailureCategory::NoRoute => {
            "Open a direct channel to the destination or add channels to intermediate nodes."
        }
        FailureCategory::AssetUnsupported => {
            "Confirm the destination accepts this asset and that a channel exists for it."
        }
        FailureCategory::ChannelNotReady => {
            "Wait for the funding transaction to confirm; retry once the channel reports ChannelReady."
        }
        FailureCategory::Timeout => "Retry the payment; consider a longer TLC expiry delta.",
        FailureCategory::InvalidInput => {
            "Check the payment arguments (destination pubkey, amount, asset) for typos or bad encoding."
        }
        FailureCategory::Unknown => {
            "No pattern matched. Inspect the raw message and check node logs."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(code: i64, message: &str) -> RpcError {
        RpcError {
            code,
            message: message.to_string(),
            data: None,
        }
    }

    #[test]
    fn standard_jsonrpc_codes_become_invalid_input() {
        for code in [-32700, -32600, -32601, -32602] {
            let c = classify(&err(code, "anything"));
            assert_eq!(c.category, FailureCategory::InvalidInput, "code {code}");
        }
    }

    #[test]
    fn insufficient_liquidity_maps_correctly() {
        let c = classify(&err(-1, "insufficient balance for payment"));
        assert_eq!(c.category, FailureCategory::LiquidityShortfall);
    }

    #[test]
    fn peer_offline_maps_correctly() {
        let c = classify(&err(-1, "peer is offline"));
        assert_eq!(c.category, FailureCategory::PeerOffline);
    }

    #[test]
    fn no_route_maps_correctly() {
        let c = classify(&err(-1, "no route to destination"));
        assert_eq!(c.category, FailureCategory::NoRoute);
    }

    #[test]
    fn asset_mismatch_maps_correctly() {
        let c = classify(&err(-1, "unsupported UDT asset"));
        assert_eq!(c.category, FailureCategory::AssetUnsupported);
    }

    #[test]
    fn channel_state_maps_correctly() {
        let c = classify(&err(-1, "channel not ready for payments"));
        assert_eq!(c.category, FailureCategory::ChannelNotReady);
    }

    #[test]
    fn timeout_maps_correctly() {
        let c = classify(&err(-1, "TLC expired: timed out"));
        assert_eq!(c.category, FailureCategory::Timeout);
    }

    #[test]
    fn unknown_falls_through() {
        let c = classify(&err(-1, "something completely unrecognized"));
        assert_eq!(c.category, FailureCategory::Unknown);
    }
}
