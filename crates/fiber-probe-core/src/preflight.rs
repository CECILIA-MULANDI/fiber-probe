use serde::Serialize;

use crate::channel::Channel;
/// The asset being sent
///
///  This is L1- discrimination : CKB vs any UDT
/// Level-2 (specific UDT identity via UdtId + Script matching) will be in Phase 2 - added when we can
/// verify against a real UDT channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asset {
    Ckb,
    Udt,
}

/// The verdict of a preflight check.
#[derive(Debug, Serialize)]
pub enum PreflightResult {
    /// A route exists and looks capable of carrying the payment.
    LikelySuccess,
    /// The payment is likely to fail; `FailReason` explains why.
    LikelyFail(FailReason),
    /// We can't decide with the information available (e.g. multi-hop
    /// routing needed but not yet implemented).
    Unknown(String),
}

/// The specific reason a payment is expected to fail.
///
/// Every variant carries the *why* so the CLI can print actionable text
/// like "insufficient liquidity: 100 CKB available, 500 CKB required"
/// instead of a generic error.
#[derive(Debug, Serialize)]
pub enum FailReason {
    /// No channel to the destination peer exists at all.
    NoDirectChannel,
    /// Channels to the peer exist, but none carry the requested asset.
    AssetMismatch,
    /// A matching-asset channel exists but isn't in `ChannelReady` state.
    ChannelNotReady { state: String },
    /// A ready channel exists but is flagged disabled by gossip.
    ChannelDisabled,
    /// A ready + enabled channel exists but doesn't have enough send-side
    /// balance. `available` is the max we saw across candidate channels.
    InsufficientLiquidity { available: u64, required: u64 },
}
/// Given a snapshot of the caller's channels, decide whether the payment
/// `(to, amount, asset)` is likely to route through direct channels only.
/// I will implement multi-hop routing is Phase 2.
pub fn analyze(channels: &[Channel], to: &str, amount: u64, asset: Asset) -> PreflightResult {
    // filter to channels whose peer pubkey matches `to`.
    // If none → return LikelyFail(NoDirectChannel).
    let peer_channels: Vec<&Channel> = channels.iter().filter(|c| c.pubkey == to).collect();
    if peer_channels.is_empty() {
        return PreflightResult::LikelyFail(FailReason::NoDirectChannel);
    }

    // keep only channels whose asset matches the requested one.
    let matching_asset: Vec<&Channel> = peer_channels
        .iter()
        .copied()
        .filter(|c| match asset {
            Asset::Ckb => c.funding_udt_type_script.is_none(),
            Asset::Udt => c.funding_udt_type_script.is_some(),
        })
        .collect();
    if matching_asset.is_empty() {
        return PreflightResult::LikelyFail(FailReason::AssetMismatch);
    }

    // keep only channels in state "ChannelReady".
    let ready: Vec<&Channel> = matching_asset
        .iter()
        .copied()
        .filter(|c| c.state.state_name == "ChannelReady")
        .collect();
    if ready.is_empty() {
        return PreflightResult::LikelyFail(FailReason::ChannelNotReady {
            state: matching_asset[0].state.state_name.clone(),
        });
    }

    // keep only enabled channels.
    let enabled: Vec<&Channel> = ready.iter().copied().filter(|c| c.enabled).collect();
    if enabled.is_empty() {
        return PreflightResult::LikelyFail(FailReason::ChannelDisabled);
    }

    //  find the fattest candidate and compare against amount.
    let available = enabled
        .iter()
        .map(|c| c.local_balance)
        .max()
        .expect("enabled is non-empty");
    if available >= amount {
        PreflightResult::LikelySuccess
    } else {
        PreflightResult::LikelyFail(FailReason::InsufficientLiquidity {
            available,
            required: amount,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{Channel, ChannelState};

    /// Small helper — Channel has too many fields to inline in every test.
    fn ch(pubkey: &str, state: &str, enabled: bool, local_balance: u64, is_udt: bool) -> Channel {
        Channel {
            channel_id: "0xtest".into(),
            pubkey: pubkey.into(),
            state: ChannelState {
                state_name: state.into(),
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

    #[test]
    fn no_channels_at_all_gives_no_direct_channel() {
        let result = analyze(&[], "0xC", 100, Asset::Ckb);
        assert!(matches!(
            result,
            PreflightResult::LikelyFail(FailReason::NoDirectChannel)
        ));
    }

    #[test]
    fn only_udt_channels_but_asked_for_ckb_gives_asset_mismatch() {
        let channels = vec![ch("0xC", "ChannelReady", true, 500, /* is_udt */ true)];
        let result = analyze(&channels, "0xC", 100, Asset::Ckb);
        assert!(matches!(
            result,
            PreflightResult::LikelyFail(FailReason::AssetMismatch)
        ));
    }

    #[test]
    fn channel_still_negotiating_gives_channel_not_ready() {
        let channels = vec![ch("0xC", "NegotiatingFunding", true, 500, false)];
        let result = analyze(&channels, "0xC", 100, Asset::Ckb);
        match result {
            PreflightResult::LikelyFail(FailReason::ChannelNotReady { state }) => {
                assert_eq!(state, "NegotiatingFunding");
            }
            other => panic!("expected ChannelNotReady, got {other:?}"),
        }
    }

    #[test]
    fn ready_but_disabled_gives_channel_disabled() {
        let channels = vec![ch("0xC", "ChannelReady", false, 500, false)];
        let result = analyze(&channels, "0xC", 100, Asset::Ckb);
        assert!(matches!(
            result,
            PreflightResult::LikelyFail(FailReason::ChannelDisabled)
        ));
    }

    #[test]
    fn ready_enabled_but_underfunded_reports_gap() {
        let channels = vec![ch("0xC", "ChannelReady", true, 100, false)];
        let result = analyze(&channels, "0xC", 500, Asset::Ckb);
        match result {
            PreflightResult::LikelyFail(FailReason::InsufficientLiquidity {
                available,
                required,
            }) => {
                assert_eq!(available, 100);
                assert_eq!(required, 500);
            }
            other => panic!("expected InsufficientLiquidity, got {other:?}"),
        }
    }

    #[test]
    fn ready_enabled_and_funded_gives_likely_success() {
        let channels = vec![ch("0xC", "ChannelReady", true, 500, false)];
        let result = analyze(&channels, "0xC", 100, Asset::Ckb);
        assert!(matches!(result, PreflightResult::LikelySuccess));
    }
}
