use serde::{Deserialize, Serialize};
/// A fiber payment channel as shown by 'list_channels' RPC
#[derive(Deserialize, Serialize, Debug)]
pub struct Channel {
    pub channel_id: String,
    pub pubkey: String,
    pub state: ChannelState,
    pub enabled: bool,
    #[serde(deserialize_with = "crate::hex::deserialize_hex_u64")]
    pub local_balance: u64,
    #[serde(deserialize_with = "crate::hex::deserialize_hex_u64")]
    pub remote_balance: u64,
    pub funding_udt_type_script: Option<serde_json::Value>,
}
/// Channel lifecycle state. Fiber's state machine has many state names
/// (NegotiatingFunding, AwaitingChannelReady, ChannelReady, ...); we treat
/// it as an opaque string and let preflight decide which ones are routable.
#[derive(Deserialize, Serialize, Debug)]
pub struct ChannelState {
    pub state_name: String,
}

/// Wraps the `list_channels` RPC result: the wire shape is
/// `{"channels": [...]}`, not a bare array.
#[derive(Deserialize, Debug)]
pub struct ListChannelsResult {
    pub channels: Vec<Channel>,
}