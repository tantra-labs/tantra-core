# 0005 — Trade Planner & Trade Manager (entry / SL / TSL / scale-out / sizing)

**Status**: Proposed

## 0. Executive Summary & Objective

Today a `Signal` (`src/signal.rs`) carries optional `entry_price`, `stop_loss`, `take_profit`,
`volatility_atr` but nothing computes them, and `Decision::PlaceOrder` has no stop/target.
This plan adds a pure, deterministic **TradePlanner** (signal -> sized plan) and **TradeManager**
(bar-by-bar state machine: move stop, partial exit, exit) driven by **named, versioned presets**
(`qullamaggie`, `ee_plus`, `atr_swing`). One implementation drives backtest and live.
Master roadmap: [root plan 0008](../../../docs/plans/0008-trade-signal-generator.md).

## 1. Context & Gaps Covered

- Reuses `SizingMethod`, `RiskBudget`, `PositionSizingRequest/Result`, `calculate_position_size` (`src/sizing.rs`).
- `OrderType` (`src/order.rs`) has no bracket/trailing; the plan emits `Decision::PlaceOrder` + stop order intents and
  synthetic OCO is runtime-managed until `tantra-exec/docs/gaps/0003` and `tantra-ledger/docs/gaps/0012` land.
- Level-band types for `ee_plus` come from [plan 0003](0003-intraday-level-map-domain-types.md).
- `tantra-risk/docs/gaps/0005` (SL validation) should consume `TradePlan` rather than raw `stop_loss`.
- Root plans 0005 (EE+) and 0006 (Qullamaggie) define the preset rules.

## 2. Component Specifications & Phases

All types: `Decimal` only, `schemars`, `#[serde(deny_unknown_fields)]` on inputs, no I/O, no clock (timestamps injected),
wasm32-compatible. New module `src/trade_plan.rs` (+ `trade_manager.rs`).

- `EntryRule { Market, LimitAt(Price), StopAbove{trigger}, OpeningRangeBreak{minutes} }` + chase guard (skip if moved > k x ATR).
- `StopRule { Fixed(Price), Percent(Decimal), AtrMultiple{mult}, LevelBand{band, buffer_atr}, Swing, DayLow, DayHigh, Chandelier{period, mult} }`
  with `max_stop_distance` guard (e.g. <= 1 x ADR%).
- `TrailRule { Off, Atr{mult}, Swing, Chandelier{period, mult}, CloseBelowMa{kind: Sma|Ema, period}, Percent(Decimal) }`
  + `activation_after_r`, `breakeven_after: Option<BreakevenTrigger{AtR(Decimal)|AfterBars(u32)|AfterFirstTarget}>`,
  `time_stop: Option<{bars, min_progress_r}>`.
- `TargetLadder { legs: Vec<TargetLeg{trigger: AtR(Decimal)|AtPrice|AfterBars(u32), fraction}>, runner }`; fractions sum <= 1;
  quantity split respects lot size, remainder to runner.
- `SizingSpec` = `SizingMethod` + caps (`max_weight_pct`, `max_adv_pct`), delegating to `calculate_position_size`.
- `TradePlanSpec { preset_id, version, entry, stop, trail, targets, sizing }` (versioned, TOML/JSON-loadable; all fields overridable).
- `PlannerInputs { signal, last_price, atr?, adr_pct?, swing_low/high?, day_low/high?, ma_values, adv20?, timestamp }` —
  indicator values are computed by the caller (core cannot depend on `tantra-market`).
- `TradePlanner::plan(&TradePlanSpec, &PlannerInputs, &RiskBudget) -> Result<TradePlan, PlanError>`;
  `TradePlan { instrument, side, entry, initial_stop, risk_per_unit, quantity, risk_at_stake, targets: Vec<PlannedTarget{price, qty}>, trail, reward_risk, spec_version, rationale }`;
  `PlanError { StopTooWide, ZeroRisk, BelowMinLot, ChaseGuard, MissingInput(..) }`.
- `TradeManager`: `on_bar(&mut self, bar{o,h,l,c,ts}, snapshot) -> Vec<TradeAction{MoveStop{new}, PartialExit{qty, reason}, ExitAll{reason}, None}>`.
  Intrabar policy configurable, default conservative (stop before target when both inside high/low). Stop never loosens.

Phases: **P1** DTOs + `TradePlanner` (+ errors, sizing reuse); **P2** `TradeManager` state machine; **P3** presets as data
(`qullamaggie`, `ee_plus`, `atr_swing`) + golden vectors + `schemars` schema export.

## 3. Test Plan (TDD)

Tests only in `tests/trade_plan_tests.rs`, `tests/trade_manager_tests.rs`, `tests/trade_plan_proptest.rs`, `tests/trade_plan_golden.rs` (red first).
- proptest: `risk_at_stake <= equity x risk_pct`; qty multiple of lot; stop monotone (never loosens); ladder qty sum <= position; no look-ahead (actions at bar t use data <= t).
- Golden vectors per preset (spec TOML -> plan JSON; bar series -> action list), incl. gap-through-stop and same-bar stop/target.
- Negative: StopTooWide (qullamaggie stop > 1 x ADR%), ChaseGuard, BelowMinLot, MissingInput.

## 4. Dependencies & Tier Notes

L0 only: `rust_decimal`, `serde`, `schemars`, `thiserror`. Consumers: runtime (plan 0006), ffi (plan 0002), agent (plan 0014).
Indicator gap: `tantra-market/docs/gaps/0007` (ADR%, Chandelier).

## 5. Open Questions

- Should `Decision` gain a typed `PlaceBracket`, or stay order-intents-only until exec gap 0003?
- Spec file format of record: TOML (SDK catalog) with JSON as wire form?
