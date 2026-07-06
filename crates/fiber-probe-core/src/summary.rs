use serde::Serialize;

use crate::channel::Channel;
use crate::node_info::NodeInfo;

/// A snapshot of the local node — identity, health, and every channel it holds.
///
/// This is what the CLI's `status` command formats for terminal or `--json`
/// output. Fetched atomically-ish (two RPC calls, back-to-back) so the numbers
/// are close in time but not strictly a single snapshot.
#[derive(Debug, Serialize)]
pub struct NodeSummary {
    pub node: NodeInfo,
    pub channels: Vec<Channel>,
}

impl NodeSummary {
    /// Total local (send-side) balance in shannons across all channels
    /// carrying the native CKB asset. Ignores UDT channels.
    pub fn total_ckb_local_balance(&self) -> u64 {
        self.channels
            .iter()
            .filter(|c| c.funding_udt_type_script.is_none())
            .map(|c| c.local_balance)
            .sum()
    }

    /// Count of channels that are ChannelReady + enabled (i.e. routable now).
    pub fn routable_channel_count(&self) -> usize {
        self.channels
            .iter()
            .filter(|c| c.state.state_name == "ChannelReady" && c.enabled)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::ChannelState;

    fn ch(state: &str, enabled: bool, local_balance: u64, is_udt: bool) -> Channel {
        Channel {
            channel_id: "0xtest".into(),
            pubkey: "0xC".into(),
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

    fn empty_node() -> NodeInfo {
        NodeInfo {
            version: "test".into(),
            pubkey: "0xA".into(),
            node_name: None,
            peers_count: 0,
            channel_count: 0,
            pending_channel_count: 0,
            chain_hash: "0x0".into(),
        }
    }

    #[test]
    fn ckb_balance_sums_only_ckb_channels() {
        let summary = NodeSummary {
            node: empty_node(),
            channels: vec![
                ch("ChannelReady", true, 100, false), // CKB
                ch("ChannelReady", true, 500, true),  // UDT — should be skipped
                ch("ChannelReady", true, 200, false), // CKB
            ],
        };
        assert_eq!(summary.total_ckb_local_balance(), 300);
    }

    #[test]
    fn routable_count_requires_ready_and_enabled() {
        let summary = NodeSummary {
            node: empty_node(),
            channels: vec![
                ch("ChannelReady", true, 100, false),          // yes
                ch("ChannelReady", false, 100, false),         // no (disabled)
                ch("NegotiatingFunding", true, 100, false),    // no (not ready)
                ch("ChannelReady", true, 100, true),           // yes (UDT still routable)
            ],
        };
        assert_eq!(summary.routable_channel_count(), 2);
    }
}
