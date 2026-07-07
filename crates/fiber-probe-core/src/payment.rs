use serde::{Deserialize, Serialize};

/// Status of a payment previously initiated via `send_payment`.
///
/// Only routed-but-later-completed payments have a session Fiber will look up;
/// router-rejected sends error synchronously without creating a session.
#[derive(Deserialize, Serialize, Debug)]
pub struct PaymentStatus {
    pub payment_hash: String,
    /// One of: "Created", "Inflight", "Success", "Failed".
    pub status: String,
    /// Populated when `status == "Failed"`; the raw Fiber error message.
    pub failed_error: Option<String>,
    /// Routing fee actually paid, in shannons.
    #[serde(deserialize_with = "crate::hex::deserialize_hex_u64")]
    pub fee: u64,
}
