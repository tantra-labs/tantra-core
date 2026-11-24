//! Unit tests for `tantra_core::trade_plan` (Plan 0005, P1) and the
//! `RiskBudget` / `PositionSizingRequest` weight & ADV extensions.

use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use tantra_core::{
    BreakevenTrigger, ChartPattern, EntryRule, Exchange, InstrumentId, Level, LevelId, LevelKind,
    LevelMap, LevelSide, MaKind, Money, OrderSide, PatternKind, PatternPivot, PatternStage,
    PlanError, PlanSubject, PlannerInputs, PositionSide, Quantity, RiskBudget, Signal,
    SignalDirection, SignalHorizon, SignalKind, SizingMethod, SizingSpec, StopRule, StrategySignal,
    TargetLadder, TargetLeg, TargetTrigger, Timeframe, TradePlanSpec, TradePlanner, TrailRule,
    TrailSpec,
};
use tantra_core::{PositionSizingRequest, calculate_position_size};

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn instrument() -> InstrumentId {
    InstrumentId::try_new(Exchange::Nse, "RELIANCE").unwrap()
}

fn ts() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000, 0).unwrap()
}

fn price(s: &str) -> tantra_core::Price {
    tantra_core::Price::from_rupees(s).unwrap()
}

fn budget() -> RiskBudget {
    RiskBudget::new(
        Money::inr(dec("100000.00")),
        dec("0.01"),
        Quantity::from_u64(1),
    )
}

/// Default long market spec with a `Fixed(980)` stop and an `AtR(2)` half exit.
fn long_spec() -> TradePlanSpec {
    TradePlanSpec::new(
        "test_long",
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

fn long_inputs() -> PlannerInputs {
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
fn plan_market_long_fixed_stop_happy_path() {
    let plan = TradePlanner::plan(&long_spec(), &long_inputs(), &budget()).unwrap();
    assert_eq!(plan.instrument, instrument());
    assert_eq!(plan.side, OrderSide::Buy);
    assert_eq!(plan.entry, price("1000.00"));
    assert_eq!(plan.initial_stop, price("980.00"));
    assert_eq!(plan.risk_per_unit, dec("20.00"));
    assert_eq!(plan.quantity, Quantity::from_u64(50));
    assert_eq!(plan.risk_at_stake.amount, dec("1000.00"));
    assert_eq!(plan.risk_at_stake.currency, tantra_core::Currency::Inr);
    assert_eq!(plan.targets.len(), 1);
    assert_eq!(plan.targets[0].price, price("1040.00"));
    assert_eq!(plan.targets[0].qty, Quantity::from_u64(25));
    assert_eq!(plan.reward_risk, Decimal::from(2));
    assert_eq!(plan.spec_version, 1);
    assert!(plan.rationale.contains("fixed"));
    assert!(plan.rationale.contains("fixed_fractional"));
}

#[test]
fn plan_happy_path_for_each_stop_rule() {
    // Fixed
    let plan = TradePlanner::plan(&long_spec(), &long_inputs(), &budget()).unwrap();
    assert_eq!(plan.initial_stop, price("980.00"));

    // Percent: 2% below a 1000 entry.
    let spec = TradePlanSpec::new(
        "pct",
        1,
        EntryRule::Market,
        StopRule::Percent(dec("0.02")),
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &long_inputs(), &budget()).unwrap();
    assert_eq!(plan.initial_stop, price("980.00"));

    // AtrMultiple: 2 × atr(10) below a 1000 entry.
    let spec = TradePlanSpec::new(
        "atr",
        1,
        EntryRule::Market,
        StopRule::AtrMultiple {
            mult: Decimal::from(2),
        },
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &long_inputs(), &budget()).unwrap();
    assert_eq!(plan.initial_stop, price("980.00"));

    // LevelBand: support band low 970 − 0.5 × atr(10) = 965.
    let level = Level::new(
        LevelId(1),
        LevelKind::ElephantSupport1,
        LevelSide::Support,
        (dec("970.00"), dec("980.00")),
        4,
    )
    .unwrap();
    let mut inputs = long_inputs();
    inputs.level_map = Some(LevelMap::new(vec![level]));
    inputs.adr_pct = Some(dec("5.00"));
    let spec = TradePlanSpec::new(
        "band",
        1,
        EntryRule::Market,
        StopRule::LevelBand {
            band_id: LevelId(1),
            buffer_atr: dec("0.5"),
        },
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();
    assert_eq!(plan.initial_stop, price("965.00"));

    // Swing: stop at the swing low.
    let mut inputs = long_inputs();
    inputs.swing_low = Some(price("972.00"));
    let spec = TradePlanSpec::new(
        "swing",
        1,
        EntryRule::Market,
        StopRule::Swing,
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();
    assert_eq!(plan.initial_stop, price("972.00"));

    // DayExtreme: stop at the day low.
    let mut inputs = long_inputs();
    inputs.day_low = Some(price("975.00"));
    let spec = TradePlanSpec::new(
        "day",
        1,
        EntryRule::Market,
        StopRule::DayExtreme,
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();
    assert_eq!(plan.initial_stop, price("975.00"));

    // Chandelier: swing high 1010 − 3 × atr(10) = 980.
    let mut inputs = long_inputs();
    inputs.swing_high = Some(price("1010.00"));
    let spec = TradePlanSpec::new(
        "chandelier",
        1,
        EntryRule::Market,
        StopRule::Chandelier {
            period: 20,
            mult: Decimal::from(3),
        },
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();
    assert_eq!(plan.initial_stop, price("980.00"));

    // PatternInvalidation (no buffer): bullish pattern invalidation 975.
    let pattern = bull_pattern(price("975.00"));
    let mut inputs = long_inputs();
    inputs.subject = PlanSubject::ChartPattern(pattern);
    inputs.last_price = price("1000.00");
    let spec = TradePlanSpec::new(
        "pattern",
        1,
        EntryRule::Market,
        StopRule::PatternInvalidation { buffer_atr: None },
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();
    assert_eq!(plan.initial_stop, price("975.00"));
}

fn bull_pattern(invalidation: tantra_core::Price) -> ChartPattern {
    ChartPattern::new(
        PatternKind::Flag,
        SignalDirection::Bullish,
        Timeframe::M5,
        PatternStage::FormingArmed,
        vec![PatternPivot {
            bar_ts: ts(),
            price: price("990.00"),
            kind: "high".to_string(),
        }],
        price("1005.00"),
        invalidation,
        Some(price("1060.00")),
        80,
        ts(),
        ts(),
        None,
    )
    .unwrap()
}

#[test]
fn plan_refuses_zero_risk() {
    let spec = TradePlanSpec::new(
        "zero",
        1,
        EntryRule::Market,
        StopRule::Fixed(price("1000.00")),
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let err = TradePlanner::plan(&spec, &long_inputs(), &budget()).unwrap_err();
    assert_eq!(err, PlanError::ZeroRisk);
}

#[test]
fn plan_refuses_stop_too_wide() {
    // Stop 50 points below a 1000 entry = 5%, above the 1×ADR% (2%) guard.
    let spec = TradePlanSpec::new(
        "wide",
        1,
        EntryRule::Market,
        StopRule::Fixed(price("950.00")),
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let mut inputs = long_inputs();
    inputs.adr_pct = Some(dec("2.00"));
    let err = TradePlanner::plan(&spec, &inputs, &budget()).unwrap_err();
    assert_eq!(err, PlanError::StopTooWide);
}

#[test]
fn plan_refuses_chase_guard() {
    // Signal recommended entry 980, market now 1000, atr 5 → chasing 20 > 10.
    let signal = Signal::new(
        instrument(),
        SignalDirection::Bullish,
        SignalHorizon::Intraday,
        Timeframe::M1,
        dec("0.80"),
        "test",
        ts(),
    )
    .with_entry_price(price("980.00"));
    let mut inputs = long_inputs();
    inputs.subject = PlanSubject::Signal(signal);
    inputs.atr = Some(dec("5.00"));
    let err = TradePlanner::plan(&long_spec(), &inputs, &budget()).unwrap_err();
    assert_eq!(err, PlanError::ChaseGuard);
}

#[test]
fn plan_refuses_below_min_lot() {
    let budget = RiskBudget::new(
        Money::inr(dec("100000.00")),
        dec("0.01"),
        Quantity::from_u64(100),
    );
    // qty = 50 lots < min_lot 100.
    let err = TradePlanner::plan(&long_spec(), &long_inputs(), &budget).unwrap_err();
    assert_eq!(err, PlanError::BelowMinLot);
}

#[test]
fn plan_refuses_insufficient_reward() {
    // First target at 2R is below the 2.5R minimum.
    let spec = long_spec().with_min_reward_risk(dec("2.5"));
    let err = TradePlanner::plan(&spec, &long_inputs(), &budget()).unwrap_err();
    assert_eq!(err, PlanError::InsufficientReward);
}

#[test]
fn plan_missing_input_opening_range_break_without_level_map() {
    let spec = TradePlanSpec::new(
        "orb",
        1,
        EntryRule::OpeningRangeBreak { minutes: 15 },
        long_spec().stop,
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let err = TradePlanner::plan(&spec, &long_inputs(), &budget()).unwrap_err();
    assert_eq!(err, PlanError::MissingInput("level_map"));
}

#[test]
fn plan_missing_input_atr_multiple_without_atr() {
    let mut inputs = long_inputs();
    inputs.atr = None;
    let spec = TradePlanSpec::new(
        "atr",
        1,
        EntryRule::LimitAt(price("1000.00")),
        StopRule::AtrMultiple {
            mult: Decimal::from(2),
        },
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let err = TradePlanner::plan(&spec, &inputs, &budget()).unwrap_err();
    assert_eq!(err, PlanError::MissingInput("atr"));
}

#[test]
fn plan_missing_input_pattern_breakout_without_pattern() {
    let spec = TradePlanSpec::new(
        "pb",
        1,
        EntryRule::PatternBreakout { buffer_ticks: 2 },
        long_spec().stop,
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let err = TradePlanner::plan(&spec, &long_inputs(), &budget()).unwrap_err();
    assert_eq!(err, PlanError::MissingInput("pattern"));
}

#[test]
fn plan_refuses_invalidated_pattern() {
    // Bullish pattern invalidated at 975; last price is already through it.
    let mut inputs = long_inputs();
    inputs.subject = PlanSubject::ChartPattern(bull_pattern(price("975.00")));
    inputs.last_price = price("970.00");
    let err = TradePlanner::plan(&long_spec(), &inputs, &budget()).unwrap_err();
    assert_eq!(err, PlanError::PatternInvalidated);
}

#[test]
fn plan_tick_snapping_is_conservative_long() {
    // Entry snapped up (never better than market), stop snapped down (never
    // loosens risk), target snapped toward the entry.
    let mut inputs = long_inputs();
    inputs.tick_size = price("0.25");
    inputs.last_price = price("1000.37");
    let spec = TradePlanSpec::new(
        "snap",
        1,
        EntryRule::Market,
        StopRule::Fixed(price("980.19")),
        long_spec().trail,
        TargetLadder::new(
            vec![TargetLeg {
                trigger: TargetTrigger::AtPrice(price("1040.39")),
                fraction: Decimal::ONE,
            }],
            false,
        )
        .unwrap(),
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();

    // Entry snapped up: 1000.37 → 1000.50.
    assert_eq!(plan.entry.value(), dec("1000.50"));
    assert!(
        plan.entry.value() >= inputs.last_price.value(),
        "buy entry never improves on market"
    );
    // Stop snapped down (away from entry): 980.19 → 980.00.
    assert_eq!(plan.initial_stop.value(), dec("980.00"));
    assert!(
        plan.initial_stop.value() <= dec("980.19"),
        "stop never loosens for a long"
    );
    // Risk distance never shrinks.
    let raw_risk = dec("1000.37") - dec("980.19");
    assert!(
        plan.risk_per_unit >= raw_risk,
        "snapped risk must never loosen"
    );
    // Target snapped toward the entry: 1040.39 → 1040.25.
    assert_eq!(plan.targets[0].price.value(), dec("1040.25"));
    assert!(
        plan.targets[0].price.value() <= dec("1040.39"),
        "target snapped toward entry"
    );
}

#[test]
fn plan_tick_snapping_is_conservative_short() {
    let mut inputs = long_inputs();
    inputs.subject = PlanSubject::Direct;
    inputs.side = OrderSide::Sell;
    inputs.tick_size = price("0.25");
    inputs.last_price = price("1000.37");
    inputs.day_high = None;
    inputs.day_low = None;
    let spec = TradePlanSpec::new(
        "snap_short",
        1,
        EntryRule::Market,
        StopRule::Fixed(price("1020.19")),
        long_spec().trail,
        TargetLadder::new(
            vec![TargetLeg {
                trigger: TargetTrigger::AtPrice(price("960.31")),
                fraction: Decimal::ONE,
            }],
            false,
        )
        .unwrap(),
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();

    // Entry snapped down: 1000.37 → 1000.25.
    assert_eq!(plan.entry.value(), dec("1000.25"));
    assert!(
        plan.entry.value() <= inputs.last_price.value(),
        "sell entry never improves on market"
    );
    // Stop snapped up (away from entry): 1020.19 → 1020.25.
    assert_eq!(plan.initial_stop.value(), dec("1020.25"));
    assert!(
        plan.initial_stop.value() >= dec("1020.19"),
        "stop never loosens for a short"
    );
    let raw_risk = dec("1020.19") - dec("1000.37");
    assert!(
        plan.risk_per_unit >= raw_risk,
        "snapped risk must never loosen"
    );
    // Target snapped toward the entry: 960.31 → 960.50.
    assert_eq!(plan.targets[0].price.value(), dec("960.50"));
    assert!(
        plan.targets[0].price.value() >= dec("960.31"),
        "target snapped toward entry"
    );
}

#[test]
fn plan_strategy_signal_subject_end_to_end() {
    let signal = StrategySignal::new(
        OrderSide::Buy,
        SignalKind::Breakout,
        vec![LevelId(1)],
        dec("80"),
        instrument(),
        ts(),
    )
    .unwrap();
    let level = Level::new(
        LevelId(1),
        LevelKind::ElephantSupport1,
        LevelSide::Support,
        (dec("970.00"), dec("980.00")),
        4,
    )
    .unwrap();
    let mut inputs = long_inputs();
    inputs.subject = PlanSubject::StrategySignal(signal);
    inputs.level_map = Some(LevelMap::new(vec![level]));

    let plan = TradePlanner::plan(&long_spec(), &inputs, &budget()).unwrap();
    assert_eq!(plan.side, OrderSide::Buy);
    assert_eq!(plan.instrument, instrument());
    assert_eq!(plan.quantity, Quantity::from_u64(50));
    assert_eq!(plan.targets.len(), 1);
}

#[test]
fn plan_pattern_subject_end_to_end() {
    let pattern = bull_pattern(price("975.00"));
    let mut inputs = long_inputs();
    inputs.subject = PlanSubject::ChartPattern(pattern.clone());
    inputs.last_price = price("1000.00");
    inputs.adr_pct = Some(dec("5.00"));

    let spec = TradePlanSpec::new(
        "pattern_breakout",
        1,
        EntryRule::PatternBreakout { buffer_ticks: 2 },
        StopRule::PatternInvalidation { buffer_atr: None },
        long_spec().trail,
        TargetLadder::new(
            vec![TargetLeg {
                trigger: TargetTrigger::MeasuredMove {
                    fraction_of_height: Decimal::ONE,
                    fallback: None,
                },
                fraction: Decimal::ONE,
            }],
            false,
        )
        .unwrap(),
        long_spec().sizing,
    );

    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();
    // Entry = breakout 1005 + 2 ticks (0.10) = 1005.10, snapped up.
    assert_eq!(plan.entry.value(), dec("1005.10"));
    assert_eq!(plan.initial_stop.value(), dec("975.00"));
    // Measured move = |1005 − 975| = 30 → target 1035.10.
    assert_eq!(plan.targets.len(), 1);
    assert_eq!(plan.targets[0].price.value(), dec("1035.10"));
    assert_eq!(plan.targets[0].reason, "measured move");
    assert_eq!(plan.side, OrderSide::Buy);
    let _ = pattern;
}

#[test]
fn target_ladder_rejects_fractions_above_one() {
    let result = TargetLadder::new(
        vec![
            TargetLeg {
                trigger: TargetTrigger::AtR(Decimal::from(1)),
                fraction: dec("0.8"),
            },
            TargetLeg {
                trigger: TargetTrigger::AtR(Decimal::from(2)),
                fraction: dec("0.3"),
            },
        ],
        true,
    );
    assert!(result.is_err());
}

#[test]
fn sizing_caps_weight_and_adv() {
    // A FixedFractional(1%) budget that would buy 50 qty, with a 4% weight cap
    // (notional 50000 / 100000 = 50% → must cap down to 4000 notional = 4 qty).
    let budget = RiskBudget::new(
        Money::inr(dec("100000.00")),
        dec("0.01"),
        Quantity::from_u64(1),
    )
    .with_max_weight_pct(dec("0.04"));
    let request = PositionSizingRequest::new(
        instrument(),
        PositionSide::Long,
        price("1000.00"),
        Money::inr(dec("100000.00")),
        budget,
        SizingMethod::FixedFractional {
            risk_pct: dec("0.01"),
        },
    )
    .with_stop_loss(price("980.00"));
    let result = calculate_position_size(&request).unwrap();
    assert_eq!(result.quantity, Quantity::from_u64(4));
    assert!(result.capped_by_limit);
    assert!(result.reason.as_deref().unwrap().contains("max_weight_pct"));

    // ADV cap: max 2% of adv20 = 2% of 500 = 10 qty.
    let budget = RiskBudget::new(
        Money::inr(dec("100000.00")),
        dec("0.01"),
        Quantity::from_u64(1),
    )
    .with_max_adv_pct(dec("0.02"));
    let request = PositionSizingRequest::new(
        instrument(),
        PositionSide::Long,
        price("1000.00"),
        Money::inr(dec("100000.00")),
        budget,
        SizingMethod::FixedFractional {
            risk_pct: dec("0.01"),
        },
    )
    .with_stop_loss(price("980.00"))
    .with_adv20(Quantity::from_u64(500));
    let result = calculate_position_size(&request).unwrap();
    assert_eq!(result.quantity, Quantity::from_u64(10));
    assert!(result.capped_by_limit);
    assert!(result.reason.as_deref().unwrap().contains("max_adv_pct"));
}

#[test]
fn plan_weight_caps_from_sizing_spec() {
    // SizingSpec max_weight_pct 4% caps the 50-qty position to 4 qty; the
    // ladder leg then carries 2 qty (50% of the position).
    let spec = TradePlanSpec::new(
        "capped",
        1,
        EntryRule::Market,
        StopRule::Fixed(price("980.00")),
        long_spec().trail,
        long_spec().targets,
        SizingSpec {
            method: SizingMethod::FixedFractional {
                risk_pct: dec("0.01"),
            },
            max_weight_pct: Some(dec("0.04")),
            max_adv_pct: None,
        },
    );
    let plan = TradePlanner::plan(&spec, &long_inputs(), &budget()).unwrap();
    assert_eq!(plan.quantity, Quantity::from_u64(4));
    assert_eq!(plan.targets[0].qty, Quantity::from_u64(2));
    let weight = plan.entry.value() * plan.quantity.value() / dec("100000.00");
    assert!(weight <= dec("0.04"));
}

#[test]
fn plan_sell_side_end_to_end() {
    let mut inputs = long_inputs();
    inputs.side = OrderSide::Sell;
    inputs.subject = PlanSubject::Direct;
    inputs.last_price = price("1000.00");
    let spec = TradePlanSpec::new(
        "short",
        1,
        EntryRule::Market,
        StopRule::Fixed(price("1020.00")),
        long_spec().trail,
        long_spec().targets,
        long_spec().sizing,
    );
    let plan = TradePlanner::plan(&spec, &inputs, &budget()).unwrap();
    assert_eq!(plan.side, OrderSide::Sell);
    assert_eq!(plan.entry, price("1000.00"));
    assert_eq!(plan.initial_stop, price("1020.00"));
    assert_eq!(plan.targets[0].price, price("960.00"));
    assert_eq!(plan.reward_risk, Decimal::from(2));
}

#[test]
fn serde_roundtrip_trade_plan_spec() {
    let json = serde_json::to_string(&long_spec()).unwrap();
    let back: TradePlanSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(long_spec(), back);
    let _ = BreakevenTrigger::AfterFirstTarget;
    let _ = MaKind::Sma;
}
