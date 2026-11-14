# `tantra-core` — Canonical Quant Transform Primitives Reference

Documents `tantra-core/src/quant.rs`: the single canonical home for pure,
zero-I/O numeric transforms over exact `Decimal` values. Downstream crates
(`tantra-market-indicators`, `tantra-agent-analytics`,
`tantra-runtime-backtest`, and the future `tantra-ledger-allocation`) should
delegate to these rather than re-deriving locally. The Phase A audit (below)
found **no applicable delegation sites** in the current downstream crates, so
gap 0001 and plan 0002 were **closed 2026-11-14 on that decision**; delegation
remains a revisit-on-new-consumer item rather than a rewrite.

## `QuantSeries`

```rust
pub struct QuantSeries(Vec<Decimal>);
```

An ordered series of exact decimal values (price path, equity curve, or returns
series). Thin newtype over `Vec<Decimal>` with conversions from `&[Price]`,
`&[Money]`, `Vec<Decimal>`, and `&[Decimal]`.

## Transform Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `simple_returns` | `(&QuantSeries) -> Result<QuantSeries>` | `(p[t] - p[t-1]) / p[t-1]`, n-1 outputs |
| `log_returns` | `(&QuantSeries) -> Result<QuantSeries>` | `ln(p[t] / p[t-1])`, n-1 outputs |
| `realized_volatility` | `(&QuantSeries, Decimal) -> Result<Decimal>` | Sample stdev × sqrt(annualization) |
| `ewma_volatility` | `(&QuantSeries, Decimal) -> Result<Decimal>` | RiskMetrics EWMA, lambda in (0,1) |
| `rolling_mean` | `(&QuantSeries, usize) -> Result<QuantSeries>` | Fixed-window SMA |
| `rolling_stdev` | `(&QuantSeries, usize) -> Result<QuantSeries>` | Fixed-window sample stdev |
| `sharpe_ratio` | `(&QuantSeries, Decimal, Decimal) -> Result<Decimal>` | Annualized Sharpe |
| `max_drawdown` | `(&QuantSeries) -> Result<Decimal>` | Max peak-to-trough decline |
| `correlation` | `(&QuantSeries, &QuantSeries) -> Result<Decimal>` | Pearson correlation |
| `beta` | `(&QuantSeries, &QuantSeries) -> Result<Decimal>` | Asset beta vs benchmark |
| `historical_var` | `(&QuantSeries, Decimal) -> Result<Decimal>` | Empirical VaR (nearest-rank) |
| `historical_cvar` | `(&QuantSeries, Decimal) -> Result<Decimal>` | Empirical CVaR / Expected Shortfall |
| `tracking_error` | `(&QuantSeries, &QuantSeries, Decimal) -> Result<Decimal>` | Vol of return differences |

All transforms are exported from `lib.rs`.

## `QuantError`

| Variant | Trigger |
|---------|---------|
| `InsufficientData { needed, got }` | Not enough observations |
| `DivisionByZero` | Zero prior price, zero variance, zero peak |
| `WindowLargerThanSeries { window, len }` | Rolling window > series length |
| `InvalidArgument(&'static str)` | Bad argument (zero window, bad lambda, out-of-range annualization, mismatched lengths) |

## Zero-Floats Note

All math is pure `Decimal`. `log_returns` uses `rust_decimal`'s `maths` feature
(`MathematicalOps::ln` / `MathematicalOps::sqrt`), computed in pure `Decimal`
(192-bit `DecWide` intermediates, no `f64`). There is **no `f32`/`f64`
anywhere in this file**, including as intermediates.

## Determinism

Replaying the same series produces byte-for-byte identical results. The module
is zero-I/O: no network, filesystem, or clock queries.

## Tests

- `tantra-core/tests/quant_tests.rs` — 33 golden-vector tests.
- `tantra-core/tests/quant_proptest.rs` — property-based invariants.

## Downstream call-site audit (plan 0002 Phase A — 2026-11-13)

Audit of the two crates named by gap 0001 for local re-derivations of the transforms above.
**Finding: there are no 1:1 delegation sites** — neither crate re-derives a transform with
compatible semantics and type policy, so no in-place replacement was made. Delegation remains
a "revisit when a new Decimal-bounded quant consumer appears" item, not a mechanical rewrite.

**Decision (2026-11-14)**: gap 0001 + plan 0002 closed on this audit — the downstream crates
are f64-internal by documented policy, and forcing delegation would change public behavior.
Delegation stays open only as a future wiring point when a Decimal-bounded quant consumer
(e.g. `tantra-ledger-performance` parity vectors) appears.

| Downstream site | Local math | quant equivalent | Delegation verdict |
|---|---|---|---|
| `tantra-market-indicators/src/sma.rs` `Sma::update` | streaming O(1) ring-buffer SMA over `f64` | `rolling_mean` (batch, `Decimal`) | Not applicable — stateful streaming indicator (f64 by policy); batch/Decimal delegate would break O(1) updates and change warm-up semantics |
| `tantra-market-indicators/src/ema.rs`, `rma.rs`, `rsi.rs`, `atr.rs`, `bollinger.rs`, `stochastic.rs` | streaming EWMA / rolling stats over `f64` | `rolling_stdev` / `ewma_volatility` (batch, `Decimal`) | Not applicable — same streaming/batch + type-policy mismatch |
| `tantra-agent-analytics/src/math.rs` `mean`, `variance` | batch f64 sample stats | `rolling_mean` / `rolling_stdev` (windowed, `Decimal`) | Not applicable — different shape (plain vs windowed) and type policy |
| `tantra-agent-analytics/src/equity_stats.rs` `EquityStats::from_returns` | f64 annualized vol, annualized risk-free-adjusted Sharpe, Sortino, Calmar, max drawdown | `sharpe_ratio`, `max_drawdown` (non-annualized, `Decimal`) | Not a drop-in — extra annualization/sortino/calmar semantics; f64-internal compute policy documented |
| `tantra-agent-analytics/src/{dsr,monte_carlo,pbo,validation,cointegration,volatility}.rs` | f64 statistical gates, GARCH | none overlapping | Not applicable — no quant equivalents |

**Why**: `tantra_core::quant` is pure-`Decimal`, batch, zero-I/O; the two downstream crates are
f64-internal by documented policy (`market-indicators` streaming, `agent-analytics`
statistical). Forcing delegation would require converting both crates' compute policy and
changing public behavior — out of scope for a consolidation gap. If a future consumer needs
Decimal-bounded performance stats (e.g. `tantra-ledger-performance` parity vectors), that is the
place to wire `quant::sharpe_ratio`/`max_drawdown` with parity tests.

## Related

- Gap 0001 / Plan 0002 — call-site consolidation to delegate downstream math here.
- `tantra-core/src/analytics/` — statistical gates that build on similar math
  (see [code-sizing-analytics](code-sizing-analytics.md)).
