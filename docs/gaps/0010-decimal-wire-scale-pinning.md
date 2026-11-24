# 0010 — Inline `Decimal` serialization is not pinned to a canonical representation

**Status**: open
**Filed**: 2026-11-23 — workspace design review (root plan 0021) · **Kind**: deployment · **Severity**: medium
**Plan**: [0006](../plans/0006-design-review-2026-11.md)

## What's stubbed or hardcoded

- `Cargo.toml:9` enables the `rust_decimal` feature `serde-with-str`, but `rg "serde(with" src` / `rust_decimal::serde` returns nothing. Plain `Decimal` fields (e.g. `Instrument.tick_size` `src/instrument.rs:79`, `Money.amount` `src/types.rs:463`, `PriceBand.band_pct`) use the crate default, so string-vs-number depends on feature unification across the dependency graph, while `Price`/`Quantity` use custom visitors.
- Scale is not normalised: `dec!(1.50)` and `dec!(1.5)` serialise differently but compare equal, so golden bytes and idempotency hashes (SHA-256 prompt replay, audit WAL) differ across producers.

Confidence: medium.

## Why

A hash-chained audit WAL or prompt replay computed in Rust vs Python (`decimal.Decimal`) can disagree on scale, and a Python/TS consumer may receive a number or a string depending on feature unification.

## How to close it

1. Define `mod decimal_wire` (`serialize_str` / `deserialize_str_or_num`, rejecting floats).
2. Apply `#[serde(with = ...)]` and `#[schemars(with = "String")]` on every bare `Decimal` field.
3. Decide and document normalization (`normalize()` vs preserve scale).
4. Add golden vectors (`1.50`, `1E+2`, `-0`) and a Python parity vector in `tantra-ffi`.

**Verification**: new `tests/decimal_wire_tests.rs` plus `proptest` round-trip; regenerate `schema/domain/domain_schema.json` via `make codegen` and freeze per [0004](0004-wire-schema-evolution-gate.md); `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `make test`, `make prod`.

## Related

- Root gap 0015 (pricing and quant math single source) and root gap 0018 (lakehouse manifest/ref byte-compat fixtures).
- [0004](0004-wire-schema-evolution-gate.md), closed gap `0003` (`README.md#closed-gaps`).
