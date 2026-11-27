# 0009 — `Bar` lacks interval, OHLC invariants and unambiguous timestamp semantics

**Status**: open
**Filed**: 2026-11-23 — workspace design review (root plan 0021) · **Kind**: design-flaw · **Severity**: medium
**Plan**: [0006](../plans/0006-design-review-2026-11.md)

## What's stubbed or hardcoded

- `src/market.rs:117-132` `Bar { instrument, open, high, low, close, volume, timestamp }`:
  - no `Timeframe` field;
  - the doc says "start or end timestamp (UTC)" (ambiguous);
  - no `high >= max(open, close)` / `low <= min(open, close)` check;
  - pub fields and derived `Deserialize`.
- `Timeframe` exists at `src/signal.rs:19` but is not attached; `BarRequest` (`src/ports.rs`) passes it separately, so a `Vec<Bar>` is not self-describing.

## Why

A bar-open vs bar-close stamping mismatch between brokers produces a one-bar look-ahead in backtests (a same-bar signal executes at that bar's close), the classic backtest bias.

## How to close it

1. Add `interval: Timeframe` to `Bar` (consider moving `Timeframe` to `market.rs`; see plan 0006 R2).
2. Define and document the timestamp convention: bar OPEN time, UTC, half-open interval.
3. Add `Bar::try_new` and `#[serde(try_from)]` enforcing OHLC ordering and non-negative volume.
4. Provide golden vectors per broker normaliser.
5. Coordinate the provider migration in `tantra-market-store`, which retires its duplicate `Bar` (known root 0019 P4.3).

**Verification**: `proptest` for OHLC ordering and golden JSON in a new `tests/bar_tests.rs`; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `make test`, `make prod`; bump `SCHEMA_VERSION` and freeze the snapshot per closed gap `0004` (`README.md#closed-gaps`, `schema/compatibility/README.md`).

## Related

- Extends root plan 0019 P4.3 (retire downstream `Bar` duplicate).
- Closed gap `0003` (`README.md#closed-gaps`), closed gap `0004` (`README.md#closed-gaps`, `schema/compatibility/README.md`).
