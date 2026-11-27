# `tantra-core` — DDD Architecture & Shared Kernel Specification

## 1. Bounded Context & Responsibility

`tantra-core` is the **Shared Kernel (Tier 0)** of the Tantra Labs
ecosystem. Every Rust crate and the Python FFI boundary (`tantra-ffi`) depend
on `tantra-core`, while `tantra-core` depends on **zero internal crates** and
performs **zero I/O** (no network, no filesystem, no clock queries — timestamps
are explicit `DateTime<Utc>` parameters).

The kernel owns the ubiquitous language:

- Strongly-typed aggregate IDs — `ClientOrderId`, `AlgoId`, `AccountId`, `TransactionId`.
- Exact-decimal financial value objects — `Price`, `Quantity`, `Money`, `Currency`, `MinorUnit`.
- Market-agnostic classification value objects — `Sector`, `Industry`, `IndexId`, `MarketCapBucket`.
- Canonical instrument, market, and order contracts — `InstrumentId`, `Tick`, `Bar`, `OrderRequest`, `OrderAck`, `FillReport`.
- The converged recursive `Decimal` Screener DSL (`ScreenerFilterGroup`, `ScreenerFilterPredicate`, `ClassificationFilter`, `RelativeTo`, `RelativeStat`).
- The closed AI `Decision` grammar and authoritative `DomainEvent`s.
- Hexagonal port traits, the trading calendar, dynamic position sizing grammar, and the statistical validation gates.
- Single source of truth for wire contracts: every wire type derives `JsonSchema`; OpenAPI/JSON/MCP schemas are generated from them (`make schema` here; OpenAPI via `tantra-ffi-codegen` / `tantra-runtime`).

## 2. Strict Invariants

| Invariant | Enforcement |
|-----------|-------------|
| **Zero-I/O domain purity** | No `tokio`, `std::fs`, `std::net`, or `SystemTime`/`Utc::now()` in `src/`; all transitions take explicit `DateTime<Utc>`. |
| **Zero-floats policy** | No `f32`/`f64` anywhere in `src/`; all financial math is `rust_decimal::Decimal` (with the `maths` feature — `ln`/`sqrt` computed in pure `Decimal`). |
| **Zero inline tests** | No `#[cfg(test)]` / `#[test]` inside `src/`; all tests live in `tests/` parallel to `src/` (parallel test directory rule). |
| **Wire-contract generation** | All wire types derive `JsonSchema` via `schemars`; generated schema files are never hand-edited. |
| **Retargetability** | Compiles to `wasm32-unknown-unknown` and native `x86_64` / `aarch64`; no OS-specific dependencies. |

## 3. Module Map

```
tantra-core/src/
├── lib.rs            # Re-exports + SCHEMA_VERSION (v10) + MIN_SUPPORTED_SCHEMA_VERSION + REST_API_VERSION (v2) / API_VERSION (legacy v1)
├── error.rs          # CoreError taxonomy
├── ids.rs            # ClientOrderId, AlgoId, AccountId, TransactionId
├── types.rs          # Exchange, InstrumentId, Price, Quantity, Money, PriceBand, Currency, CountryCode, MinorUnit
├── classification.rs # Sector, Industry, IndexId, MarketCapBucket (slim market-agnostic VOs)
├── instrument.rs     # Instrument / InstrumentProfile, InstrumentKind, OptionType, PositionSide, TradeId, VenueOrderId, DatasetId
├── market.rs         # Tick (Ltp | Quote | Depth), DepthLevel, Bar
├── order.rs          # OrderSide, OrderType, TimeInForce, OrderStatus, OrderRequest, ModifyRequest, CancelRequest, OrderAck, FillReport, FeeBreakdown, RejectReason, OrderEvent, OrderEventKind
├── decision.rs       # Closed Decision enum + validate()/from_json_str()
├── events.rs         # DomainEvent, ExecutionEvent, TradingState, Reason, ReasonGroup, ReasonPolarity
├── wire.rs           # TraceContext (W3C traceparent)
├── calendar.rs       # SessionWindow + TradingCalendar port
├── portfolio.rs      # PortfolioSnapshot, PortfolioPosition, PortfolioHolding, PortfolioGreeks, PortfolioTotals, PortfolioAccount
├── screener.rs       # Recursive ScreenerScanRequest/Response, ScreenerFilterGroup/Item/Predicate, FilterOp, MetricPeriod, MetricKeySpec, PredicateValue, ClassificationFilter, RelativeTo, RelativeStat, ScreenerRow, SortSpec
├── signal.rs         # Signal, SignalDirection, SignalHorizon, Timeframe (M1..Month1), SignalScanRequest/Response
├── ports.rs          # Hexagonal port traits (BarReader, TickReader, ..., PositionSizer) + PortError/PortResult
├── sizing.rs         # SizingMethod, RiskBudget, PositionSizingRequest/Result, calculate_position_size
├── quant.rs          # QuantSeries + canonical quant transforms (plan 0001)
└── analytics/        # ValidationBlock + 6 statistical gates (DSR, PBO, WFE, holdout, Monte Carlo, cost stress)
```

Reference docs:

| Module | Doc |
|--------|-----|
| `types.rs` | [code-types](code-types.md) |
| `ids.rs`, `error.rs` | [code-ids](code-ids.md) |
| `classification.rs`, `instrument.rs`, `market.rs`, `order.rs`, `portfolio.rs`, `screener.rs`, `signal.rs`, `calendar.rs` | [code-wire-contracts](code-wire-contracts.md) |
| `decision.rs` | [code-decision](code-decision.md) |
| `events.rs`, `wire.rs` | [code-events](code-events.md) |
| `ports.rs` | [code-ports](code-ports.md) |
| `quant.rs` | [code-quant](code-quant.md) |
| `sizing.rs`, `analytics/` | [code-sizing-analytics](code-sizing-analytics.md) |
| Tier 0 implementation record | [code-migration-map](code-migration-map.md) |

## 4. Schema & Versioning

- `SCHEMA_VERSION: u32 = 10` — the JSON wire contract version owned by this
  crate (changelog + bump policy in `lib.rs`); `MIN_SUPPORTED_SCHEMA_VERSION`
  = N-2 bounds the additive-only window.
- `REST_API_VERSION: &str = "v2"` — canonical REST prefix; `API_VERSION = "v1"`
  is the legacy alias kept for `tantra-ffi`.
- `WireEnvelope<T>` (`wire.rs`) — `{ schema_version, payload }`, rejects versions
  outside the window.
- `schema/domain/domain_schema.json` — generated by `schema::domain_schema()`
  (`make schema`; CI `make schema-check`).
- `schema/compatibility/v{N}_domain_schema.json` — frozen snapshots (`make
  schema-freeze`); v7/v8 are hand-written history, v10 is the first generated one.
  See `schema/compatibility/README.md`.
- `schema/golden/v{N}/<Type>__<case>.json` — per-version payload corpus.
- `schema/golden/*.json` — golden serialization vectors (`tick_ltp`,
  `tick_quote`, `price_band`, `decision_*`).

## 5. Implementation Status

### Implemented & Verified

| Area | Module(s) | Tests |
|------|-----------|-------|
| Value objects + identifiers | `types.rs`, `ids.rs`, `error.rs` | `tests/core_tests.rs`, `tests/proptest_invariants.rs` |
| Classification value objects (plan 0004) | `classification.rs` | `tests/classification_tests.rs` (6), `tests/classification_proptest.rs` (4) |
| Market / order / instrument wire contracts | `market.rs`, `order.rs`, `instrument.rs` | `tests/core_tests.rs`, `tests/golden_test.rs` |
| Closed `Decision` grammar | `decision.rs` | `tests/core_tests.rs`, `tests/golden_test.rs` |
| Domain events + trace context | `events.rs`, `wire.rs` | `tests/core_tests.rs` |
| Ports + calendar | `ports.rs`, `calendar.rs` | `tests/core_tests.rs` |
| Sizing grammar | `sizing.rs` | `tests/core_tests.rs` |
| Quant transforms (plan 0001) | `quant.rs` | `tests/quant_tests.rs` (33), `tests/quant_proptest.rs` |
| Statistical validation gates | `analytics/` | `tests/analytics_gates_tests.rs` |
| Converged Screener DSL + signals + portfolio (plan 0004) | `screener.rs`, `signal.rs`, `portfolio.rs` | `tests/core_tests.rs`, `tests/screener_extension_tests.rs` (7) |
| Schema generation, compatibility & golden vectors | `schema.rs`, `wire.rs`, `schema/` | `tests/schema_compat_recursive.rs`, `tests/golden_test.rs` |

### Verification Gate

```bash
make prod      # cargo build --workspace --profile prod
make test      # cargo test --workspace --profile prod
make check     # cargo check --workspace --profile prod
make clippy    # cargo clippy --workspace --all-targets --profile prod -- -D warnings
make fmt       # cargo fmt
```

### What Remains

- Call-site consolidation (`gap 0001` / `plan 0002`) was closed on audit (see
  [code-quant](code-quant.md) and the [plans index](../plans/README.md#implemented-plans)).
- Remaining active work items are tracked in the [gaps](../gaps/README.md) and
  [plans](../plans/README.md) indexes (`0003` intraday level-map / EE+ domain
  types and `0005` trade planner & manager).
