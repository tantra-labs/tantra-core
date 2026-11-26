# 0006 — tantra-core design review (2026-11): value-object invariants, wire-schema evolution & canonical types

**Status**: In progress (2026-11-25 — Phase A #1–#2 / gaps 0003, 0005 closed)
**Scope**: `tantra-core` · **Umbrella**: root `docs/plans/0021-workspace-design-review-2026-11.md`

## 0. Executive Summary & Objective

A spot-check review of `tantra-core` found no clock, I/O, float or `Result<_, String>` violations in `src/`. It did find that the L0 value objects can be built in invalid states (pub fields, `debug_assert`-only validation), that the wire-compat gate only covers v7/v8 top-level keys while `SCHEMA_VERSION` is 10, and that several canonical types (`PortError`, `PriceBand`, calendar, `Money`, `Bar`) are duplicated or under-specified.

This plan makes the kernel self-validating, makes wire evolution CI-enforced, and removes the duplicate canonical types downstream. Non-goals: analytics consolidation (root plan 0019 P4.2) and the architecture-verification gate (root plan 0019 P4.6) stay owned by root 0019; this plan only extends them where noted.

## 1. Context & Gaps Covered

| Gap | Title | Kind | Severity |
|---|---|---|---|
| 0003 (closed — see [gaps README](../gaps/README.md#closed-gaps)) | Value objects constructible in invalid states | design-flaw | high (closed 2026-11-24) |
| [0004](../gaps/0004-wire-schema-evolution-gate.md) | Wire-schema compatibility gate (v7/v8 only, top-level keys) | deployment | high |
| 0005 (closed — see [gaps README](../gaps/README.md#closed-gaps)) | `PortError` stringly-typed, no `unavailable`, `cancel(&str)` | design-flaw | medium (closed 2026-11-25) |
| [0006](../gaps/0006-canonical-priceband-and-calendar-trait.md) | Duplicated `PriceBand` and calendar traits | duplication | medium |
| [0007](../gaps/0007-calendar-session-model.md) | Calendar session model (fixed offset, single window) | design-flaw | medium |
| 0008 | `Money::notional` hard-codes INR — **closed 2026-11-26** (see [gaps index](../gaps/README.md)) | design-flaw | medium |
| [0009](../gaps/0009-bar-invariants-and-interval.md) | `Bar` interval, OHLC invariants, timestamp semantics | design-flaw | medium |
| [0010](../gaps/0010-decimal-wire-scale-pinning.md) | Decimal wire representation not pinned | deployment | medium |

Plan-only items (no standalone gap doc):

- **R1 — Feature flags, chrono default features, path deps, licence** — `Cargo.toml:9-14` makes `schemars` and `serde_json` unconditional; chrono default features (`clock`, `std`, `wasmbind`) leave `Utc::now` available in L0; siblings use path deps (e.g. tantra-ledger `Cargo.toml:24`); `license = "AGPL-3.0"` (`Cargo.toml:20`); `[profile.*]` blocks in the crate manifest (`Cargo.toml:25-60`) are ignored for dependents. Owned jointly with root plan 0023 and root gap 0026 (frame as provenance/attribution, non-commercial project).
- **R2 — Overlapping `signal.rs` / `signals.rs` and flat re-export sprawl** — `src/signal.rs:151` `Signal` vs `src/signals.rs:1` `StrategySignal`; `signals.rs:17` `ELEPHANT_EDGE_SIGNAL_GENERATOR` const in L0; `Timeframe` (`src/signal.rs:19`) used by market/levels/options; `src/lib.rs:28-51` 24 `pub mod` plus flat `pub use` (`:53-120`) and ~100 lines of version-history prose (`:122-165`).
- **R3 — Non-deterministic id generation in L0** — `src/order.rs:197` `OrderRequest::new` calls `Uuid::new_v4()`; `Cargo.toml:14` `v4`/`rng-getrandom`, `Cargo.toml:19-21` wasm getrandom shim; `OrderAck.client_order_id: Uuid` (`src/order.rs:271`) vs `ClientOrderId` in `src/ids.rs`. Links root gap 0010.
- **R4 — Oversized modules and duplicated numeric kernels** — `src/trade_plan.rs` (969 lines), `src/screener.rs` (927), `src/quant.rs` (514), `src/sizing.rs` (446); `DecWide`/`ln` in `src/quant.rs:25-27` vs erf/CDF in `src/analytics/distribution.rs:13` and `dsr.rs:13`; private `snap_to_tick` (`src/trade_plan.rs:930`) is the only tick-rounding helper.
- **R5 — Remaining stringly-typed identifiers** — `src/instrument.rs:128,147,166` `DatasetId`/`TradeId`/`VenueOrderId` pub-inner `String`; `OrderRequest::new(..., algo_id: impl Into<String>)` (`src/order.rs:~190`) while `AlgoId` exists (`src/ids.rs`); `InstrumentId.token`/`isin: Option<String>` (`src/types.rs:84`) with no ISIN format/Luhn check. Links root gaps 0010 and 0013 (the SEBI algo-id compliance control).

## 2. Component Specifications & Phases

### Phase A — Quick, low-risk fixes
1. **0003 (Closed 2026-11-24)** — private inner fields, `try_new`, `try_from` deserialization for `Price`, `Quantity`, `PriceBand`, `InstrumentId`, `Instrument`. Tests first: `tests/value_object_invariants.rs`.
2. **0005 (Closed 2026-11-25)** — `PortError::unavailable`, `kind()`/`is_retryable()`, typed `cancel(&VenueOrderId)`, `AccountId` in `PortfolioAccount`. Tests: `tests/ports_error_tests.rs` + `schema/golden/port_error_schema_golden.json`.
3. ~~**0008**~~ (closed 2026-11-26, `a1448f5`) — `Money::notional(price, qty, currency)`, checked arithmetic, `quantize`.
4. **R3** — remove `OrderRequest::new`, drop `v4`/`rng-getrandom` and the getrandom shim, migrate `OrderAck.client_order_id` to `ClientOrderId`.
5. **R5** — `OrderRequest` takes `AlgoId`; `Isin::try_new`; private inners with `try_new` for the three id newtypes.

**Gate**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `cargo test -p tantra-core --test value_object_invariants --profile prod`, `make test`, `make prod`.

### Phase B — Structural refactors / de-duplication
1. **0007** — session model (`SessionKind`, `sessions_for_date`, half-open windows, day navigation).
2. **0006** — delete market-finance `PriceBand`; `MarketCalendar` over `TradingCalendar`; chat-tools `CalendarReader` becomes a composition-root adapter. Consumer changes land in tantra-market, tantra-agent and tantra-runtime after the core release.
3. **0009** — `Bar` with `interval`, OPEN-time convention, `try_new`; migrate `tantra-market-store` `Bar`.
4. **0010** — `mod decimal_wire`, apply to every bare `Decimal`, pin normalization, golden vectors.
5. **R2** — move `Timeframe`, rename `signals.rs` to `strategy_signal.rs` with a deprecated re-export, trim `SCHEMA_VERSION` prose into `docs/design/`, and add a `tests/public_api.rs` path snapshot.
6. **R4** — extract `num/` (`DecWide`, `ln`, `exp`, `erf`, `RoundingPolicy`); promote `snap_to_tick` to `Instrument::{round_price_to_tick, round_qty_to_lot}`; split `trade_plan.rs` and `screener.rs`. Golden vectors in `tests/quant_tests.rs` and `tests/analytics_gates_tests.rs` must stay byte-identical.

**Gate**: Phase A gate plus `cargo test -p tantra-core --test calendar_tests --profile prod`, `--test bar_tests`, `--test decimal_wire_tests`, `--test quant_tests`, `--test analytics_gates_tests`; per touched downstream repo `make check` and `make test`.

### Phase C — Deployment readiness
1. **0004** — freeze v9/v10 snapshots, recursive schema diff, auto-enumerated DTO coverage, `WireEnvelope<T>`, single `API_VERSION` policy, N-1/N-2 golden corpora, `domain_schema.json` regeneration check.
2. **R1** — `default-features = false` on chrono (`serde`, `alloc`), features `schema` and `json`, `clippy.toml` `disallowed_methods` for `Utc::now`, CI matrix (`--no-default-features`, `--target wasm32-unknown-unknown`), versioned/git-tag pinning for consumers, move profiles to the workspace root.

**Gate**: `cargo check --target wasm32-unknown-unknown --profile prod`, `cargo check --no-default-features --profile prod`, `cargo test -p tantra-core --test schema_compat_recursive --profile prod`, `make codegen` leaves a clean `git diff --exit-code schema/`, `make prod`.

## 3. Relationship to Other Plans

- Not duplicated: root plan 0019 P4.2 (analytics consolidation) and P4.6 (architecture/CI verification gate). R4 only unifies the numeric kernels; R1 and 0004 add the wasm and regeneration checks to the P4.6 gate rather than creating a parallel one.
- Extends root plan 0019 P4.3 (downstream redefinitions) via 0005, 0006 and 0009.
- Feeds root plan 0022 (independent version rollout; 0004) and root plan 0023 (CI, versioning, licensing/SBOM; 0004, R1).
- Root gaps: 0012 (0005), 0019 (0006, 0007), 0020 (0007), 0014 (0008), 0015 and 0018 (0010), 0010 (R3, R5), 0013 (R5), 0026 (R1).

## 4. Exit Criteria

1. ~~No value object in `src/types.rs`, `src/instrument.rs` or `src/market.rs` can be constructed or deserialized into an invalid state (`tests/value_object_invariants.rs` green).~~ (`src/types.rs`, `src/instrument.rs` done in closed 0003; `Bar` tracked in 0009.)
2. `schema/compatibility/` holds a snapshot for every released `SCHEMA_VERSION`, and CI fails on any breaking recursive-diff change.
3. `PortError::unavailable` exists and no downstream crate re-defines `PortError`, `PriceBand` or a calendar trait.
4. `Money` arithmetic is currency-checked; `notional` takes an explicit currency.
5. `rg "Uuid::new_v4|Utc::now" src` is empty and enforced by build/lint config.
6. `cargo check --target wasm32-unknown-unknown` passes in `make verify-architecture`.
7. All Rust gates pass and `make prod` is current; closed gaps are removed and indexed per `.agents/rules/gap-tracking.md`.
