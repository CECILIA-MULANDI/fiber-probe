use serde::{Deserialize, Deserializer};
/// serde deserializer for hex-encoded u64 values like "0x1", "0x2540be400".
/// Used via `#[serde(deserialize_with = "crate::hex::deserialize_hex_u64")]` on any u64 field.

pub fn deserialize_hex_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    // Read the JSON value as an owned String
    let s = String::deserialize(deserializer)?;

    // Strip the "0x" prefix
    // If missing return a serde error
    let hex = s
        .strip_prefix("0x")
        .ok_or_else(|| serde::de::Error::custom("expected hex string starting with '0x'"))?;

    // Parse the hex as u64
    u64::from_str_radix(hex, 16).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    // A small wrapper to test `deserialize_hex_u64` in isolation
    #[derive(Deserialize, Debug)]
    #[serde(transparent)]
    struct Wrap {
        #[serde(deserialize_with = "deserialize_hex_u64")]
        value: u64,
    }
    #[test]
    fn parse_hex_string() {
        // 0x2540be400 == 10_000_000_000 (the min funding amount from your NodeInfo output)

        let w: Wrap = serde_json::from_str(r#""0x2540be400""#).expect("should parse");
        assert_eq!(w.value, 10_000_000_000);
    }
    #[test]
    fn errors_on_missing_prefix() {
        let result: Result<Wrap, _> = serde_json::from_str(r#""2540""#);
        assert!(result.is_err(), "should reject strings without 0x prefix");
    }
}
