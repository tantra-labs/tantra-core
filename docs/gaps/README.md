# Gap Index — `tantra-core`

Per workspace rule `.agents/rules/gap-tracking.md`:
- **Gaps** (`docs/gaps/XXXX-*.md`): individual missing features, stubs, or rule violations.
- **Plans** (`docs/plans/XXXX-*.md`): collections of gaps.
- Numbering is per-repo, sequential, never reused.

## Open Gaps

| # | Title | Severity | Status | Plan |
|---|---|---|---|---|
| [0004](0004-wire-schema-evolution-gate.md) | Wire-schema compatibility gate covers only v7/v8 top-level keys; no snapshot for v10 | high | open | [0006](../plans/0006-design-review-2026-11.md) |
| [0005](0005-typed-port-errors.md) | `PortError` stringly-typed, no `unavailable`, `cancel` takes `&str` | medium | open | [0006](../plans/0006-design-review-2026-11.md) |
| [0006](0006-canonical-priceband-and-calendar-trait.md) | Duplicated `PriceBand` and calendar traits across four repos | medium | open | [0006](../plans/0006-design-review-2026-11.md) |
| [0007](0007-calendar-session-model.md) | `TradingCalendar` fixed offset, single unvalidated session, no day navigation | medium | open | [0006](../plans/0006-design-review-2026-11.md) |
| [0008](0008-money-currency-safety.md) | `Money::notional` hard-codes INR; no currency-checked arithmetic | medium | open | [0006](../plans/0006-design-review-2026-11.md) |
| [0009](0009-bar-invariants-and-interval.md) | `Bar` lacks interval, OHLC invariants and timestamp semantics | medium | open | [0006](../plans/0006-design-review-2026-11.md) |
| [0010](0010-decimal-wire-scale-pinning.md) | Inline `Decimal` serialization not pinned to a canonical representation | medium | open | [0006](../plans/0006-design-review-2026-11.md) |

## Closed Gaps

| # | Title | Closed In |
|---|---|---|
| 0003 | Value objects constructible in invalid states (`Price`, `Quantity`, `PriceBand`, `Instrument`, `InstrumentId`) | Closed 2026-11-24 (plan 0006 Phase A #1): private inner fields with validated `try_new` / `try_from_parts` constructors and `#[serde(try_from = "...")]` invariants on `Price` (`>= 0`, `get()`/`as_decimal()`), `Quantity` (`>= 0`, `get()`/`as_decimal()`), `PriceBand` (`lower <= reference_price <= upper`, `0 < band_pct <= 1`), `InstrumentId` (non-empty trimmed ASCII-uppercase symbol normalization across `new`/`try_new`/serde, `symbol()` accessor), and `Instrument` (`tick_size > 0`, `lot_size > 0`, `try_new`); verified in `tests/value_object_invariants.rs` (unit + proptest + golden negative JSON) and migrated downstream L1–L5 workspaces (`tantra-risk`, `tantra-exec`, `tantra-ledger`, `tantra-market`, `tantra-agent`, `tantra-runtime`, `tantra-ffi` incl. `make py` 521 pytest, `tantra-ui`, `tantra-runtime-api-rest/schema/openapi.v2.json` + `tantra-ui-web/src/lib/api-schema.d.ts`) under `--profile prod` and `wasm32-unknown-unknown`. |
| 0002 | No intraday level-map / options-positioning / signal domain types (Elephant Edge EE+) | Closed 2026-11-17 (plan 0003, T12.1): `src/levels.rs` (`LevelKind`/`LevelSide`/`Level`/`LevelMap`/`LevelId`), `src/options.rs` (`OptionChainSnapshot`/`StrikePositioning`/`GammaExposure` + `gamma_flip_strike`), `src/order_blocks.rs` (`OrderBlock`), `src/signals.rs` (`SignalKind`/`StrategySignal`→`Signal`/`PositionIntent`) — Decimal-only, zero-I/O, `JsonSchema` wire contracts; `SCHEMA_VERSION` v9; tests `levels_tests`/`levels_proptest`/`options_tests`/`order_blocks_tests`/`signals_tests` + golden vectors. Stop/trail types land in plan 0005 (`src/trade_plan.rs`) per the reconciliation note |
| 0001 | Phase 2 call-site consolidation: delegate `tantra-market-indicators` / `tantra-agent-analytics` return/vol math to `tantra_core::quant` | Closed 2026-11-14 (plan 0002 Phase A audit): **no 1:1 delegation sites** — `tantra-market-indicators` is streaming f64 by policy, `tantra-agent-analytics` is statistical f64, and `tantra_core::quant` is batch Decimal; forcing delegation would change public behavior. Full matrix + verdict in `docs/design/code-quant.md` §Downstream call-site audit. Revisit only if a new Decimal-bounded quant consumer (e.g. `tantra-ledger-performance` parity vectors) appears |
