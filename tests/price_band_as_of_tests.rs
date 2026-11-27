//! `PriceBand::as_of` freshness stamp (tantra-risk gap 0006, `StaleReferenceData`).

use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use tantra_core::{Price, PriceBand};

fn px(v: i64) -> Price {
    Price::try_new(Decimal::from(v)).expect("price")
}

fn band() -> PriceBand {
    PriceBand::try_new(px(90), px(110), px(100)).expect("band")
}

#[test]
fn as_of_defaults_to_none() {
    assert_eq!(band().as_of(), None);
}

#[test]
fn with_as_of_stamps_the_band() {
    let ts = Utc.with_ymd_and_hms(2026, 11, 27, 3, 45, 0).unwrap();
    let b = band().with_as_of(ts);
    assert_eq!(b.as_of(), Some(ts));
    assert_eq!(b.reference_price(), band().reference_price());
}

#[test]
fn as_of_round_trips_and_is_omitted_when_absent() {
    let unstamped = serde_json::to_value(band()).unwrap();
    assert!(
        unstamped.get("as_of").is_none(),
        "absent as_of must not serialize"
    );

    let ts = Utc.with_ymd_and_hms(2026, 11, 27, 3, 45, 0).unwrap();
    let stamped = band().with_as_of(ts);
    let json = serde_json::to_string(&stamped).unwrap();
    let back: PriceBand = serde_json::from_str(&json).unwrap();
    assert_eq!(back, stamped);
}

#[test]
fn legacy_payload_without_as_of_deserializes() {
    let raw = r#"{"lower":"90","upper":"110","reference_price":"100"}"#;
    let b: PriceBand = serde_json::from_str(raw).unwrap();
    assert_eq!(b.as_of(), None);
}
