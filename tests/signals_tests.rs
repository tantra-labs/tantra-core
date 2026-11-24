//! Unit tests for `tantra_core::signals` (Plan 0003, Phase 4).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use tantra_core::{
    CoreError, Exchange, InstrumentId, LevelId, Money, OrderSide, PositionIntent, Quantity, Signal,
    SignalDirection, SignalHorizon, SignalKind, StrategySignal, Timeframe,
};

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn ts() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-01-15T14:35:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn instrument() -> InstrumentId {
    InstrumentId::try_new(Exchange::Nse, "RELIANCE").unwrap()
}

fn signal() -> StrategySignal {
    StrategySignal::new(
        OrderSide::Buy,
        SignalKind::Breakout,
        vec![LevelId(3), LevelId(8)],
        dec("90"),
        instrument(),
        ts(),
    )
    .unwrap()
}

#[test]
fn strategy_signal_rejects_empty_ref_levels() {
    let err = StrategySignal::new(
        OrderSide::Buy,
        SignalKind::Retest,
        vec![],
        dec("50"),
        instrument(),
        ts(),
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn strategy_signal_rejects_confidence_out_of_range() {
    assert!(
        StrategySignal::new(
            OrderSide::Buy,
            SignalKind::Breakout,
            vec![LevelId(1)],
            dec("-1"),
            instrument(),
            ts(),
        )
        .is_err()
    );
    assert!(
        StrategySignal::new(
            OrderSide::Buy,
            SignalKind::Breakout,
            vec![LevelId(1)],
            dec("101"),
            instrument(),
            ts(),
        )
        .is_err()
    );
    // Boundary values are legal.
    assert!(
        StrategySignal::new(
            OrderSide::Buy,
            SignalKind::Breakout,
            vec![LevelId(1)],
            dec("0"),
            instrument(),
            ts(),
        )
        .is_ok()
    );
    assert!(
        StrategySignal::new(
            OrderSide::Buy,
            SignalKind::Breakout,
            vec![LevelId(1)],
            dec("100"),
            instrument(),
            ts(),
        )
        .is_ok()
    );
}

#[test]
fn strategy_signal_to_signal_buy_maps_fields() {
    let s = signal();
    let out: Signal = s.clone().into();
    assert_eq!(out.instrument, instrument());
    assert_eq!(out.direction, SignalDirection::Bullish);
    assert_eq!(out.horizon, SignalHorizon::Intraday);
    assert_eq!(out.timeframe, Timeframe::M1);
    assert_eq!(out.strength, dec("0.90"));
    assert_eq!(out.generated_by, "elephant_edge");
    assert_eq!(out.timestamp, ts());
    assert_eq!(out.entry_price, None);
    assert_eq!(out.stop_loss, None);
    assert_eq!(out.take_profit, None);
}

#[test]
fn strategy_signal_to_signal_sell_is_bearish() {
    let s = StrategySignal::new(
        OrderSide::Sell,
        SignalKind::Reversal,
        vec![LevelId(3)],
        dec("75"),
        instrument(),
        ts(),
    )
    .unwrap();
    let out: Signal = s.into();
    assert_eq!(out.direction, SignalDirection::Bearish);
    assert_eq!(out.strength, dec("0.75"));
}

#[test]
fn strategy_signal_serde_roundtrip() {
    let s = signal();
    let json = serde_json::to_string(&s).unwrap();
    let decoded: StrategySignal = serde_json::from_str(&json).unwrap();
    assert_eq!(s, decoded);
}

#[test]
fn position_intent_requires_positive_quantity() {
    let err = PositionIntent::new(
        instrument(),
        OrderSide::Buy,
        Quantity::from_u64(0),
        Money::inr(dec("50000")),
        dec("0.25"),
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn position_intent_rejects_out_of_range_max_fraction() {
    assert!(
        PositionIntent::new(
            instrument(),
            OrderSide::Buy,
            Quantity::from_u64(10),
            Money::inr(dec("50000")),
            dec("-0.1"),
        )
        .is_err()
    );
    assert!(
        PositionIntent::new(
            instrument(),
            OrderSide::Buy,
            Quantity::from_u64(10),
            Money::inr(dec("50000")),
            dec("1.1"),
        )
        .is_err()
    );
    // Boundary values are legal.
    assert!(
        PositionIntent::new(
            instrument(),
            OrderSide::Buy,
            Quantity::from_u64(10),
            Money::inr(dec("50000")),
            dec("0.0"),
        )
        .is_ok()
    );
    assert!(
        PositionIntent::new(
            instrument(),
            OrderSide::Buy,
            Quantity::from_u64(10),
            Money::inr(dec("50000")),
            dec("1.0"),
        )
        .is_ok()
    );
}

#[test]
fn position_intent_accepts_valid_intent() {
    let intent = PositionIntent::new(
        instrument(),
        OrderSide::Sell,
        Quantity::from_u64(125),
        Money::inr(dec("50000")),
        dec("0.25"),
    )
    .unwrap();
    assert_eq!(intent.quantity, Quantity::from_u64(125));
    assert_eq!(intent.side, OrderSide::Sell);
    assert_eq!(intent.risk_cap, Money::inr(dec("50000")));
    assert_eq!(intent.max_fraction, dec("0.25"));
}

#[test]
fn position_intent_serde_roundtrip() {
    let intent = PositionIntent::new(
        instrument(),
        OrderSide::Buy,
        Quantity::from_u64(125),
        Money::inr(dec("50000")),
        dec("0.25"),
    )
    .unwrap();
    let json = serde_json::to_string(&intent).unwrap();
    let decoded: PositionIntent = serde_json::from_str(&json).unwrap();
    assert_eq!(intent, decoded);
}
