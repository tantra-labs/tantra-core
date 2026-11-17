# Gap Index — `tantra-core`

Per workspace rule `.agents/rules/gap-tracking.md`:
- **Gaps** (`docs/gaps/XXXX-*.md`): individual missing features, stubs, or rule violations.
- **Plans** (`docs/plans/XXXX-*.md`): collections of gaps.
- Numbering is per-repo, sequential, never reused.

## Open Gaps

| # | Title | Severity | Status | Plan |
|---|-------|----------|--------|------|
| _(none)_ | — | — | — | — |

## Closed Gaps

| # | Title | Closed In |
|---|-------|-----------|
| 0002 | No intraday level-map / options-positioning / signal domain types (Elephant Edge EE+) | Closed 2026-11-17 (plan 0003, T12.1): `src/levels.rs` (`LevelKind`/`LevelSide`/`Level`/`LevelMap`/`LevelId`), `src/options.rs` (`OptionChainSnapshot`/`StrikePositioning`/`GammaExposure` + `gamma_flip_strike`), `src/order_blocks.rs` (`OrderBlock`), `src/signals.rs` (`SignalKind`/`StrategySignal`→`Signal`/`PositionIntent`) — Decimal-only, zero-I/O, `JsonSchema` wire contracts; `SCHEMA_VERSION` v9; tests `levels_tests`/`levels_proptest`/`options_tests`/`order_blocks_tests`/`signals_tests` + golden vectors. Stop/trail types land in plan 0005 (`src/trade_plan.rs`) per the reconciliation note |
| 0001 | Phase 2 call-site consolidation: delegate `tantra-market-indicators` / `tantra-agent-analytics` return/vol math to `tantra_core::quant` | Closed 2026-11-14 (plan 0002 Phase A audit): **no 1:1 delegation sites** — `tantra-market-indicators` is streaming f64 by policy, `tantra-agent-analytics` is statistical f64, and `tantra_core::quant` is batch Decimal; forcing delegation would change public behavior. Full matrix + verdict in `docs/design/code-quant.md` §Downstream call-site audit. Revisit only if a new Decimal-bounded quant consumer (e.g. `tantra-ledger-performance` parity vectors) appears |
