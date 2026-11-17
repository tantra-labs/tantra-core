//! Trade planning: entry / stop / trailing / target-ladder / sizing → [`TradePlan`].
//!
//! [`TradePlanner::plan`] turns a versioned [`TradePlanSpec`] plus live
//! [`PlannerInputs`] into a fully-quantized, conservatively tick-snapped,
//! risk-sized [`TradePlan`]. It is the keystone consumed by `tantra-exec`
//! (gap 0003), `tantra-risk` (gap 0005), and `tantra-ledger` (gap 0012).
//!
//! Zero-I/O, `Decimal`-only, WASM-clean. Plan 0005, slice P1. The TOML presets
//! and the `TradeManager` state machine are later slices.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::chart_pattern::{ChartPattern, PatternStage};
use crate::error::CoreError;
use crate::instrument::PositionSide;
use crate::levels::{LevelId, LevelMap};
use crate::order::OrderSide;
use crate::signal::{Signal, SignalDirection, Timeframe};
use crate::signals::StrategySignal;
use crate::sizing::{PositionSizingRequest, RiskBudget, SizingMethod, calculate_position_size};
use crate::types::{InstrumentId, Money, Price, Quantity};

/// Chase guard: a market entry may not chase price more than this multiple of
/// ATR away from the subject's reference (e.g. a signal's recommended entry or
/// a pattern's breakout level).
pub const MAX_EXTENSION_ATR: u32 = 2;

/// How the trade enters the market.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EntryRule {
    /// Enter at the prevailing market price.
    Market,
    /// Enter at a resting limit price or better.
    LimitAt(Price),
    /// Buy stop: enter when price trades at or above `trigger`.
    StopAbove {
        /// Trigger price.
        trigger: Price,
    },
    /// Sell stop: enter when price trades at or below `trigger`.
    StopBelow {
        /// Trigger price.
        trigger: Price,
    },
    /// Enter on an opening-range break; requires a [`LevelMap`] input. The
    /// entry anchors to the tightest resistance band high (long) / support
    /// band low (short) on the map.
    OpeningRangeBreak {
        /// Opening range length in minutes.
        minutes: u32,
    },
    /// Enter on a chart-pattern breakout; requires a [`PlanSubject::ChartPattern`].
    /// Resting breakout trigger when [`PatternStage::FormingArmed`], market
    /// entry when [`PatternStage::BreakoutConfirmed`].
    PatternBreakout {
        /// Buffer in ticks beyond the breakout level.
        buffer_ticks: u32,
    },
}

/// How the initial stop loss is derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum StopRule {
    /// Absolute stop price.
    Fixed(Price),
    /// Stop at `entry × (1 ∓ pct)` (below entry for longs, above for shorts).
    Percent(Decimal),
    /// Stop at `entry ∓ mult × atr`.
    AtrMultiple {
        /// ATR multiple.
        mult: Decimal,
    },
    /// Stop at a level-map level's band edge `∓ buffer_atr × atr`.
    LevelBand {
        /// Level-map level identifier.
        band_id: LevelId,
        /// ATR buffer beyond the band edge.
        buffer_atr: Decimal,
    },
    /// Stop at the swing low (long) / swing high (short).
    Swing,
    /// Stop at the session's extreme: day low (long) / day high (short).
    DayExtreme,
    /// Chandelier exit: `highest_high - mult × atr` (long) /
    /// `lowest_low + mult × atr` (short).
    Chandelier {
        /// Lookback periods for the highest high / lowest low.
        period: u32,
        /// ATR multiple.
        mult: Decimal,
    },
    /// Stop at the chart pattern's invalidation level, optionally buffered by
    /// `buffer_atr × atr`.
    PatternInvalidation {
        /// Optional ATR buffer beyond the invalidation level.
        buffer_atr: Option<Decimal>,
    },
}

impl StopRule {
    /// Default cap on stop distance, as a multiple of ADR% (1×).
    pub const DEFAULT_MAX_STOP_ADR_MULT: u32 = 1;

    /// The `max_stop_distance` guard multiplier for this rule. ATR-anchored
    /// rules default to their own multiple (floored at 1); everything else
    /// uses [`StopRule::DEFAULT_MAX_STOP_ADR_MULT`].
    #[must_use]
    pub fn max_stop_adr_mult(&self) -> Decimal {
        match self {
            Self::AtrMultiple { mult } | Self::Chandelier { mult, .. } => (*mult).max(Decimal::ONE),
            _ => Decimal::from(Self::DEFAULT_MAX_STOP_ADR_MULT),
        }
    }

    /// The `max_stop_distance` guard, evaluated cleanly in percent:
    /// `|entry - stop| / entry × 100 <= mult × adr_pct`.
    ///
    /// Returns `true` when the stop is too wide and the plan must be refused
    /// with [`PlanError::StopTooWide`].
    #[must_use]
    pub fn is_stop_too_wide(&self, entry: Decimal, stop: Decimal, adr_pct: Decimal) -> bool {
        if entry <= Decimal::ZERO {
            return false;
        }
        let distance_pct = (entry - stop).abs() / entry * Decimal::from(100);
        distance_pct > self.max_stop_adr_mult() * adr_pct
    }
}

/// Moving-average kind used by close-based trailing exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MaKind {
    /// Simple moving average.
    Sma,
    /// Exponential moving average.
    Ema,
}

/// How the stop trails the position after activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TrailRule {
    /// No trailing.
    Off,
    /// Trail `mult × atr` from the highest high (long) / lowest low (short).
    Atr {
        /// ATR multiple.
        mult: Decimal,
    },
    /// Trail on swing structure.
    Swing,
    /// Chandelier trail.
    Chandelier {
        /// Lookback periods.
        period: u32,
        /// ATR multiple.
        mult: Decimal,
    },
    /// Exit on a close beyond the moving average (long exits on a close below,
    /// short on a close above).
    CloseMa {
        /// Moving-average kind.
        kind: MaKind,
        /// Moving-average period.
        period: u32,
    },
    /// Trail a fixed percentage from the running extreme.
    Percent(Decimal),
}

/// Trigger that moves the stop to breakeven.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum BreakevenTrigger {
    /// Move to breakeven once the trade is `R` multiples in profit.
    AtR(Decimal),
    /// Move to breakeven after `N` sessions.
    AfterSessions(u32),
    /// Move to breakeven after the first target fills.
    AfterFirstTarget,
}

/// Time-stop configuration: exit if the trade has not reached `min_progress_r`
/// within `bars` bars of `timeframe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct TimeStop {
    /// Number of bars to allow for progress.
    pub bars: u32,
    /// Minimum progress (in R) required before the time stop expires.
    pub min_progress_r: Decimal,
    /// Bar interval the time stop counts in.
    pub timeframe: Timeframe,
}

/// Full trailing-stop configuration carried into a [`TradePlan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TrailSpec {
    /// The trailing rule applied after activation.
    pub rule: TrailRule,
    /// Trailing activates once the trade is `activation_after_r` multiples in
    /// profit.
    pub activation_after_r: Decimal,
    /// Optional breakeven trigger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breakeven_after: Option<BreakevenTrigger>,
    /// Optional time stop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_stop: Option<TimeStop>,
}

/// Trigger that prices a target leg.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetTrigger {
    /// Target at `R` multiples from the entry.
    AtR(Decimal),
    /// Absolute price target.
    AtPrice(Price),
    /// Time-based exit after `N` sessions (priced at the current market at
    /// plan time; the trade manager re-prices the fill at session close).
    AfterSessions(u32),
    /// Target at `fraction_of_height` of the pattern / level-map measured
    /// height, with an optional fallback trigger when no height is derivable.
    MeasuredMove {
        /// Fraction of the measured height to target (defaults to 1.0).
        #[serde(default = "default_measured_fraction")]
        fraction_of_height: Decimal,
        /// Fallback trigger when the measured height is unavailable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fallback: Option<Box<TargetTrigger>>,
    },
}

fn default_measured_fraction() -> Decimal {
    Decimal::ONE
}

/// One leg of a target ladder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TargetLeg {
    /// Trigger for this leg.
    pub trigger: TargetTrigger,
    /// Fraction of the position to exit at this leg, in `0..=1`.
    pub fraction: Decimal,
}

/// A ladder of scale-out targets plus an optional runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TargetLadder {
    /// Scale-out legs, in order.
    pub legs: Vec<TargetLeg>,
    /// Whether a runner position is kept after all legs fill.
    pub runner: bool,
}

impl TargetLadder {
    /// Validated constructor: every leg fraction must lie in `0..=1` and the
    /// fractions must sum to `<= 1` (the remainder, if any, is the runner).
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] on violation.
    pub fn new(legs: Vec<TargetLeg>, runner: bool) -> Result<Self, CoreError> {
        let mut sum = Decimal::ZERO;
        for leg in &legs {
            if !(Decimal::ZERO..=Decimal::ONE).contains(&leg.fraction) {
                return Err(CoreError::Validation(format!(
                    "target leg fraction must be in 0..=1: {}",
                    leg.fraction
                )));
            }
            sum += leg.fraction;
        }
        if sum > Decimal::ONE {
            return Err(CoreError::Validation(format!(
                "target ladder fractions must sum to at most 1, got {sum}"
            )));
        }
        Ok(Self { legs, runner })
    }
}

/// Position sizing configuration for a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SizingSpec {
    /// Position sizing model.
    pub method: SizingMethod,
    /// Per-trade position weight cap (`allocated_notional / equity`), if set.
    /// Overrides the account-level [`RiskBudget::max_weight_pct`] when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_weight_pct: Option<Decimal>,
    /// Per-trade share-of-ADV20 cap, if set. Overrides the account-level
    /// [`RiskBudget::max_adv_pct`] when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_adv_pct: Option<Decimal>,
}

/// A versioned, overridable trade plan specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TradePlanSpec {
    /// Preset identifier (`"qullamaggie"`, `"ee_plus"`, `"atr_swing"`,
    /// `"pattern_breakout"`, or a custom id).
    pub preset_id: String,
    /// Spec version (bumped by the preset author on rule changes).
    pub version: u32,
    /// Entry rule.
    pub entry: EntryRule,
    /// Initial stop rule.
    pub stop: StopRule,
    /// Trailing-stop configuration.
    pub trail: TrailSpec,
    /// Scale-out target ladder.
    pub targets: TargetLadder,
    /// Position sizing configuration.
    pub sizing: SizingSpec,
    /// Minimum reward/risk required from the first target, if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_reward_risk: Option<Decimal>,
}

impl TradePlanSpec {
    /// Create a spec with the core fields; every field is `pub` and therefore
    /// overridable after construction.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        preset_id: impl Into<String>,
        version: u32,
        entry: EntryRule,
        stop: StopRule,
        trail: TrailSpec,
        targets: TargetLadder,
        sizing: SizingSpec,
    ) -> Self {
        Self {
            preset_id: preset_id.into(),
            version,
            entry,
            stop,
            trail,
            targets,
            sizing,
            min_reward_risk: None,
        }
    }

    /// Attach a minimum reward/risk requirement.
    pub fn with_min_reward_risk(mut self, min_reward_risk: Decimal) -> Self {
        self.min_reward_risk = Some(min_reward_risk);
        self
    }
}

/// The subject a plan is derived from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PlanSubject {
    /// A canonical kernel [`Signal`] (direction and optional entry anchor).
    Signal(Signal),
    /// An EE+ [`StrategySignal`] (side and referenced level ids).
    StrategySignal(StrategySignal),
    /// A validated [`ChartPattern`] (breakout geometry).
    ChartPattern(ChartPattern),
    /// No subject — the plan is driven directly by [`PlannerInputs::side`].
    Direct,
}

/// Live planner inputs. All timestamps and market data are injected — zero
/// wall clock, zero I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannerInputs {
    /// The subject the plan is derived from.
    pub subject: PlanSubject,
    /// The instrument to trade.
    pub instrument: InstrumentId,
    /// Intended side (used for [`PlanSubject::Direct`]).
    pub side: OrderSide,
    /// Current market price.
    pub last_price: Price,
    /// Minimum price increment for conservative snapping.
    pub tick_size: Price,
    /// ATR of the bar interval, when computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atr: Option<Decimal>,
    /// Average daily range as a percentage (e.g. `2.5` means 2.5%), used by
    /// the `max_stop_distance` guard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adr_pct: Option<Decimal>,
    /// Swing low (longs anchor stops here).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swing_low: Option<Price>,
    /// Swing high (shorts anchor stops here).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swing_high: Option<Price>,
    /// Session low.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_low: Option<Price>,
    /// Session high.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_high: Option<Price>,
    /// Latest moving-average values `(kind, period, value)`, consumed by
    /// close-based trailing rules in the trade manager slice.
    pub ma_values: Vec<(MaKind, u32, Decimal)>,
    /// 20-day average daily volume, for liquidity caps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adv20: Option<Quantity>,
    /// The session's intraday level map, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_map: Option<LevelMap>,
    /// When the plan is being generated (injected).
    pub timestamp: DateTime<Utc>,
}

/// A single scale-out target in a [`TradePlan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlannedTarget {
    /// Limit price for this leg, snapped toward the entry.
    pub price: Price,
    /// Quantity to exit at this leg (lot-rounded down).
    pub qty: Quantity,
    /// Human-readable reason (`"target 1 at 2R"`, `"measured move"`, ...).
    pub reason: String,
}

/// A fully-quantized, risk-sized trade plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TradePlan {
    /// The instrument to trade.
    pub instrument: InstrumentId,
    /// Trade side.
    pub side: OrderSide,
    /// Conservative (tick-snapped) entry price.
    pub entry: Price,
    /// Conservative (tick-snapped) initial stop price.
    pub initial_stop: Price,
    /// `|entry - initial_stop|`, the per-unit risk.
    pub risk_per_unit: Decimal,
    /// Sized quantity (lot-rounded).
    pub quantity: Quantity,
    /// `risk_per_unit × quantity`, the total capital at stake.
    pub risk_at_stake: Money,
    /// Scale-out targets, in order.
    pub targets: Vec<PlannedTarget>,
    /// Trailing-stop configuration.
    pub trail: TrailSpec,
    /// Reward/risk of the first target.
    pub reward_risk: Decimal,
    /// Spec version this plan was built from.
    pub spec_version: u32,
    /// Human-readable rationale documenting the stop rule and sizing path.
    pub rationale: String,
}

/// Why a plan was refused.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// The initial stop distance exceeds the `max_stop_distance` guard.
    #[error("initial stop is too wide for the adr_pct max stop distance guard")]
    StopTooWide,
    /// Entry equals the initial stop — nothing at risk.
    #[error("entry equals initial stop (zero risk distance)")]
    ZeroRisk,
    /// The sized quantity is below the minimum lot size.
    #[error("sized quantity is below the minimum lot size")]
    BelowMinLot,
    /// A market entry would chase price beyond the chase allowance.
    #[error("market entry would chase price beyond the chase allowance")]
    ChaseGuard,
    /// The first target's reward/risk is below the minimum requirement.
    #[error("first target reward/risk is below min_reward_risk")]
    InsufficientReward,
    /// A required planner input is missing.
    #[error("missing planner input: {0}")]
    MissingInput(&'static str),
    /// The chart pattern was invalidated before the plan completed.
    #[error("the chart pattern was invalidated")]
    PatternInvalidated,
}

/// Pure, deterministic trade planner.
#[derive(Debug, Clone, Copy, Default)]
pub struct TradePlanner;

impl TradePlanner {
    /// Build a fully-snap[ped, risk-sized [`TradePlan`] from a spec and live
    /// inputs, failing closed on every guard.
    ///
    /// # Errors
    /// Returns [`PlanError`] for `ZeroRisk` (entry == stop), `ChaseGuard`
    /// (market entry extending > 2×ATR from the subject reference),
    /// `StopTooWide` (stop beyond `max_stop_adr_mult × adr_pct`),
    /// `BelowMinLot`, `InsufficientReward`, `MissingInput`, or
    /// `PatternInvalidated`.
    pub fn plan(
        spec: &TradePlanSpec,
        inputs: &PlannerInputs,
        budget: &RiskBudget,
    ) -> Result<TradePlan, PlanError> {
        let side = Self::resolve_side(inputs)?;

        // 0. Pattern invalidation guard: a chart-pattern subject whose
        //    invalidation level is already breached cannot be traded.
        if let PlanSubject::ChartPattern(pattern) = &inputs.subject {
            let invalidated = match pattern.direction {
                SignalDirection::Bullish => {
                    inputs.last_price.value() <= pattern.invalidation_level.value()
                }
                SignalDirection::Bearish => {
                    inputs.last_price.value() >= pattern.invalidation_level.value()
                }
                SignalDirection::Neutral => false,
            };
            if invalidated {
                return Err(PlanError::PatternInvalidated);
            }
        }

        // 1. Resolve the raw entry and initial stop.
        let raw_entry = Self::resolve_entry(spec, inputs, side)?;
        let raw_stop = Self::resolve_stop(spec, inputs, side, raw_entry.value())?;

        // 2. Conservative tick snapping: entry snapped against the market,
        //    stop snapped away from the entry (never loosening risk), targets
        //    snapped toward the entry.
        let tick = inputs.tick_size.value();
        let entry = snap_to_tick(raw_entry.value(), tick, &side, true);
        let stop = snap_to_tick(raw_stop.value(), tick, &side, false);

        // 3. ZeroRisk guard — refuse before any sizing fallback.
        let risk_per_unit = (entry - stop).abs();
        if risk_per_unit == Decimal::ZERO {
            return Err(PlanError::ZeroRisk);
        }

        // 4. Chase guard (market entries only). Requires atr.
        if matches!(spec.entry, EntryRule::Market) {
            let atr = inputs.atr.ok_or(PlanError::MissingInput("atr"))?;
            let reference = Self::chase_reference(inputs);
            let allowance = Decimal::from(MAX_EXTENSION_ATR) * atr;
            let chased = match side {
                OrderSide::Buy => inputs.last_price.value() > reference + allowance,
                OrderSide::Sell => inputs.last_price.value() < reference - allowance,
            };
            if chased {
                return Err(PlanError::ChaseGuard);
            }
        }

        // 5. StopTooWide guard via adr_pct (skipped when adr_pct is absent).
        if let Some(adr_pct) = inputs.adr_pct {
            if spec.stop.is_stop_too_wide(entry, stop, adr_pct) {
                return Err(PlanError::StopTooWide);
            }
        }

        // 6. Size the position through calculate_position_size, overriding the
        //    account-level caps with the per-trade SizingSpec caps.
        let mut sizing_budget = budget.clone();
        if let Some(weight) = spec.sizing.max_weight_pct {
            sizing_budget.max_weight_pct = Some(weight);
        }
        if let Some(adv) = spec.sizing.max_adv_pct {
            sizing_budget.max_adv_pct = Some(adv);
        }
        let pos_side = match side {
            OrderSide::Buy => PositionSide::Long,
            OrderSide::Sell => PositionSide::Short,
        };
        let mut request = PositionSizingRequest::new(
            inputs.instrument.clone(),
            pos_side,
            Price::new(entry),
            sizing_budget.equity,
            sizing_budget.clone(),
            spec.sizing.method.clone(),
        )
        .with_stop_loss(Price::new(stop));
        if let Some(adv20) = inputs.adv20 {
            request = request.with_adv20(adv20);
        }
        let sizing_result =
            calculate_position_size(&request).map_err(|_| PlanError::MissingInput("sizing"))?;
        let quantity = sizing_result.quantity;
        let min_lot = sizing_budget.min_lot_size.value();
        if quantity.value() < min_lot.max(Decimal::ONE) {
            return Err(PlanError::BelowMinLot);
        }

        // 7. Build the target ladder (snapping each target toward the entry).
        let mut targets = Vec::with_capacity(spec.targets.legs.len());
        for leg in &spec.targets.legs {
            if let Some((raw_price, reason)) =
                Self::target_price(leg.trigger.clone(), inputs, entry, risk_per_unit, side)?
            {
                // Targets snap toward the entry (never promising more than the
                // raw level); on the stop side this is the "away" direction.
                let target = snap_to_tick(raw_price, tick, &side, false);
                let qty = leg_quantity(quantity, leg.fraction, min_lot.max(Decimal::ONE));
                targets.push(PlannedTarget {
                    price: Price::new(target),
                    qty,
                    reason,
                });
            }
        }

        // 8. Reward/risk from the first target, then the min_reward_risk check.
        let reward_risk = targets
            .first()
            .map(|t| (t.price.value() - entry).abs() / risk_per_unit)
            .unwrap_or(Decimal::ZERO);
        if let Some(min_rr) = spec.min_reward_risk {
            if reward_risk < min_rr {
                return Err(PlanError::InsufficientReward);
            }
        }

        let risk_at_stake = Money::new(
            risk_per_unit * quantity.value(),
            sizing_budget.equity.currency,
        );
        let rationale = format!(
            "stop={} sizing={}",
            stop_label(&spec.stop),
            sizing_label(&spec.sizing.method)
        );

        Ok(TradePlan {
            instrument: inputs.instrument.clone(),
            side,
            entry: Price::new(entry),
            initial_stop: Price::new(stop),
            risk_per_unit,
            quantity,
            risk_at_stake,
            targets,
            trail: spec.trail.clone(),
            reward_risk,
            spec_version: spec.version,
            rationale,
        })
    }

    /// Resolve the trade side from the subject (`Direct` uses [`PlannerInputs::side`]).
    fn resolve_side(inputs: &PlannerInputs) -> Result<OrderSide, PlanError> {
        match &inputs.subject {
            PlanSubject::Signal(signal) => match signal.direction {
                SignalDirection::Bullish => Ok(OrderSide::Buy),
                SignalDirection::Bearish => Ok(OrderSide::Sell),
                SignalDirection::Neutral => Err(PlanError::MissingInput("directional signal")),
            },
            PlanSubject::StrategySignal(signal) => Ok(signal.side),
            PlanSubject::ChartPattern(pattern) => match pattern.direction {
                SignalDirection::Bullish => Ok(OrderSide::Buy),
                SignalDirection::Bearish => Ok(OrderSide::Sell),
                SignalDirection::Neutral => Err(PlanError::MissingInput("directional pattern")),
            },
            PlanSubject::Direct => Ok(inputs.side),
        }
    }

    /// The price a market entry is measured against for the chase guard:
    /// a signal's recommended entry, a pattern's breakout level, or the last
    /// price when there is no anchor.
    fn chase_reference(inputs: &PlannerInputs) -> Decimal {
        match &inputs.subject {
            PlanSubject::Signal(signal) => signal
                .entry_price
                .map_or(inputs.last_price.value(), |price| price.value()),
            PlanSubject::ChartPattern(pattern) => pattern.breakout_level.value(),
            _ => inputs.last_price.value(),
        }
    }

    /// Resolve the raw entry price before snapping.
    fn resolve_entry(
        spec: &TradePlanSpec,
        inputs: &PlannerInputs,
        side: OrderSide,
    ) -> Result<Price, PlanError> {
        match &spec.entry {
            EntryRule::Market => Ok(inputs.last_price),
            EntryRule::LimitAt(price) => Ok(*price),
            EntryRule::StopAbove { trigger } => Ok(*trigger),
            EntryRule::StopBelow { trigger } => Ok(*trigger),
            EntryRule::OpeningRangeBreak { .. } => {
                let map = inputs
                    .level_map
                    .as_ref()
                    .ok_or(PlanError::MissingInput("level_map"))?;
                match side {
                    OrderSide::Buy => map
                        .resistance_levels()
                        .map(|level| level.band.1)
                        .min()
                        .map(Price::new)
                        .ok_or(PlanError::MissingInput("resistance level")),
                    OrderSide::Sell => map
                        .support_levels()
                        .map(|level| level.band.0)
                        .max()
                        .map(Price::new)
                        .ok_or(PlanError::MissingInput("support level")),
                }
            }
            EntryRule::PatternBreakout { buffer_ticks } => {
                let pattern = match &inputs.subject {
                    PlanSubject::ChartPattern(pattern) => pattern,
                    _ => return Err(PlanError::MissingInput("pattern")),
                };
                match pattern.stage {
                    PatternStage::FormingArmed => {
                        let buffer = Decimal::from(*buffer_ticks) * inputs.tick_size.value();
                        match side {
                            OrderSide::Buy => {
                                Ok(Price::new(pattern.breakout_level.value() + buffer))
                            }
                            OrderSide::Sell => {
                                Ok(Price::new(pattern.breakout_level.value() - buffer))
                            }
                        }
                    }
                    PatternStage::BreakoutConfirmed => Ok(inputs.last_price),
                }
            }
        }
    }

    /// Resolve the raw initial stop price before snapping.
    fn resolve_stop(
        spec: &TradePlanSpec,
        inputs: &PlannerInputs,
        side: OrderSide,
        entry: Decimal,
    ) -> Result<Price, PlanError> {
        match &spec.stop {
            StopRule::Fixed(price) => Ok(*price),
            StopRule::Percent(pct) => match side {
                OrderSide::Buy => Ok(Price::new(
                    (entry * (Decimal::ONE - *pct)).max(Decimal::ZERO),
                )),
                OrderSide::Sell => Ok(Price::new(entry * (Decimal::ONE + *pct))),
            },
            StopRule::AtrMultiple { mult } => {
                let atr = inputs.atr.ok_or(PlanError::MissingInput("atr"))?;
                let distance = *mult * atr;
                match side {
                    OrderSide::Buy => Ok(Price::new((entry - distance).max(Decimal::ZERO))),
                    OrderSide::Sell => Ok(Price::new(entry + distance)),
                }
            }
            StopRule::LevelBand {
                band_id,
                buffer_atr,
            } => {
                let map = inputs
                    .level_map
                    .as_ref()
                    .ok_or(PlanError::MissingInput("level_map"))?;
                let level = map.get(*band_id).ok_or(PlanError::MissingInput("level"))?;
                let buffer = if *buffer_atr == Decimal::ZERO {
                    Decimal::ZERO
                } else {
                    let atr = inputs.atr.ok_or(PlanError::MissingInput("atr"))?;
                    *buffer_atr * atr
                };
                match side {
                    OrderSide::Buy => Ok(Price::new((level.band.0 - buffer).max(Decimal::ZERO))),
                    OrderSide::Sell => Ok(Price::new(level.band.1 + buffer)),
                }
            }
            StopRule::Swing => match side {
                OrderSide::Buy => Ok(inputs
                    .swing_low
                    .ok_or(PlanError::MissingInput("swing_low"))?),
                OrderSide::Sell => Ok(inputs
                    .swing_high
                    .ok_or(PlanError::MissingInput("swing_high"))?),
            },
            StopRule::DayExtreme => match side {
                OrderSide::Buy => Ok(inputs.day_low.ok_or(PlanError::MissingInput("day_low"))?),
                OrderSide::Sell => {
                    Ok(inputs.day_high.ok_or(PlanError::MissingInput("day_high"))?)
                }
            },
            StopRule::Chandelier { mult, .. } => {
                let atr = inputs.atr.ok_or(PlanError::MissingInput("atr"))?;
                match side {
                    OrderSide::Buy => {
                        let highest = inputs
                            .swing_high
                            .or(inputs.day_high)
                            .ok_or(PlanError::MissingInput("swing_high"))?;
                        Ok(Price::new(
                            (highest.value() - *mult * atr).max(Decimal::ZERO),
                        ))
                    }
                    OrderSide::Sell => {
                        let lowest = inputs
                            .swing_low
                            .or(inputs.day_low)
                            .ok_or(PlanError::MissingInput("swing_low"))?;
                        Ok(Price::new(lowest.value() + *mult * atr))
                    }
                }
            }
            StopRule::PatternInvalidation { buffer_atr } => {
                let pattern = match &inputs.subject {
                    PlanSubject::ChartPattern(pattern) => pattern,
                    _ => return Err(PlanError::MissingInput("pattern")),
                };
                let base = pattern.invalidation_level.value();
                match (side, buffer_atr) {
                    (OrderSide::Buy, Some(buffer)) => {
                        let atr = inputs.atr.ok_or(PlanError::MissingInput("atr"))?;
                        Ok(Price::new((base - *buffer * atr).max(Decimal::ZERO)))
                    }
                    (OrderSide::Sell, Some(buffer)) => {
                        let atr = inputs.atr.ok_or(PlanError::MissingInput("atr"))?;
                        Ok(Price::new(base + *buffer * atr))
                    }
                    (OrderSide::Buy, None) | (OrderSide::Sell, None) => {
                        Ok(pattern.invalidation_level)
                    }
                }
            }
        }
    }

    /// Price one target leg; `Ok(None)` means the leg is skipped (measured move
    /// with no derivable height and no fallback).
    fn target_price(
        trigger: TargetTrigger,
        inputs: &PlannerInputs,
        entry: Decimal,
        risk_per_unit: Decimal,
        side: OrderSide,
    ) -> Result<Option<(Decimal, String)>, PlanError> {
        match trigger {
            TargetTrigger::AtR(r) => {
                let price = match side {
                    OrderSide::Buy => entry + r * risk_per_unit,
                    OrderSide::Sell => entry - r * risk_per_unit,
                };
                Ok(Some((price, format!("target at {r}R"))))
            }
            TargetTrigger::AtPrice(price) => Ok(Some((price.value(), "price target".to_string()))),
            TargetTrigger::AfterSessions(sessions) => Ok(Some((
                inputs.last_price.value(),
                format!("time exit after {sessions} session(s)"),
            ))),
            TargetTrigger::MeasuredMove {
                fraction_of_height,
                fallback,
            } => {
                if let Some(height) = Self::measured_move_height(inputs) {
                    let extension = fraction_of_height * height;
                    let price = match side {
                        OrderSide::Buy => entry + extension,
                        OrderSide::Sell => entry - extension,
                    };
                    Ok(Some((price, "measured move".to_string())))
                } else if let Some(fallback) = fallback {
                    Self::target_price(*fallback, inputs, entry, risk_per_unit, side)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Measured height: the pattern's breakout height, else the level map's
    /// gap between the tightest resistance band high and support band low.
    fn measured_move_height(inputs: &PlannerInputs) -> Option<Decimal> {
        if let PlanSubject::ChartPattern(pattern) = &inputs.subject {
            return Some(pattern.measured_height());
        }
        let map = inputs.level_map.as_ref()?;
        let resistance_high = map.resistance_levels().map(|level| level.band.1).min();
        let support_low = map.support_levels().map(|level| level.band.0).max();
        match (resistance_high, support_low) {
            (Some(res), Some(sup)) if res > sup => Some(res - sup),
            _ => None,
        }
    }
}

/// Snap a price conservatively: for the winning side of a fill (`toward_market`
/// is `true`) a buy snaps up and a sell snaps down; otherwise (`false`, i.e.
/// the stop or a target) the snap moves toward the entry — a stop away from the
/// entry (never loosening risk), a target toward the entry.
fn snap_to_tick(price: Decimal, tick: Decimal, side: &OrderSide, toward_market: bool) -> Decimal {
    if tick <= Decimal::ZERO {
        return price;
    }
    let snapped = match (side, toward_market) {
        (OrderSide::Buy, true) | (OrderSide::Sell, false) => (price / tick).ceil() * tick,
        (OrderSide::Sell, true) | (OrderSide::Buy, false) => (price / tick).floor() * tick,
    };
    snapped.max(Decimal::ZERO)
}

/// Lot-round a leg quantity down to a multiple of `min_lot`.
fn leg_quantity(total: Quantity, fraction: Decimal, min_lot: Decimal) -> Quantity {
    let raw = total.value() * fraction;
    let lots = (raw / min_lot).floor();
    Quantity::new((lots * min_lot).max(Decimal::ZERO))
}

fn stop_label(rule: &StopRule) -> String {
    match rule {
        StopRule::Fixed(_) => "fixed".to_string(),
        StopRule::Percent(_) => "percent".to_string(),
        StopRule::AtrMultiple { .. } => "atr_multiple".to_string(),
        StopRule::LevelBand { .. } => "level_band".to_string(),
        StopRule::Swing => "swing".to_string(),
        StopRule::DayExtreme => "day_extreme".to_string(),
        StopRule::Chandelier { .. } => "chandelier".to_string(),
        StopRule::PatternInvalidation { .. } => "pattern_invalidation".to_string(),
    }
}

fn sizing_label(method: &SizingMethod) -> String {
    match method {
        SizingMethod::FixedFractional { risk_pct } => format!("fixed_fractional({risk_pct})"),
        SizingMethod::VolatilityTarget { .. } => "volatility_target".to_string(),
        SizingMethod::KellyCriterion { .. } => "kelly_criterion".to_string(),
        SizingMethod::EqualRiskContribution => "equal_risk_contribution".to_string(),
        SizingMethod::FixedNotional { .. } => "fixed_notional".to_string(),
    }
}
