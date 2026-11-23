# 0005 — `PortError` is stringly-typed, lacks `unavailable`, and `cancel` takes `&str`

**Status**: open
**Filed**: 2026-11-23 — workspace design review (root plan 0021) · **Kind**: design-flaw · **Severity**: medium
**Plan**: [0006](../plans/0006-design-review-2026-11.md)

## What's stubbed or hardcoded

- `src/ports.rs:27-40`: `ConnectionFailed(String)`, `NotFound(String)`, `Unsupported(String)`, `CalculationFailed(String)`. `rg unavailable src` finds nothing, although `.agents/rules/interface-first-and-modularity.md` §1.3 requires `PortError::unavailable(...)` paired with a gap doc.
- `src/ports.rs:166` `fn cancel(&mut self, order_id: &str)` takes a raw string instead of `ClientOrderId` / `VenueOrderId`.
- `src/order.rs:269,333` `OrderAck.order_id: String`; `src/portfolio.rs:99` `account_id: String` although `AccountId` exists (`src/ids.rs`).
- The variant doc at `src/ports.rs:39` says "sizing failure" for a generic `CalculationFailed` (copy/paste of sizing semantics).
- `PortError` derives only `Debug, PartialEq, Eq`: no `Serialize`/`JsonSchema`, no `retryable()` or error code, so downstream (`tantra-market-store` `ports.rs`, known 0019 P4.3) re-defines it.

## Why

Adapters cannot distinguish retryable from permanent failures without parsing strings. The gap-tracking rule cannot be satisfied for missing capabilities because the constructor does not exist, which pushes callers toward fake empty responses (root gap 0012).

## How to close it

1. Add `#[non_exhaustive]` `PortError::Unavailable { capability: &'static str, gap_ref: &'static str }` plus a `PortError::unavailable(...)` constructor.
2. Add `kind() -> PortErrorKind` (`Retryable` / `Permanent` / `Unavailable`) and `is_retryable()`.
3. Derive `Serialize` + `JsonSchema` for the wire-facing form.
4. Change `cancel` to take `&ClientOrderId` / `&VenueOrderId`; switch `PortfolioAccount.account_id` to `AccountId` with a serde-transparent repr; fix the `CalculationFailed` doc.

**Verification**: new `tests/ports_error_tests.rs` and a schema snapshot; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `make test`, `make prod`.

## Related

- Root gap 0012 (fabricated/synthetic data presented as real) consumes `PortError::unavailable`.
- Extends root plan 0019 P4.3 (downstream `PortError` redefinition).
- Plan 0006 R5 (stringly-typed ids) overlaps on `VenueOrderId`.
