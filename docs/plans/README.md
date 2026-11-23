# Plans Index — `tantra-core`

Per workspace rule `.agents/rules/gap-tracking.md`:
- **Gaps** (`docs/gaps/XXXX-*.md`): individual missing features, stubs, or rule violations.
- **Plans** (`docs/plans/XXXX-*.md`): collections of gaps.
- When implemented, plan documents are cleaned up from `docs/plans/` and recorded in this index.
- Numbering is per-repo, sequential, never reused.

The former `docs/roadmap/` directory has been dismantled into this index: the
completed Tier 0 milestones (value objects, wire contracts, golden vectors,
quant primitives, analytics gates) live in `docs/design/` (architecture + module
references + implementation record); the only remaining roadmap item
(call-site consolidation) is gap 0001, tracked by plan 0002.

## Active Plans

| # | Title | Target Scope | Status |
|---|-------|--------------|--------|
| [0005](0005-trade-planner-and-manager.md) | Trade Planner & Trade Manager (entry/SL/TSL/scale-out/sizing, named presets) | `tantra-core` | **P1 Implemented 2026-11-17** (`src/trade_plan.rs` + `src/chart_pattern.rs` + `TradePlanner` + `RiskBudget` weight/ADV; `SCHEMA_VERSION` v10 — T12/TSG prerequisite); P2 `TradeManager` + P3 presets remain — root plan `docs/plans/0008` |
| [0006](0006-design-review-2026-11.md) | tantra-core design review (2026-11): value-object invariants, wire-schema evolution & canonical types | `tantra-core` | Proposed (2026-11-23) — gaps 0003–0010 + R1–R5 |

## Implemented Plans

| # | Title | Target Scope | Implemented In |
|---|-------|--------------|----------------|
| 0003 | Intraday Level-Map, Options-Positioning & Signal Domain Types (Elephant Edge EE+) | `src/levels.rs`, `src/options.rs`, `src/order_blocks.rs`, `src/signals.rs` | Closed 2026-11-17 (T12.1): `LevelKind`/`LevelSide`/`Level`/`LevelMap`/`LevelId`, `OptionChainSnapshot`/`StrikePositioning`/`GammaExposure` + `gamma_flip_strike`, `OrderBlock`, `SignalKind`/`StrategySignal` (→`Signal`)/`PositionIntent`; `SCHEMA_VERSION` v9; tests + golden vectors. Stop/trail live in plan 0005 `src/trade_plan.rs` (reconciliation). |
| 0001 | Canonical Quant Transform Primitives (Returns, Volatility, Rolling Stats) | `src/quant.rs` | `QuantSeries`, `simple_returns`, `log_returns`, `realized_volatility`, `ewma_volatility`, `rolling_mean`, `rolling_stdev`, `sharpe_ratio`, `max_drawdown`, `correlation`, `beta`, `historical_var`, `historical_cvar`, `tracking_error`, and `QuantError` in `src/quant.rs`; exported from `lib.rs`; golden-vector tests in `tests/quant_tests.rs` (33) and property-based invariants in `tests/quant_proptest.rs`; commits `9ce4880` + `43a11f0`. |
| 0002 | Call-Site Consolidation: Delegate Downstream Quant Math to `tantra_core::quant` | `tantra-core`, `tantra-market-indicators`, `tantra-agent-analytics` | Closed 2026-11-14 (Phase A audit): no 1:1 delegation sites — streaming f64 indicators and f64 statistical gates vs batch-Decimal `quant`; Phases B–C not applicable. Verdict + matrix in `docs/design/code-quant.md`; gap 0001 closed. |
| 0004 | Classification Value Objects & Converged Screener Wire Types (`Sector`/`Industry`/`IndexId`/`MarketCapBucket`, recursive Screener DSL) | `src/classification.rs`, `src/screener.rs`, `src/signal.rs`, `tantra-ffi`, `tantra-market`, `tantra-runtime`, `tantra-ui` | 2026-11-17 (root plan `0010` P1 / F1): slim jurisdiction-neutral classification VOs in `src/classification.rs`; converged recursive `Decimal` Screener DSL (`LogicalOp`, 13 `FilterOp` variants, `MetricPeriod`, `MetricKeySpec`, `PredicateValue` rejecting floats/NaN, `ClassificationFilter`, `RelativeTo`, `RelativeStat`, `ScreenerFilterGroup`/`Item`, v8 compat shims) in `src/screener.rs`; expanded `Timeframe::{H2, H4, Month1}` in `src/signal.rs`; `tests/classification_tests.rs` (6), `tests/classification_proptest.rs` (4), `tests/screener_extension_tests.rs` (7); exported in `tantra-ffi-codegen` + aligned Python SDK (`tantra.wire.screener`) + downstream Rust migrations (`tantra-market-indicators`, `tantra-runtime-api`, `tantra-ui`). |
