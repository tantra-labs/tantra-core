# 0008 — `Money::notional` hard-codes INR and `Money` has no currency-checked arithmetic

**Status**: open
**Filed**: 2026-11-23 — workspace design review (root plan 0021) · **Kind**: design-flaw · **Severity**: medium
**Plan**: [0006](../plans/0006-design-review-2026-11.md)

## What's stubbed or hardcoded

- `src/types.rs:486-491` `Money::notional(price, qty)` returns `currency: Currency::Inr` regardless of the instrument.
- `src/types.rs:460-465` `Money { amount, currency }` has pub fields, no `Add`/`Sub` that checks currency, and no scale/rounding policy (`MinorUnit` at `:366` exists but `Money` never quantizes to it).
- `src/instrument.rs:72` `Instrument.currency` exists (non-INR via `with_currency`, `:112`), but `notional` cannot see it.

## Why

Any USD or other non-INR instrument gets an INR-tagged notional that the risk engine compares against INR limits: silent mis-valuation of exposure and limits.

## How to close it

1. Change to `Money::notional(price, qty, currency)`; deprecate the old signature for one release.
2. Add `checked_add` / `checked_sub` returning `CoreError::CurrencyMismatch`.
3. Add `Money::quantize(RoundingStrategy)` using `MinorUnit`.
4. Make `Money` fields private with accessors, consistent with closed gap `0003` (`README.md#closed-gaps`).
5. Update callers in `tantra-risk` and `tantra-ledger` to pass the instrument currency.

**Verification**: new `tests/money_tests.rs` with `proptest` (`a + b - b == a`; a currency mismatch always errors); `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `make test`, `make prod`.

## Related

- Root gap 0014 (unified financial enums and position model).
- Closed gap `0003` (`README.md#closed-gaps`).
