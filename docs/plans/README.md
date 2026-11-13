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
| [0002](0002-call-site-consolidation.md) | Call-Site Consolidation: Delegate Downstream Quant Math to `tantra_core::quant` | `tantra-core`, `tantra-market-indicators`, `tantra-agent-analytics` | Ready — tracks gap 0001 |
| [0003](0003-intraday-level-map-domain-types.md) | Intraday Level-Map, Options-Positioning & Stop-Protection Domain Types (Elephant Edge EE+) | `tantra-core` | Proposed — tracks gap 0002; master plan root `docs/plans/0005` |

## Implemented Plans

| # | Title | Target Scope | Implemented In |
|---|-------|--------------|----------------|
| 0001 | Canonical Quant Transform Primitives (Returns, Volatility, Rolling Stats) | `src/quant.rs` | `QuantSeries`, `simple_returns`, `log_returns`, `realized_volatility`, `ewma_volatility`, `rolling_mean`, `rolling_stdev`, `sharpe_ratio`, `max_drawdown`, `correlation`, `beta`, `historical_var`, `historical_cvar`, `tracking_error`, and `QuantError` in `src/quant.rs`; exported from `lib.rs`; golden-vector tests in `tests/quant_tests.rs` (33) and property-based invariants in `tests/quant_proptest.rs`; commits `9ce4880` + `43a11f0`. |
