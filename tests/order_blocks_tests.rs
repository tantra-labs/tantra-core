//! Unit tests for `tantra_core::order_blocks` (Plan 0003, Phase 3).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use tantra_core::{CoreError, LevelSide, OrderBlock};

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn ts() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-01-15T10:15:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn order_block_accepts_valid_zone() {
    let block =
        OrderBlock::new(ts(), LevelSide::Support, (dec("2462.00"), dec("2472.00"))).unwrap();
    assert_eq!(block.side, LevelSide::Support);
    assert!(block.zone.0 <= block.zone.1);
}

#[test]
fn order_block_accepts_zero_width_zone() {
    let block = OrderBlock::new(
        ts(),
        LevelSide::Resistance,
        (dec("2500.00"), dec("2500.00")),
    )
    .unwrap();
    assert_eq!(block.zone.0, block.zone.1);
}

#[test]
fn order_block_rejects_inverted_zone() {
    let err = OrderBlock::new(
        ts(),
        LevelSide::Resistance,
        (dec("2472.00"), dec("2462.00")),
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn order_block_serde_roundtrip() {
    let block =
        OrderBlock::new(ts(), LevelSide::Support, (dec("2462.00"), dec("2472.00"))).unwrap();
    let json = serde_json::to_string(&block).unwrap();
    let decoded: OrderBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(block, decoded);
}
