//! Golden vectors for `tantra_core::trade_plan` (Plan 0005, P1).
//!
//! Fixtures live in `schema/golden/` and must stay byte-for-byte equal to the
//! deterministic output of `TradePlanner::plan` for the fixtures' inputs.

use std::fs;

use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use tantra_core::Price;
use tantra_core::{
    EntryRule, Exchange, InstrumentId, Level, LevelId, LevelKind, LevelMap, LevelSide, Money,
    OrderSide, PlanSubject, PlannerInputs, Quantity, RiskBudget, Signal, SignalDirection,
    SignalHorizon, SignalKind, SizingMethod, SizingSpec, StopRule, StrategySignal, TargetLadder,
    TargetLeg, TargetTrigger, Timeframe, TradePlan, TradePlanSpec, TradePlanner, TrailRule,
    TrailSpec,
};

fn ts() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000, 0).unwrap()
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn price(s: &str) -> Price {
    Price::from_rupees(s).unwrap()
}

fn instrument() -> InstrumentId {
    InstrumentId::new(Exchange::Nse, "RELIANCE")
}

fn budget() -> RiskBudget {
    RiskBudget::new(
        Money::inr(dec("100000.00")),
        dec("0.01"),
        Quantity::from_u64(1),
    )
}

fn base_spec(preset_id: &str) -> TradePlanSpec {
    TradePlanSpec::new(
        preset_id,
        1,
        EntryRule::Market,
        StopRule::Fixed(price("980.00")),
        TrailSpec {
            rule: TrailRule::Off,
            activation_after_r: Decimal::ONE,
            breakeven_after: None,
            time_stop: None,
        },
        TargetLadder::new(
            vec![TargetLeg {
                trigger: TargetTrigger::AtR(Decimal::from(2)),
                fraction: dec("0.5"),
            }],
            true,
        )
        .unwrap(),
        SizingSpec {
            method: SizingMethod::FixedFractional {
                risk_pct: dec("0.01"),
            },
            max_weight_pct: None,
            max_adv_pct: None,
        },
    )
}

fn base_inputs() -> PlannerInputs {
    PlannerInputs {
        subject: PlanSubject::Direct,
        instrument: instrument(),
        side: OrderSide::Buy,
        last_price: price("1000.00"),
        tick_size: price("0.05"),
        atr: Some(dec("10.00")),
        adr_pct: Some(dec("3.00")),
        swing_low: None,
        swing_high: None,
        day_low: None,
        day_high: None,
        ma_values: vec![],
        adv20: None,
        level_map: None,
        timestamp: ts(),
    }
}

#[test]
fn golden_long_signal_subject_plan() {
    let json = fs::read_to_string("schema/golden/trade_plan_long_signal_golden.json").unwrap();
    let fixture: Value = serde_json::from_str(&json).unwrap();

    // Plan from a `Signal`-subject (bullish) long.
    let signal = Signal::new(
        instrument(),
        SignalDirection::Bullish,
        SignalHorizon::Intraday,
        Timeframe::M1,
        dec("0.8"),
        "test_algo",
        ts(),
    );
    let mut inputs = base_inputs();
    inputs.subject = PlanSubject::Signal(signal);
    let plan: TradePlan = TradePlanner::plan(&base_spec("test_long"), &inputs, &budget()).unwrap();

    // The deterministic plan must match the committed vector exactly.
    let produced = serde_json::to_value(&plan).unwrap();
    assert_eq!(
        produced, fixture,
        "long Signal-subject plan drifted from its golden vector"
    );

    // Load-and-assert semantics, following the existing golden_test.rs pattern.
    let loaded: TradePlan = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, plan);
    assert_eq!(loaded.side, OrderSide::Buy);
    assert_eq!(loaded.targets[0].price, price("1040.00"));
    assert_eq!(loaded.reward_risk, Decimal::from(2));
}

#[test]
fn golden_short_strategy_signal_subject_plan() {
    let json =
        fs::read_to_string("schema/golden/trade_plan_short_strategy_signal_golden.json").unwrap();
    let fixture: Value = serde_json::from_str(&json).unwrap();

    // Plan from an EE+ `StrategySignal` (sell) short with a referenced level.
    let signal = StrategySignal::new(
        OrderSide::Sell,
        SignalKind::Breakout,
        vec![LevelId(1)],
        dec("80"),
        instrument(),
        ts(),
    )
    .unwrap();
    let level = Level::new(
        LevelId(1),
        LevelKind::ElephantResistance1,
        LevelSide::Resistance,
        (dec("1010.00"), dec("1020.00")),
        4,
    )
    .unwrap();
    let mut spec = base_spec("test_short");
    spec.stop = StopRule::Fixed(price("1020.00"));
    let mut inputs = base_inputs();
    inputs.subject = PlanSubject::StrategySignal(signal);
    inputs.side = OrderSide::Sell;
    inputs.level_map = Some(LevelMap::new(vec![level]));
    inputs.last_price = price("1000.00");

    let plan: TradePlan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();

    let produced = serde_json::to_value(&plan).unwrap();
    assert_eq!(
        produced, fixture,
        "short StrategySignal-subject plan drifted from its golden vector"
    );

    let loaded: TradePlan = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, plan);
    assert_eq!(loaded.side, OrderSide::Sell);
    assert_eq!(loaded.initial_stop, price("1020.00"));
    assert_eq!(loaded.targets[0].price, price("960.00"));
}
