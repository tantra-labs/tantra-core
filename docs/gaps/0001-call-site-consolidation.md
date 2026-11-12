# Gap 0001 — Phase 2 Call-Site Consolidation: `tantra_core::quant` Delegation

**Status**: open — tracked by [plan 0002](../plans/0002-call-site-consolidation.md)

## What's Stubbed or Duplicated

Plan 0001 implemented `src/quant.rs` with canonical quant transform primitives
(`QuantSeries`, `simple_returns`, `log_returns`, `realized_volatility`,
`ewma_volatility`, `rolling_mean`, `rolling_stdev`, `sharpe_ratio`,
`max_drawdown`, `correlation`, `beta`, `historical_var`, `historical_cvar`,
`tracking_error`, `QuantError`). These are exported from `lib.rs` with
golden-vector tests (`tests/quant_tests.rs`, 33 tests) and property-based
invariants (`tests/quant_proptest.rs`).

However, **downstream crates still re-derive these transforms locally** instead
of delegating to `tantra_core::quant`:

- `tantra-market-indicators` computes SMA/EMA/RSI/etc. with its own local math.
- `tantra-agent-analytics` computes Sharpe/drawdown/volatility with its own local math.

This duplication is the "Phase 2 call-site consolidation" recorded as future
work when plan 0001 was closed. Until consolidated, a bug fix or precision
change in one crate does not propagate to the others, violating the workspace
"Consolidation Over Duplication" invariant (`.agents/AGENTS.md` §4.1).

## Why It Matters

- **Consolidation over duplication**: The same mathematical transforms exist in
  multiple bounded contexts, creating drift risk.
- **Zero-floats consistency**: Downstream local math may inadvertently introduce
  `f64` intermediates; `tantra_core::quant` guarantees pure `Decimal` (via
  `rust_decimal`'s `maths` feature).
- **Deterministic replay**: Only one canonical implementation ensures
  byte-for-byte identical results across backtesting and live paths.

## How to Close It

1. Audit `tantra-market-indicators/src/*.rs` and
   `tantra-agent-analytics/src/*.rs` for local implementations of returns,
   volatility, rolling stats, Sharpe, and drawdown.
2. Replace local math with calls to `tantra_core::quant` functions (add the
   `tantra-core` dependency where missing).
3. Add integration tests asserting parity between `tantra_core::quant` outputs
   and downstream indicator outputs on shared golden vectors.
4. Update `docs/design/code-quant.md` with the delegation matrix.

## Verification

`cargo test --workspace --profile prod` passes across `tantra-core`,
`tantra-market-indicators`, and `tantra-agent-analytics`; no local
re-derivation of the listed transforms remains downstream.
