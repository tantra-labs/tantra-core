//! TDD tests for `tantra_core::classification` value objects (Plan 0004 Phase A).

use std::str::FromStr;

use schemars::schema_for;
use tantra_core::classification::{
    IndexId, Industry, MAX_CLASSIFICATION_LEN, MAX_INDEX_ID_LEN, MarketCapBucket, Sector,
};
use tantra_core::error::CoreError;

#[test]
fn sector_trims_normalizes_whitespace_and_roundtrips_json() {
    let s = Sector::try_new("  Financial   Services \t").unwrap();
    assert_eq!(s.as_str(), "Financial Services");
    assert_eq!(s.to_string(), "Financial Services");

    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(json, "\"Financial Services\"");
    let decoded: Sector = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, s);
}

#[test]
fn sector_rejects_empty_control_and_oversized_inputs() {
    for bad in ["", "   ", "\t\n", "Finance\x07Services", "IT\0"] {
        assert!(
            matches!(Sector::try_new(bad), Err(CoreError::Validation(_))),
            "expected Validation error for {bad:?}"
        );
    }

    let too_long = "A".repeat(MAX_CLASSIFICATION_LEN + 1);
    assert!(matches!(
        Sector::try_new(&too_long),
        Err(CoreError::Validation(_))
    ));

    let at_limit = "A".repeat(MAX_CLASSIFICATION_LEN);
    assert!(Sector::try_new(&at_limit).is_ok());

    // Deserialization must enforce the exact same invariants at the wire boundary.
    assert!(serde_json::from_str::<Sector>("\"   \"").is_err());
    assert!(serde_json::from_str::<Sector>(&format!("\"{too_long}\"")).is_err());
}

#[test]
fn industry_validates_and_roundtrips() {
    let ind = Industry::try_new("  IT -   Software ").unwrap();
    assert_eq!(ind.as_str(), "IT - Software");
    assert_eq!(Industry::from_str("IT - Software").unwrap(), ind);

    let json = serde_json::to_string(&ind).unwrap();
    assert_eq!(json, "\"IT - Software\"");
    let decoded: Industry = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, ind);

    assert!(matches!(
        Industry::try_new(""),
        Err(CoreError::Validation(_))
    ));
    assert!(serde_json::from_str::<Industry>("\"\"").is_err());
    assert!(serde_json::from_str::<Industry>("\"Bad\\u0000Ind\"").is_err());
}

#[test]
fn index_id_uppercases_and_validates_ascii_identifier() {
    let nifty = IndexId::try_new("  nifty500 ").unwrap();
    assert_eq!(nifty.as_str(), "NIFTY500");
    assert_eq!(nifty.to_string(), "NIFTY500");

    let midcap = IndexId::try_new("nifty_midcap-150").unwrap();
    assert_eq!(midcap.as_str(), "NIFTY_MIDCAP-150");

    for canonical in ["NIFTY50", "NIFTY500", "SENSEX", "SP500"] {
        let parsed = IndexId::from_str(canonical).unwrap();
        let json = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json, format!("\"{canonical}\""));
        let decoded: IndexId = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, parsed);
    }

    for bad in [
        "",
        "   ",
        "NIFTY 50",
        "NIFTY:50",
        "NIFTY/50",
        "S&P500",
        "निफ्टी50",
    ] {
        assert!(
            matches!(IndexId::try_new(bad), Err(CoreError::Validation(_))),
            "expected Validation error for {bad:?}"
        );
        assert!(
            serde_json::from_value::<IndexId>(serde_json::json!(bad)).is_err(),
            "serde must reject invalid IndexId {bad:?}"
        );
    }

    let too_long = "X".repeat(MAX_INDEX_ID_LEN + 1);
    assert!(IndexId::try_new(&too_long).is_err());
}

#[test]
fn market_cap_bucket_wire_format_is_bare_snake_case() {
    let pairs = [
        (MarketCapBucket::Large, "large"),
        (MarketCapBucket::Mid, "mid"),
        (MarketCapBucket::Small, "small"),
        (MarketCapBucket::Micro, "micro"),
    ];
    for (bucket, wire) in pairs {
        assert_eq!(bucket.as_str(), wire);
        assert_eq!(bucket.to_string(), wire);
        assert_eq!(MarketCapBucket::from_str(wire).unwrap(), bucket);
        assert_eq!(
            MarketCapBucket::from_str(&wire.to_uppercase()).unwrap(),
            bucket
        );
        let json = serde_json::to_string(&bucket).unwrap();
        assert_eq!(json, format!("\"{wire}\""));
        let decoded: MarketCapBucket = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, bucket);
    }

    assert!(MarketCapBucket::from_str("mega").is_err());
    assert!(serde_json::from_str::<MarketCapBucket>("\"mega\"").is_err());
}

#[test]
fn classification_types_generate_json_schemas() {
    let _ = schema_for!(Sector);
    let _ = schema_for!(Industry);
    let _ = schema_for!(IndexId);
    let _ = schema_for!(MarketCapBucket);
}
