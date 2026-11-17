# Tier 0 Migration Map & Invariant Checklist

Every Tier 0 deliverable has landed in `tantra-core` and is verified:

1. **Aggregate IDs & error taxonomy** — `src/ids.rs`, `src/error.rs`
2. **Value objects & domain primitives** — `src/types.rs` (`Exchange`, `InstrumentId`, `Price`, `Quantity`, `Money`, `PriceBand`, `Currency`, `CountryCode`, `MinorUnit`), `src/classification.rs` (`Sector`, `Industry`, `IndexId`, `MarketCapBucket`)
3. **Wire contracts** — `src/instrument.rs`, `src/market.rs`, `src/order.rs` (incl. `FeeBreakdown`), `src/portfolio.rs`, `src/screener.rs`, `src/signal.rs`, `src/calendar.rs`
4. **AI `Decision` & `DomainEvent`s** — `src/decision.rs`, `src/events.rs`, `src/wire.rs` (`TraceContext`)
5. **Hexagonal ports** — `src/ports.rs` (`BarReader`, `TickReader`, `OrderBookReader`, `QuoteReader`, `DepthReader`, `InstrumentMaster`, `MarketDataConnector`, `ExecutionGateway`, `PositionSizer`)
6. **Sizing & statistical gates** — `src/sizing.rs`, `src/analytics/` (`dsr`, `pbo`, `walk_forward`, `holdout`, `monte_carlo`, `cost_stress`, `ValidationBlock`)
7. **Quant transform primitives** — `src/quant.rs` (plan 0001: 12 transform functions + `QuantSeries` + `QuantError`, 33 golden-vector tests, property tests)

## Crate-Topology Consolidation Impact

`tantra-core` was **unchanged** structurally during root plan 0003 crate-topology
cleanup: its role as the zero-dependency Tier 0 shared kernel was already
compliant. `tantra-runtime-feed` (absorbed into `tantra-runtime-api-rest::market_feed`)
retains local `f64`/columnar market-feed types (`bar.rs`), while `tantra-core::Bar`
remains canonical everywhere else; see the root crate-topology ADR.

## Invariant Compliance

All invariants from [architecture.md §2](architecture.md#2-strict-invariants)
hold across `tantra-core`:

- [x] **Zero-I/O**: no `tokio`, `std::fs`, `std::net`, or `Utc::now()` in `src/`.
- [x] **Zero-floats**: zero `f32`/`f64` fields in any financial struct.
- [x] **Zero inline tests**: no `#[cfg(test)]` / `#[test]` inside `src/`.
- [x] **WASM retargetable**: compiles to `wasm32-unknown-unknown`.

## Verification Commands

```bash
# Run from the repository root
make prod      # cargo build --workspace --profile prod
make test      # cargo test --workspace --profile prod
make check     # cargo check --workspace --profile prod
make clippy    # cargo clippy --workspace --all-targets --profile prod -- -D warnings
make fmt       # cargo fmt
```

## Remaining Work

See the [gaps](../gaps/README.md) (`gap 0001` / `plan 0002` closed on audit —
see [code-quant](code-quant.md)) and [plans](../plans/README.md) indexes.
