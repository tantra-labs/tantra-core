# 0004 — Classification & Alert Wire Types

**Status**: Proposed

## 0. Executive Summary & Objective

Add the L0 value objects and wire contracts that every other layer needs for categorised
screening and group alerts: a 4-level NSE classification hierarchy, `MarketCapBucket`, `IndexId`,
`ScreenCategory`, `InstrumentClassification`, crossing operators and peer-relative filters on the
existing screener predicate, and `AlertRule` / `AlertEvent`. tantra-core stays pure: no I/O, no
clock, `Decimal` only, wasm-compatible, schemars schemas.

References: research [`trendlyne-screener-alerts-mcp.md`](../../../docs/research/trendlyne-screener-alerts-mcp.md)
(F1 + F7 wire); root roadmap [`0007`](../../../docs/plans/0007-screener-taxonomy-and-alerts.md).
Trendlyne is an idea source only; nothing here depends on it.

## 1. Context & Gaps Covered

Existing `src/screener.rs`: `FilterOp {Eq,Ne,Gt,Gte,Lt,Lte,Between}`, `MetricKeySpec`,
`MetricDefinition`, `MetricRef`, `ScreenerFilterPredicate{metric, op, value, value_upper}`,
`ScreenerFilterGroup{predicates, match_all}`, `SortSpec`, `ScreenerScanRequest`, `ScreenerRow`,
`ScreenerScanResponse`. There is no Sector/Industry/MarketCapBucket/IndexId type, no crossing
operators (Python `ComparisonOp` has them), and no peer-relative filter.

Covers research gaps **F1** and the wire half of **F7**.

## 2. Component Specifications & Phases

### Phase A — Classification value objects (`src/classification.rs`)
- `Sector`, `Industry`: validated newtypes (non-empty, trimmed, bounded length, canonical case);
  constructors return `Result<_, DomainError>`.
- `ClassificationPath{macro_sector, sector, industry, basic_industry}` — NSE 4-level hierarchy;
  deeper levels optional, parents required for children.
- `MarketCapBucket {Large, Mid, Small, Micro}`: SEBI/AMFI rank cutoffs 1–100 / 101–250 / 251+;
  `Micro` is below a configurable market-cap floor. `MarketCapBucket::from_rank(rank, mcap, micro_floor: Decimal)`
  is a pure helper; the ranking itself is computed by the data layer (tantra-data plan 0007).
- `IndexId` newtype (`"NIFTY50"`, `"NIFTY500"`, `"SENSEX"`), upper-case ASCII validated.
- `ScreenCategory` enum mirroring the research taxonomy rows (Expert, Quality/Value/Momentum score,
  PriceVolume, Intraday, Swing, RelativeValue, RelativeReturns, Financials, Technicals,
  MovingAverages, Shareholding, Candlesticks, RedFlags, PeBuySellZone, Other); stable `snake_case`
  wire strings that Python mirrors verbatim.
- `InstrumentClassification{instrument: InstrumentId, path, cap_bucket, indices: Vec<IndexId>, themes: Vec<String>, as_of: NaiveDate}`.

### Phase B — Screener extensions (`src/screener.rs`, backward compatible)
- `FilterOp` gains `CrossesAbove`, `CrossesBelow` (additive variants; unknown-variant policy
  documented in the schema version note).
- `ClassificationFilter{sector?, industry?, cap_bucket?, index?, theme?}` as an optional field on
  `ScreenerFilterGroup` (`#[serde(default, skip_serializing_if)]`).
- `RelativeTo {Sector, Industry, Index(IndexId)}` + optional `relative_to` on
  `ScreenerFilterPredicate`, with `relative_stat` (`Median`, `Mean`, `ReturnDelta`), enabling
  `pe < sector_median(pe)` and `return_3m > index_return(NIFTY500, 3m)`.
- Request/response carry `schema_version`; old payloads deserialize unchanged.

### Phase C — Alert wire types (`src/alert.rs`)
- `AlertRule{id, owner, universe: UniverseSpec, condition: ScreenerFilterGroup, frequency, trigger}`;
  `AlertFrequency {M15, M30, H1, D1}`; `AlertTrigger {Enter, Exit, Level}`.
- `UniverseSpec` = explicit instruments | `IndexId` | watchlist id | classification filter.
- `AlertEvent{rule_id, instrument, trigger, observed_at, metrics: BTreeMap<String, Decimal>}`;
  timestamps are injected by callers (no clock in core).
- Input DTOs use `#[serde(deny_unknown_fields)]`; all derive `JsonSchema`.

## 3. Test Plan (TDD)

Written first, in `tests/` (never inline):
- `tests/classification_tests.rs`: newtype validation, hierarchy invariants, `from_rank` boundaries
  (100/101/250/251, micro floor).
- `tests/screener_extension_tests.rs`: old golden requests still parse; new ops/filters round-trip.
- `tests/alert_wire_tests.rs`: golden JSON vectors under `tests/fixtures/`; `deny_unknown_fields` rejection.
- `tests/classification_proptest.rs`: serde round-trip, `from_rank` monotone in rank.
- Schema snapshot tests for every new type; wasm32 compile check in CI.

## 4. Dependencies & Tier Notes

L0; depends only on `rust_decimal`, `chrono`, `serde`, `schemars`, `thiserror`. Consumers:
[tantra-data 0007](../../../tantra-data/docs/plans/0007-instrument-master-index-membership.md),
[tantra-data 0008](../../../tantra-data/docs/plans/0008-screen-dsl-catalog-qvm.md),
[tantra-strategies 0002](../../../tantra-strategies/docs/plans/0002-universe-screen-context.md),
[tantra-runtime 0005](../../../tantra-runtime/docs/plans/0005-group-screener-alerts.md),
[tantra-agent 0013](../../../tantra-agent/docs/plans/0013-screener-mcp-tools.md). No floats.

## 5. Open Questions

- Pin the NSE classification vocabulary version (source file and effective date) with tantra-data 0007.
- Should `Micro` floor live in the type or only in data-layer config? (Proposed: config only.)
