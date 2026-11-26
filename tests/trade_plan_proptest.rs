//! Property-based invariants for `tantra_core::trade_plan` (Plan 0005, P1).

use chrono::{TimeZone, Utc};
use proptest::prelude::*;
use rust_decimal::Decimal;
use tantra_core::{
    EntryRule, Exchange, InstrumentId, Money, OrderSide, PlanSubject, PlannerInputs, Quantity,
    RiskBudget, SizingMethod, SizingSpec, StopRule, TargetLadder, TargetLeg, TargetTrigger,
    TradePlanSpec, TradePlanner, TrailRule, TrailSpec,
};
use tantra_core::{PositionSide, Price};
use tantra_core::{PositionSizingRequest, calculate_position_size};

fn ts() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000, 0).unwrap()
}

fn instrument() -> InstrumentId {
    InstrumentId::try_new(Exchange::Nse, "RELIANCE").unwrap()
}

/// Build a `Direct`-subject long market plan. `adr_pct` is left unset so the
/// `max_stop_distance` guard never interferes with the sizing/snapping
/// properties under test.
fn plan_long(
    entry: Decimal,
    tick: Decimal,
    stop: Decimal,
    equity: Decimal,
    risk_pct: Decimal,
    min_lot: u64,
    budget_tweaks: &dyn Fn(RiskBudget) -> RiskBudget,
) -> Option<tantra_core::TradePlan> {
    if !(entry > Decimal::ZERO && stop > Decimal::ZERO && stop < entry) {
        return None;
    }
    let spec = TradePlanSpec::new(
        "prop_long",
        1,
        EntryRule::Market,
        StopRule::Fixed(Price::try_new(stop).unwrap()),
        TrailSpec {
            rule: TrailRule::Off,
            activation_after_r: Decimal::ONE,
            breakeven_after: None,
            time_stop: None,
        },
        TargetLadder::new(
            vec![TargetLeg {
                trigger: TargetTrigger::AtR(Decimal::from(2)),
                fraction: Decimal::ONE,
            }],
            false,
        )
        .unwrap(),
        SizingSpec {
            method: SizingMethod::FixedFractional { risk_pct },
            max_weight_pct: None,
            max_adv_pct: None,
        },
    );
    let inputs = PlannerInputs {
        subject: PlanSubject::Direct,
        instrument: instrument(),
        side: OrderSide::Buy,
        last_price: Price::try_new(entry).unwrap(),
        tick_size: Price::try_new(tick).unwrap(),
        atr: Some(entry / Decimal::from(100)),
        adr_pct: None,
        swing_low: None,
        swing_high: None,
        day_low: None,
        day_high: None,
        ma_values: vec![],
        adv20: None,
        level_map: None,
        timestamp: ts(),
    };
    let budget = budget_tweaks(RiskBudget::new(
        Money::inr(equity),
        risk_pct,
        Quantity::from_u64(min_lot),
    ));
    TradePlanner::plan(&spec, &inputs, &budget).ok()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn prop_risk_at_stake_bounded_and_qty_lot_multiple(
        entry in 100i64..100_000i64,
        stop_gap in 1i64..=5_000i64,
        equity in 100_000i64..100_000_000i64,
        risk_bps in 1i64..=500i64,
        min_lot in 1u64..=100u64,
    ) {
        let entry_d = Decimal::from(entry);
        let stop_d = Decimal::from(entry - stop_gap);
        let risk_pct = Decimal::from(risk_bps) / Decimal::from(10000);
        let risk_amount = Decimal::from(equity) * risk_pct;
        let min_lot_d = Decimal::from(min_lot);

        if let Some(plan) = plan_long(
            entry_d,
            Decimal::from(1), // 1-rupee tick keeps every price aligned
            stop_d,
            Decimal::from(equity),
            risk_pct,
            min_lot,
            &|budget| budget,
        ) {
            prop_assert!(
                plan.risk_at_stake.amount() <= risk_amount,
                "risk_at_stake {} must be <= equity x risk_pct {}",
                plan.risk_at_stake.amount(),
                risk_amount
            );
            prop_assert_eq!(
                plan.quantity.value() % min_lot_d,
                Decimal::ZERO,
                "quantity must be a multiple of min_lot"
            );
        }
    }

    #[test]
    fn prop_weight_and_adv_caps_respected(
        entry in 100i64..100_000i64,
        stop_gap in 1i64..=5_000i64,
        equity in 100_000i64..100_000_000i64,
        risk_bps in 1i64..=500i64,
        min_lot in 1u64..=100u64,
        adv20 in 100u64..10_000_000u64,
    ) {
        let entry_d = Decimal::from(entry);
        let stop_d = Decimal::from(entry - stop_gap);
        let risk_pct = Decimal::from(risk_bps) / Decimal::from(10000);
        let equity_d = Decimal::from(equity);
        let max_weight_pct = Decimal::from(10) / Decimal::from(100); // 10%
        let max_adv_pct = Decimal::from(5) / Decimal::from(100); // 5%
        let adv20_d = Decimal::from(adv20);

        if let Some(plan) = plan_long(
            entry_d,
            Decimal::from(1),
            stop_d,
            equity_d,
            risk_pct,
            min_lot,
            &|budget| budget.with_max_weight_pct(max_weight_pct).with_max_adv_pct(max_adv_pct),
        ) {
            let weight = plan.entry.value() * plan.quantity.value() / equity_d;
            prop_assert!(
                weight <= max_weight_pct,
                "weight {} must be <= max_weight_pct {}",
                weight,
                max_weight_pct
            );
            let adv_ratio = plan.quantity.value() / adv20_d;
            prop_assert!(
                adv_ratio <= max_adv_pct,
                "qty/adv20 {} must be <= max_adv_pct {}",
                adv_ratio,
                max_adv_pct
            );
            // The same caps must hold for the raw sizing result.
            let budget = RiskBudget::new(
                Money::inr(equity_d),
                risk_pct,
                Quantity::from_u64(min_lot),
            )
            .with_max_weight_pct(max_weight_pct)
            .with_max_adv_pct(max_adv_pct);
            let request = PositionSizingRequest::new(
                instrument(),
                PositionSide::Long,
                Price::try_new(entry_d).unwrap(),
                Money::inr(equity_d),
                budget,
                SizingMethod::FixedFractional { risk_pct },
            )
            .with_stop_loss(Price::try_new(stop_d).unwrap())
            .with_adv20(Quantity::from_u64(adv20));
            if let Ok(result) = calculate_position_size(&request) {
                let weight = result.allocated_notional.amount() / equity_d;
                prop_assert!(weight <= max_weight_pct);
                prop_assert!(result.quantity.value() / adv20_d <= max_adv_pct);
            }
        }
    }

    #[test]
    fn prop_stop_never_loosens_after_snapping(
        last_paise in 10_000i64..10_000_000i64,
        stop_gap in 1i64..=10_000i64,
        tick_paise in 1i64..=500i64,
        equity in 100_000i64..100_000_000i64,
        min_lot in 1u64..=10u64,
    ) {
        let last = Decimal::from(last_paise) / Decimal::from(100);
        let tick = Decimal::from(tick_paise) / Decimal::from(100);
        let raw_stop = last - Decimal::from(stop_gap) * tick;
        prop_assume!(raw_stop > Decimal::ZERO);
        let risk_pct = Decimal::from(50) / Decimal::from(10000);

        // Long: entry snapped up, stop snapped down (away from entry).
        let plan = plan_long(
            last,
            tick,
            raw_stop,
            Decimal::from(equity),
            risk_pct,
            min_lot,
            &|budget| budget,
        );
        if let Some(plan) = plan {
            prop_assert!(
                plan.entry.value() >= last,
                "buy entry must snap up (never better than market)"
            );
            prop_assert!(
                plan.initial_stop.value() <= raw_stop,
                "long stop must never loosen after snapping"
            );
            let raw_risk = last - raw_stop;
            prop_assert!(
                plan.risk_per_unit >= raw_risk,
                "snapped risk must never shrink"
            );
        }
    }

    #[test]
    fn prop_short_stop_never_loosens_after_snapping(
        last_paise in 10_000i64..10_000_000i64,
        stop_gap in 1i64..=10_000i64,
        tick_paise in 1i64..=500i64,
        equity in 100_000i64..100_000_000i64,
        min_lot in 1u64..=10u64,
    ) {
        let last = Decimal::from(last_paise) / Decimal::from(100);
        let tick = Decimal::from(tick_paise) / Decimal::from(100);
        let raw_stop = last + Decimal::from(stop_gap) * tick;
        let risk_pct = Decimal::from(50) / Decimal::from(10000);

        let spec = TradePlanSpec::new(
            "prop_short",
            1,
            EntryRule::Market,
            StopRule::Fixed(Price::try_new(raw_stop).unwrap()),
            TrailSpec {
                rule: TrailRule::Off,
                activation_after_r: Decimal::ONE,
                breakeven_after: None,
                time_stop: None,
            },
            TargetLadder::new(
                vec![TargetLeg {
                    trigger: TargetTrigger::AtR(Decimal::from(2)),
                    fraction: Decimal::ONE,
                }],
                false,
            )
            .unwrap(),
            SizingSpec {
                method: SizingMethod::FixedFractional { risk_pct },
                max_weight_pct: None,
                max_adv_pct: None,
            },
        );
        let inputs = PlannerInputs {
            subject: PlanSubject::Direct,
            instrument: instrument(),
            side: OrderSide::Sell,
            last_price: Price::try_new(last).unwrap(),
            tick_size: Price::try_new(tick).unwrap(),
            atr: Some(last / Decimal::from(100)),
            adr_pct: None,
            swing_low: None,
            swing_high: None,
            day_low: None,
            day_high: None,
            ma_values: vec![],
            adv20: None,
            level_map: None,
            timestamp: ts(),
        };
        let budget = RiskBudget::new(
            Money::inr(Decimal::from(equity)),
            risk_pct,
            Quantity::from_u64(min_lot),
        );
        if let Ok(plan) = TradePlanner::plan(&spec, &inputs, &budget) {
            prop_assert!(
                plan.entry.value() <= last,
                "sell entry must snap down (never better than market)"
            );
            prop_assert!(
                plan.initial_stop.value() >= raw_stop,
                "short stop must never loosen after snapping"
            );
            let raw_risk = raw_stop - last;
            prop_assert!(
                plan.risk_per_unit >= raw_risk,
                "snapped risk must never shrink"
            );
        }
    }
}
