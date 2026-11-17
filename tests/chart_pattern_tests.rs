//! Unit tests for `tantra_core::chart_pattern` (Plan 0005, P1).

use chrono::{Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use tantra_core::{
    ChartPattern, CoreError, PatternKind, PatternPivot, PatternStage, Price, SignalDirection,
    Timeframe,
};

fn ts() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000, 0).unwrap()
}

fn price(s: &str) -> Price {
    Price::from_rupees(s).unwrap()
}

fn bull_pattern() -> ChartPattern {
    ChartPattern::new(
        PatternKind::CupAndHandle,
        SignalDirection::Bullish,
        Timeframe::H1,
        PatternStage::FormingArmed,
        vec![],
        price("1000.00"),
        price("960.00"),
        Some(price("1080.00")),
        90,
        ts(),
        ts(),
        None,
    )
    .unwrap()
}

#[test]
fn pattern_accepts_valid_bullish_geometry() {
    let pattern = bull_pattern();
    assert_eq!(pattern.kind, PatternKind::CupAndHandle);
    assert_eq!(pattern.direction, SignalDirection::Bullish);
    assert_eq!(pattern.stage, PatternStage::FormingArmed);
    assert_eq!(pattern.quality, 90);
    assert_eq!(pattern.measured_height(), dec("40.00"));
}

#[test]
fn pattern_rejects_invalidation_on_winning_side() {
    // Bullish invalidation must sit below the breakout; 1010 is above it.
    let err = ChartPattern::new(
        PatternKind::Flag,
        SignalDirection::Bullish,
        Timeframe::M5,
        PatternStage::FormingArmed,
        vec![],
        price("1000.00"),
        price("1010.00"),
        None,
        80,
        ts(),
        ts(),
        None,
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn pattern_rejects_measured_target_on_losing_side() {
    // Bullish measured target must sit above the breakout; 990 is below it.
    let err = ChartPattern::new(
        PatternKind::DoubleBottom,
        SignalDirection::Bullish,
        Timeframe::D1,
        PatternStage::BreakoutConfirmed,
        vec![],
        price("1000.00"),
        price("960.00"),
        Some(price("990.00")),
        70,
        ts(),
        ts(),
        Some(ts()),
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn pattern_rejects_formed_after_armed() {
    let armed = ts();
    let formed = armed + Duration::hours(1);
    let err = ChartPattern::new(
        PatternKind::Vcp,
        SignalDirection::Bullish,
        Timeframe::H4,
        PatternStage::FormingArmed,
        vec![],
        price("1000.00"),
        price("960.00"),
        None,
        60,
        formed,
        armed,
        None,
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn pattern_rejects_quality_above_100() {
    let err = ChartPattern::new(
        PatternKind::Wedge,
        SignalDirection::Bearish,
        Timeframe::M15,
        PatternStage::FormingArmed,
        vec![],
        price("1000.00"),
        price("1040.00"),
        Some(price("940.00")),
        101,
        ts(),
        ts(),
        None,
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn pattern_accepts_bearish_geometry() {
    let pattern = ChartPattern::new(
        PatternKind::HeadAndShoulders,
        SignalDirection::Bearish,
        Timeframe::H1,
        PatternStage::BreakoutConfirmed,
        vec![PatternPivot {
            bar_ts: ts(),
            price: price("1000.00"),
            kind: "high".to_string(),
        }],
        price("960.00"),
        price("1000.00"),
        Some(price("900.00")),
        85,
        ts(),
        ts(),
        Some(ts()),
    )
    .unwrap();
    assert_eq!(pattern.direction, SignalDirection::Bearish);
    assert!(pattern.stage.is_confirmed());
    assert_eq!(pattern.measured_height(), dec("40.00"));
}

#[test]
fn pattern_neutral_direction_skips_geometry_checks() {
    let pattern = ChartPattern::new(
        PatternKind::Rectangle,
        SignalDirection::Neutral,
        Timeframe::D1,
        PatternStage::FormingArmed,
        vec![],
        price("1000.00"),
        price("1010.00"),
        None,
        50,
        ts(),
        ts(),
        None,
    )
    .unwrap();
    assert_eq!(pattern.direction, SignalDirection::Neutral);
}

#[test]
fn pattern_serde_roundtrip() {
    let pattern = bull_pattern();
    let json = serde_json::to_string(&pattern).unwrap();
    let decoded: ChartPattern = serde_json::from_str(&json).unwrap();
    assert_eq!(pattern, decoded);
    // The optional measured target round-trips too.
    assert_eq!(decoded.measured_target, Some(price("1080.00")));
}

#[test]
fn pattern_stage_transitions() {
    assert_eq!(
        PatternStage::FormingArmed.confirm(),
        PatternStage::BreakoutConfirmed
    );
    assert_eq!(
        PatternStage::BreakoutConfirmed.confirm(),
        PatternStage::BreakoutConfirmed
    );
    assert!(!PatternStage::FormingArmed.is_confirmed());
    assert!(PatternStage::BreakoutConfirmed.is_confirmed());
}

#[test]
fn pattern_kinds_serde_snake_case() {
    let json = r#""inverse_head_and_shoulders""#;
    let kind: PatternKind = serde_json::from_str(json).unwrap();
    assert_eq!(kind, PatternKind::InverseHeadAndShoulders);
    let back = serde_json::to_string(&kind).unwrap();
    assert_eq!(back, json);
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}
