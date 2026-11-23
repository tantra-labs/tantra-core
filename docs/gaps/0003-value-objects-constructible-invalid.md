# 0003 — Value objects constructible in invalid states (Price, Quantity, PriceBand, Instrument, InstrumentId)

**Status**: open
**Filed**: 2026-11-23 — workspace design review (root plan 0021) · **Kind**: design-flaw · **Severity**: high
**Plan**: [0006](../plans/0006-design-review-2026-11.md)

## What's stubbed or hardcoded

Value objects expose public fields and/or validate only under `debug_assert!`, so invalid values can be built by struct literal, `new`, or serde.

- `src/types.rs:140` `pub struct Price(pub Decimal)` and `src/types.rs:262` `pub struct Quantity(pub Decimal)`: the tuple field is public, so `Price(dec!(-1))` bypasses `try_new`.
- `src/types.rs:149-157` `Price::new` is a `const fn` guarded only by `debug_assert!`; a negative price is accepted silently in release builds.
- `src/types.rs:499-505` `PriceBand` has pub fields, a `new()` with no `lower <= upper` or reference-in-band validation, and derives `Deserialize` (no validation). `band_pct: Option<Decimal>` is documented as a fraction ("0.05") but is unvalidated.
- `src/instrument.rs:77-79` `lot_size: Quantity` and `tick_size: Decimal` are pub and unvalidated; `tick_size <= 0` would divide by zero or loop in `snap_to_tick` (`src/trade_plan.rs:930`).
- `src/types.rs:80-90` `InstrumentId` derives `Deserialize` with pub `symbol: String`; `new()` uppercases but deserialization and struct literals do not, so `NSE:reliance` != `NSE:RELIANCE` as BTreeMap/Hash keys. An empty symbol is allowed.

## Why

The workspace rule says value objects validate in their constructors (`.agents/rules/coding-and-ddd.md` §1.3). A negative price from internal arithmetic, or an un-uppercased symbol from a broker adapter, flows past `RiskEngine` and splits ledger keys. A `PriceBand` with `lower > upper` makes `contains()` always false, so every order is rejected (fail-closed, but silent).

## How to close it

1. Make the inner fields of `Price` and `Quantity` private; expose `get()` / `as_decimal()`. Keep `.0` via a deprecated accessor for one release.
2. Replace `Price::new` with `try_new`; keep an unchecked constructor `pub(crate)` only.
3. Add `PriceBand::try_new` and `#[serde(try_from = "PriceBandRaw")]`; validate `lower <= upper`, reference inside band, and `band_pct` as a fraction.
4. Give `InstrumentId` a custom `Deserialize` that normalises and validates the symbol (non-empty, ASCII upper).
5. Add `Instrument::try_new` rejecting `tick_size <= 0` and `lot_size <= 0`.

**Verification**: new `tests/value_object_invariants.rs` with `proptest` (any `Decimal` never yields an invalid object via constructors or serde) plus negative golden JSON vectors; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `make prod`.

## Related

- Prerequisite for [0006](0006-canonical-priceband-and-calendar-trait.md) (market-finance reuses the validated `PriceBand`).
- Related: [0009](0009-bar-invariants-and-interval.md) (same constructor/`try_from` pattern for `Bar`).
- Plan 0006 R5 (stringly-typed ids) shares the private-inner/`try_new` approach.
