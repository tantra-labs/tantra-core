# 0005 — Trade Planner & Trade Manager (entry / SL / TSL / scale-out / sizing)

**Status**: Proposed

## 0. Executive Summary & Objective

Today [`Signal`](../src/signal.rs) carries optional `entry_price`, `stop_loss`, `take_profit`,
and `volatility_atr` but nothing computes them, and [`Decision::PlaceOrder`](../src/decision.rs) has no stop/target.
This plan adds a pure, deterministic **TradePlanner** (signal/pattern → sized plan) and **TradeManager**
(bar-by-bar state machine: fill re-anchoring, move stop, partial exit, exit) driven by **canonical, versioned presets**
(`qullamaggie`, `ee_plus`, `atr_swing`, `pattern_breakout`). One Rust implementation drives backtest, Python FFI, and live runtime.
Master roadmap: [root plan 0008](../../../docs/plans/0008-trade-signal-generator.md).

## 1. Context & Gaps Covered

- Reuses `SizingMethod`, `RiskBudget`, `PositionSizingRequest/Result`, `calculate_position_size` (`src/sizing.rs`). Extends `RiskBudget` / `PositionSizingRequest` with optional `max_weight_pct`, `max_adv_pct`, and `adv20: Option<Quantity>` so L1 `tantra-risk` (`validate_sizing_request`) enforces weight and liquidity caps consistently.
- `OrderType` (`src/order.rs`) has no bracket/trailing; `TradePlan` drives `OrderRequest` intents and synthetic OCO in `tantra-runtime` until `tantra-exec/docs/gaps/0003` and `tantra-ledger/docs/gaps/0012` land. Keep [`Decision`](../src/decision.rs) slim (`PlaceOrder` / `ModifyOrder` / `CancelOrder`); do not bloat `Decision` before bracket execution lands.
- **Unification with [plan 0003](0003-intraday-level-map-domain-types.md)**: Plan `0003` owns `LevelMap` / `Level` / `OptionChainSnapshot` / `OrderBlock` (EE+ level inputs). Stop/trail rules (`StopRule`, `TrailRule`, `TargetLadder`, `TradePlan`) and signal unification (`PlanSubject` accepting `tantra_core::Signal`, `StrategySignal` + `LevelMap`, or `ChartPattern`) live here in `0005` so `tantra-core` has **one** stop/trail hierarchy (`StopProtection` in `0003` is collapsed/aliased to `TradePlan` stop/trail specs).
- `tantra-risk/docs/gaps/0005` (SL/TSL validation) consumes `TradePlan` / `TradePlanSpec` rather than a separate `StopProtection` struct.
- Root plans 0005 (EE+), 0006 (Qullamaggie), and 0008 §3 (Pattern-driven) define the preset rules.

## 2. Component Specifications & Phases

All types: `Decimal`-only, `schemars::JsonSchema`, `#[serde(deny_unknown_fields)]` on input/spec DTOs, no I/O, no wall clock (timestamps injected),
WASM-compatible. New modules: `src/trade_plan.rs`, `src/trade_manager.rs`, `src/chart_pattern.rs`, and embedded canonical TOML presets. Bump `SCHEMA_VERSION` (`v8` → `v9`) when adding wire contracts.

- `EntryRule { Market, LimitAt(Price), StopAbove{trigger}, StopBelow{trigger}, OpeningRangeBreak{minutes}, PatternBreakout{buffer_ticks} }` + symmetric chase guard (`max_extension_atr`, long/short aware).
- `StopRule { Fixed(Price), Percent(Decimal), AtrMultiple{mult}, LevelBand{band, buffer_atr}, Swing, DayExtreme /*DayLow for long, DayHigh for short*/, Chandelier{period, mult}, PatternInvalidation{buffer_atr} }`
  with `max_stop_distance` guard (absolute or `max_stop_adr_mult × adr_pct` evaluated cleanly in `%`: `|entry - sl| / entry × 100 <= mult × adr_pct`).
- `TrailRule { Off, Atr{mult}, Swing, Chandelier{period, mult}, CloseMa{kind: Sma|Ema, period}, Percent(Decimal) }` (long exits on close below MA, short on close above MA)
  + `activation_after_r`, `breakeven_after: Option<BreakevenTrigger{AtR(Decimal)|AfterSessions(u32)|AfterFirstTarget}>`,
  `time_stop: Option<{bars, min_progress_r, timeframe}>`.
- `TargetLadder { legs: Vec<TargetLeg{trigger: AtR(Decimal)|AtPrice|AfterSessions(u32)|MeasuredMove{fraction_of_height, fallback?}, fraction}>, runner }`; fractions sum `<= 1`;
  quantity split respects `min_lot_size`, remainder allocated to runner.
- `SizingSpec` = `SizingMethod` + caps (`max_weight_pct`, `max_adv_pct`), enforcing `|entry - initial_stop| > 0` (`PlanError::ZeroRisk` — failing closed rather than triggering `calculate_position_size`'s zero-stop fallback!) before delegating to `calculate_position_size`.
- `TradePlanSpec { preset_id, version, entry, stop, trail, targets, sizing, min_reward_risk? }` (versioned, TOML/JSON-loadable; all fields overridable).
- `PlannerInputs { subject: PlanSubject /* Signal | StrategySignal | ChartPattern | Direct */, last_price, tick_size: Price, atr?, adr_pct?, swing_low/high?, day_low/high?, ma_values, adv20?, level_map?, pattern?, timestamp }` —
  all computed prices (`entry`, `initial_stop`, `targets`) are snapped conservatively to `tick_size` (never loosening risk).
- `TradePlanner::plan(&TradePlanSpec, &PlannerInputs, &RiskBudget) -> Result<TradePlan, PlanError>`;
  `TradePlan { instrument, side, entry, initial_stop, risk_per_unit, quantity, risk_at_stake, targets: Vec<PlannedTarget{price, qty}>, trail, reward_risk, spec_version, rationale }`;
  `PlanError { StopTooWide, ZeroRisk, BelowMinLot, ChaseGuard, InsufficientReward, MissingInput(..), PatternInvalidated }`.
- `TradeManager`: explicit separation of lifecycle & timeframe events:
  - `on_entry_fill(&mut self, fill_price: Price, fill_qty: Quantity, ts: DateTime<Utc>)`: re-anchors `risk_per_unit = |fill_price - initial_stop|`, R-multiple ladder prices, and breakeven level to actual fill price (or emits `ExitAll` / flags risk breach if gap-fill collapses/violates `min_reward_risk`).
  - `on_bar(&mut self, bar: Bar, snapshot) -> Vec<TradeAction>` for intrabar high/low price triggers (`MoveStop{new}`, `PartialExit{qty, reason}`, `ExitAll{reason}`); intrabar policy configurable, default conservative (stop before target when both inside high/low). Stop never loosens (`stop` monotone).
  - `on_session_close(&mut self, timeframe: Timeframe, bar: Bar, snapshot) -> Vec<TradeAction>` for close-only rules (`CloseMa`, `AfterSessions`, `time_stop`), emitting exit actions executed strictly on the **next** bar/session (no look-ahead).

### 2a. Pattern-driven plans (`ChartPattern`; detection in `tantra-market` plan 0002)

- New wire type `src/chart_pattern.rs`:
  - `PatternStage { FormingArmed, BreakoutConfirmed }`: separates pre-breakout armed setups (VCP/flag/cup waiting on a resting `StopAbove`/`StopBelow` breakout trigger) from breakout-close-confirmed patterns (double bottom neckline break, multi-bar candlesticks).
  - `ChartPattern { kind: PatternKind, direction: SignalDirection, timeframe: Timeframe, stage: PatternStage, pivots: Vec<PatternPivot{bar_ts: DateTime<Utc>, price: Price, kind}>, breakout_level: Price, invalidation_level: Price, measured_target: Option<Price>, quality: u8 /*0..=100*/, formed_from: DateTime<Utc>, armed_at: DateTime<Utc>, confirmed_at: Option<DateTime<Utc>> }`.
  - `PatternKind` covers flag, pennant, triangle (asc/desc/sym), rectangle, Darvas box, double top/bottom, head and shoulders (and inverse), wedge, cup-and-handle, VCP, and multi-bar candlestick kinds.
  - All prices are `Decimal`; constructors validate geometry: `invalidation_level` sits on the losing side of `breakout_level`, `measured_target` (when `Some`) sits on the winning side, and `formed_from <= armed_at`.
- `PlannerInputs.pattern: Option<ChartPattern>`.
- `EntryRule::PatternBreakout { buffer_ticks: u32 }`: uses `StopAbove`/`StopBelow` at `breakout_level ± buffer_ticks × tick_size` when `stage == FormingArmed`, or market/pullback limit entry when `stage == BreakoutConfirmed` (avoiding missed entries after breakout close).
- `StopRule::PatternInvalidation { buffer_atr: Option<Decimal> }`.
- `TargetLeg.trigger` adds `MeasuredMove { fraction_of_height: Decimal, fallback: Option<Box<TargetTrigger>> }` (default `fraction_of_height = 1.0`). When `measured_target` is `None` (e.g. VCP, candlesticks), `MeasuredMove` uses `fallback` (or omits the fixed leg so the trade runs trail-only rather than failing with `MissingInput`).
- `min_reward_risk` checks `measured_target` R against `|entry - initial_stop|`, returning `PlanError::InsufficientReward` if below minimum.
- New preset `pattern_breakout`: pattern entry, pattern-invalidation SL, ½ at `MeasuredMove(1.0)` (fallback `AtR(2.0)` / runner-only when targetless), runner on Chandelier trail.
- Pattern `quality` / edge stats from `tantra-market` plan `0002` Phase D may scale `risk_pct` only after Phase D publishes versioned stats.

### Phases

- **P1**: Unify stop/signal contracts (`0003` + `0005`), `RiskBudget` weight/ADV fields, `ChartPattern`, `TradePlanSpec`/`TradePlan`/`PlanError`, `TradePlanner` (tick-snapping, `ZeroRisk` guard).
- **P2**: `TradeManager` state machine (`on_entry_fill`, `on_bar`, `on_session_close`, persistent serde state).
- **P3**: Canonical embedded TOML presets (`qullamaggie`, `ee_plus`, `atr_swing`, `pattern_breakout`), golden vectors, and `schemars` export (single source of truth consumed by FFI, strategies, runtime, and agent).

## 3. Test Plan (TDD)

Tests only in `tests/trade_plan_tests.rs`, `tests/trade_manager_tests.rs`, `tests/trade_plan_proptest.rs`, `tests/trade_plan_golden.rs`, `tests/chart_pattern_tests.rs` (red first):
- proptest: `risk_at_stake <= equity x risk_pct`; qty is lot multiple and obeys weight/ADV caps; tick alignment; stop monotone (never loosens); ladder qty sum `<= position`; no look-ahead (`on_session_close` exits do not fill on the same bar).
- Golden vectors per preset (TOML preset → `TradePlan` JSON; bar/fill sequence → `TradeAction` list), including gap-through-stop, `on_entry_fill` R-re-anchoring, same-bar stop+target, and VCP targetless fallback.
- Negative: `StopTooWide` (qullamaggie stop > 1 × ADR%), `ChaseGuard`, `BelowMinLot`, `ZeroRisk`, `InsufficientReward`, `MissingInput`, `PatternInvalidated`.

## 4. Dependencies & Tier Notes

L0 only: `rust_decimal`, `chrono`, `serde`, `schemars`, `thiserror`.
Consumers: `tantra-risk` (gap 0005), `tantra-exec` (gap 0003), `tantra-ledger` (gap 0012), `tantra-market` (plan 0002), `tantra-runtime` (plan 0006), `tantra-ffi` (plan 0002), `tantra-agent` (plan 0014).
Indicator gap: `tantra-market/docs/gaps/0007` (ADR%, Chandelier).

## 5. Decisions Locked (Previously Open)

- **`Decision` grammar**: Keep `Decision` slim until `tantra-exec` gap `0003` closes; plans convert into risk-checked `OrderRequest` intents carrying SEBI `algo_id` (`TradeManager` / synthetic OCO manages child orders).
- **Preset SSOT**: Canonical preset files are embedded/validated TOMLs in `tantra-core` (with JSON as the wire form); `tantra-ffi` exposes them to `tantra-strategies` and Python notebooks.
