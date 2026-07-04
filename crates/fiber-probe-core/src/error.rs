use crate::rpc::RpcError;
/// Possible ways an RPC call can fail
///
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP req itself failed, so server is unreachable/ connection dropped/
    /// TLS error, etc.
    /// Wraps the raw reqwest error
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Parse error, maybe when server returns bytes that don't serialize
    /// into the type we asked for
    #[error("failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),
    /// RPC error - when server returns a valid JSON-RPC error envelope
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcError),
}

pub type Result<T> = std::result::Result<T, Error>;
