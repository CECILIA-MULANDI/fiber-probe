use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct NodeInfo {
    pub version: String,
    pub pubkey: String,
    pub node_name: Option<String>,
    #[serde(deserialize_with = "crate::hex::deserialize_hex_u64")]
    pub peers_count: u64,
    #[serde(deserialize_with = "crate::hex::deserialize_hex_u64")]
    pub channel_count: u64,
    #[serde(deserialize_with = "crate::hex::deserialize_hex_u64")]
    pub pending_channel_count: u64,
    pub chain_hash: String,
}
#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn deserialize_nodeinfo() {
        let json = r#"{
            "version": "0.9.0-rc7",
            "commit_hash": "bc361aa 2026-07-02",
            "pubkey": "02bae0aaaca9c4fc6e7c2f915b6fc25011828941f978f747b55108fa953231a5b1",
            "features": ["GOSSIP_QUERIES_REQUIRED", "BASIC_MPP_REQUIRED", "TRAMPOLINE_ROUTING_REQUIRED"],
            "node_name": null,
            "addresses": [],
            "chain_hash": "0x10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606",
            "open_channel_auto_accept_min_ckb_funding_amount": "0x2540be400",
            "auto_accept_channel_ckb_funding_amount": "0x24e160300",
            "default_funding_lock_script": {
                "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                "hash_type": "type",
                "args": "0x725373344aa3bcdb7f41f1a27d2cdd4c9d8b68f9"
            },
            "tlc_expiry_delta": "0xdbba00",
            "tlc_min_value": "0x0",
            "tlc_fee_proportional_millionths": "0x3e8",
            "channel_count": "0x1",
            "pending_channel_count": "0x0",
            "peers_count": "0x1"

        }"#;

        let info: NodeInfo = serde_json::from_str(json).expect("real response should parse");

        assert_eq!(
            info.pubkey,
            "02bae0aaaca9c4fc6e7c2f915b6fc25011828941f978f747b55108fa953231a5b1"
        );
        assert_eq!(info.node_name, None);
        assert_eq!(info.peers_count, 1);
    }
}
