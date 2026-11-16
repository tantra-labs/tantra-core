# 0004 — Classification Value Objects & Converged Screener Wire Types

**Status**: Proposed (revised 2026-11-16 after architectural review)

## 0. Executive Summary & Objective

Add only the minimal, jurisdiction-neutral L0 value objects and converged screener wire contracts
that downstream bounded contexts share: `Sector`, `Industry`, `IndexId`, bare `MarketCapBucket`, and
one unified, recursive `Decimal` Screener DSL (`ScreenerFilterGroup` with nested `AND`/`OR`,
`CrossesAbove`/`CrossesBelow`, `RelativeTo`, `ClassificationFilter`). `tantra-core` stays pure and
slim: no I/O, no clock, `Decimal` only, wasm-compatible, `schemars` schemas.

**Explicitly excluded from `tantra-core` (L0)** per [`docs/ddd-conflict.md`](../../../docs/ddd-conflict.md)
and the 2026-11-16 design review:
- **`ScreenCategory` (catalog taxonomy)** -> owned by [`tantra-data` 0008](../../../tantra-data/docs/plans/0008-screen-dsl-catalog-qvm.md) as a validated category slug on TOML catalog presets (`ScreenPreset`), not an L0 enum.
- **`AlertRule` / `AlertEvent` / `DeliveryPolicy` / `UniverseSpec` (Alpha-Alerts wire types)** -> owned by L3 ([`tantra-runtime` 0007](../../../tantra-runtime/docs/plans/0007-group-screener-alerts.md) / `tantra-agent-chat-tools`), alongside existing `PriceAlert`.
- **SEBI/AMFI rank cutoffs (`from_rank`), NSE 4-level hierarchy validation, and editorial `themes`** -> owned by [`tantra-data` 0007](../../../tantra-data/docs/plans/0007-instrument-master-index-membership.md) / `tantra-market-finance::india` (mirroring `FeeBreakdown` vs `india::costs`).

References: research [`trendlyne-screener-alerts-mcp.md`](../../../docs/research/trendlyne-screener-alerts-mcp.md)
(F1); root roadmap [`0010`](../../../docs/plans/0010-screener-taxonomy-and-alerts.md).
Trendlyne is an idea source only; nothing here depends on it.

## 1. Context & Gaps Covered

Today the codebase has four incompatible screener representations:
1. `tantra-core/src/screener.rs`: flat `ScreenerFilterGroup { predicates, match_all }` (no nested `AND`/`OR` groups), 7 ops (`FilterOp::{Eq,Ne,Gt,Gte,Lt,Lte,Between}`), `MetricKeySpec { metric, period }`, not exported in `tantra-ffi-codegen`.
2. `tantra-runtime-api/src/screener.rs`: recursive `ScreenerFilterGroup { operator, items }`, stringly ops, `Option<Timeframe>`, ignores `tantra_core::screener`.
3. `tantra-ffi/python/tantra/wire/screener.py`: 13 ops (`neq` vs `ne`, `crosses_above/below`, `in`, `not_in`, `like`, `has`), `MetricKeySpec { key, period, timeframe }`, different `ScreenerScanRequest`.
4. `tantra-market-indicators/src/screener.rs`: recursive enum with `f64` operands (Zero-Floats violation).

This plan converges the wire contract onto one canonical recursive AST in `tantra-core` (Phase B)
and adds the minimal market-agnostic classification primitives (Phase A) so `tantra-risk`,
`tantra-ledger-allocation`, `tantra-runtime`, `tantra-ui`, and `tantra-ffi` share one schema.

Covers research gap **F1** (L0 slice only).

## 2. Component Specifications & Phases

### Phase A — Slim Classification Value Objects (`src/classification.rs`)
- `Sector`, `Industry`: validated newtypes (non-empty, trimmed, bounded length, canonical case);
  constructors return `Result<_, CoreError>`.
- `IndexId`: validated upper-case ASCII newtype (`"NIFTY50"`, `"NIFTY500"`, `"SENSEX"`, `"SP500"`).
- `MarketCapBucket { Large, Mid, Small, Micro }`: bare enum (`snake_case`) with **no**
  jurisdiction-specific rank cutoff helper in L0 (SEBI/AMFI rank rules live in
  `tantra-data` 0007 / `tantra-market-finance::india`).
- All derive `JsonSchema`, `Serialize`, `Deserialize` (strict validation on input).

### Phase B — Screener DSL Convergence & Extensions (`src/screener.rs`, backward-compatible migration)
- **Recursive filter tree**: replace/upgrade flat `ScreenerFilterGroup { predicates, match_all }`
  into recursive `ScreenerFilterGroup { operator: LogicalOp (And | Or), items: Vec<ScreenerFilterItem> }`
  where `ScreenerFilterItem` is `Predicate(ScreenerFilterPredicate) | Group(Box<ScreenerFilterGroup>)`
  (retaining a deserialization shim or `schema_version` bump for v8 flat payloads).
- **Unified `FilterOp`**: `Eq`, `Ne` (canonical; alias `"neq"` accepted during migration), `Gt`, `Gte`,
  `Lt`, `Lte`, `Between`, `In`, `NotIn`, `Like`, `Has`, `CrossesAbove`, `CrossesBelow`.
- **Dimensions & operands**: `MetricKeySpec { key, period: Option<MetricPeriod>, timeframe: Option<Timeframe> }`;
  `Decimal`-safe `PredicateValue` (no floats: `Decimal`, `String`, `bool`, `MetricRef`, list/bounds).
- **Classification & peer-relative filters**:
  - Optional `ClassificationFilter { sector?, industry?, cap_bucket?, index?, theme? }` on
    `ScreenerFilterGroup` (`#[serde(default, skip_serializing_if)]`; `theme` is an opaque string tag).
  - `RelativeTo { Sector, Industry, Index(IndexId) }` + `RelativeStat { Median, Mean, ReturnDelta }`
    as optional fields on `ScreenerFilterPredicate`, enabling `pe < sector_median(pe)` and
    `return_3m > index_return(NIFTY500, 3m)`.
- **Codegen & downstream cleanup**: export `ScreenerScanRequest`, `ScreenerScanResponse`,
  `ScreenerFilterGroup`, `ScreenerFilterPredicate`, `MetricDefinition`, and classification VOs from
  `tantra-ffi-codegen`; migrate `tantra-runtime-api/src/screener.rs`, `tantra-ffi/python/tantra/wire/screener.py`,
  and `tantra-market-indicators/src/screener.rs` (retiring its `f64` fork) onto these canonical types.

## 3. Test Plan (TDD)

Written first, in `tests/` (never inline):
- `tests/classification_tests.rs`: `Sector`, `Industry`, `IndexId`, `MarketCapBucket` validation & serde round-trip.
- `tests/screener_extension_tests.rs`: nested `AND`/`OR` recursion, legacy flat-group migration fixture,
  `CrossesAbove`/`CrossesBelow`, `RelativeTo`, `ClassificationFilter`, `Decimal`-only operand rejection of floats/NaN.
- `tests/classification_proptest.rs`: serde round-trip and validation invariants.
- Schema snapshot tests in `tantra-core` + `tantra-ffi-codegen` parity test; `wasm32-unknown-unknown` compile check in CI.

## 4. Dependencies & Tier Notes

L0; depends only on `rust_decimal`, `chrono`, `serde`, `schemars`, `thiserror`. Consumers:
[tantra-data 0007](../../../tantra-data/docs/plans/0007-instrument-master-index-membership.md),
[tantra-data 0008](../../../tantra-data/docs/plans/0008-screen-dsl-catalog-qvm.md),
[tantra-strategies 0002](../../../tantra-strategies/docs/plans/0002-universe-screen-context.md),
[tantra-runtime 0007](../../../tantra-runtime/docs/plans/0007-group-screener-alerts.md),
[tantra-agent 0013](../../../tantra-agent/docs/plans/0013-screener-mcp-tools.md). No floats, no I/O, no clock.

## 5. Open Questions

- Confirm deprecation window for `FilterOp` `"neq"` alias and legacy flat `ScreenerFilterGroup { predicates, match_all }` in `tantra-ui-*` fixtures.
- Confirm whether `tantra-risk` / `tantra-ledger-allocation` need `Sector`/`MarketCapBucket` in L0 immediately or only after Phase A lands.
