use crate::channel::{Channel, ListChannelsResult};
use crate::error::Result;
use crate::node_info::NodeInfo;
use crate::rpc::{Payload, RpcRequest, RpcResponse};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct RpcClient {
    http: reqwest::Client,
    base_url: String,
    next_id: AtomicU64,
}
impl RpcClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            next_id: AtomicU64::new(1),
        }
    }
    pub async fn node_info(&self) -> Result<NodeInfo> {
        // increment the id counter and get a fresh id for this request
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // build a req envelope
        let req = RpcRequest {
            jsonrpc: "2.0",
            id,
            method: "node_info",
            params: (),
        };
        // post it as JSON and receive the raw response as bytes
        let bytes = self
            .http
            .post(&self.base_url)
            .json(&req)
            .send()
            .await?
            .bytes()
            .await?;
        //parse the bytes as RpcResponse<NodeInfo>
        let parsed_bytes = serde_json::from_slice::<RpcResponse<NodeInfo>>(&bytes)?;
        match parsed_bytes.payload {
            Payload::Result { result } => Ok(result),
            Payload::Error { error } => Err(error.into()),
        }
    }
    pub async fn list_channels(&self) -> Result<Vec<Channel>> {
        #[derive(serde::Serialize)]
        struct Params {
            #[serde(skip_serializing_if = "Option::is_none")]
            peer_id: Option<String>,
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = RpcRequest {
            jsonrpc: "2.0",
            id,
            method: "list_channels",
            params: (Params { peer_id: None },),
        };
        let bytes = self
            .http
            .post(&self.base_url)
            .json(&req)
            .send()
            .await?
            .bytes()
            .await?;
        let parsed_bytes = serde_json::from_slice::<RpcResponse<ListChannelsResult>>(&bytes)?;
        match parsed_bytes.payload {
            Payload::Result { result } => Ok(result.channels),
            Payload::Error { error } => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod live {
    use super::*;

    // This test hits Node A on 127.0.0.1:8227. Run with:
    //     cargo test -p fiber-probe-core -- --ignored
    #[tokio::test]
    #[ignore]
    async fn calls_real_node_info() {
        let client = RpcClient::new("http://127.0.0.1:8227");
        let info = client.node_info().await.expect("node_info should succeed");

        // print so you can eyeball the real values on a passing run
        println!("{:#?}", info);

        // sanity assertions matching what your curl output showed
        assert_eq!(info.pubkey.len(), 66); // 33 bytes hex-encoded
        assert!(info.peers_count >= 1); // A is connected to bootnodes at minimum
        assert!(info.version.starts_with("0.9."));
    }
    #[tokio::test]
    #[ignore]
    async fn calls_real_list_channels() {
        let client = RpcClient::new("http://127.0.0.1:8227");
        let channels = client
            .list_channels()
            .await
            .expect("list_channels should succeed");
        println!("got {} channels:", channels.len());
        for c in &channels {
            println!("{:#?}", c);
        }
        assert!(
            !channels.is_empty(),
            "Node A should have at least the A→C channel"
        );
    }
}
