use serde::{Deserialize, Serialize};
#[derive(Serialize, Debug)]
struct RpcRequest<P> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: P,
}
#[derive(Deserialize, Debug)]
struct RpcError {
    code: i64,
    message: String,
    data: Option<serde_json::Value>,
}
#[derive(Deserialize, Debug)]
struct RpcResponse<R> {
    jsonrpc: String,
    id: u64,
    #[serde(flatten)]
    payload: Payload<R>,
}
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Payload<R> {
    Result { result: R },
    Error { error: RpcError },
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct FakeResult {
        node_name: String,
    }
    #[test]
    fn deserializes_success_response() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "node_name": "node-a"
            }
        }"#;
        //parse it as an RpcResponse where res = FakeResult
        let parsed: RpcResponse<FakeResult> =
            serde_json::from_str(json).expect("valid success message should deserialize");
        match parsed.payload {
            Payload::Result { result } => {
                assert_eq!(result.node_name, "node-a");
            }
            Payload::Error { error } => {
                panic!("expected Result variant, got Error: {error:?}");
            }
        }
    }
    #[test]
    fn deserializes_error_response() {
        let json = r#"{
        "jsonrpc": "2.0",
        "id":1,
        "error":{
        "code": -32601,
        "message":"Method not found",
        "data":null
    }
        }"#;

        let parsed: RpcResponse<FakeResult> =
            serde_json::from_str(json).expect("valid error message should deserialize");
        match parsed.payload {
            Payload::Error { error } => {
                assert_eq!(error.code, -32601);
                assert_eq!(error.message, "Method not found");
            }
            Payload::Result { result } => {
                panic!("expected Error variant, got Result: {result:?}")
            }
        }
    }
}
