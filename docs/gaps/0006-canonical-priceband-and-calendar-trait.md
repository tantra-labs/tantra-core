# 0006 — Duplicated `PriceBand` and calendar traits across four repos

**Status**: open
**Filed**: 2026-11-23 — workspace design review (root plan 0021) · **Kind**: duplication · **Severity**: medium
**Plan**: [0006](../plans/0006-design-review-2026-11.md)

## What's stubbed or hardcoded

Provider: `tantra-core` (owner); consumer cleanup in `tantra-market`.

- `tantra-core/src/types.rs:499` `PriceBand { lower, upper, reference_price, band_pct }` vs `tantra-market/crates/tantra-market-finance/src/rules.rs:15` `PriceBand { lower, upper, reference_price }` with its own `new` (debug_assert) and own `contains`.
- Calendar traits:
  - `tantra-core/src/calendar.rs:35` `TradingCalendar`
  - `tantra-market/crates/tantra-market-finance/src/calendar.rs:68` `MarketCalendar`
  - `tantra-agent/crates/tantra-agent-chat-tools/src/ports/calendar.rs:56` `CalendarReader`
  - `tantra-runtime/crates/tantra-runtime-api-rest/src/experimental_analysis.rs:65` adapts `NseTradingCalendar` to `MarketCalendar` via the `RuntimeNseCalendar` newtype (adapter-of-adapter, repeated at `:1092`, `:1607`, `:2044`, `:2257`).

## Why

Circuit-band logic exists twice with different validity rules. A calendar change (holiday, Muhurat session) must touch three traits plus the adapter shims.

## How to close it

1. After [0003](0003-value-objects-constructible-invalid.md) validates core's `PriceBand`, delete the market-finance `PriceBand` and re-use core's.
2. Make `MarketCalendar` a supertrait or blanket impl over `TradingCalendar`, or remove it.
3. Turn chat-tools `CalendarReader` into an adapter wired in the L3 composition root (`tantra-runtime`); drop `RuntimeNseCalendar`.
4. Land the calendar session-model change ([0007](0007-calendar-session-model.md)) before consumers migrate, so they bind to the final trait shape.

**Verification**: re-point the existing market-finance tests; add a trait-object test where `Arc<dyn TradingCalendar>` is used from all three consumers; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `make test`, `make prod` in each touched repo.

## Related

- Root gap 0019 (calendar, instrument master and corporate-action single sources).
- Extends root plan 0019 P4.3 (same class of redefinition, not listed there).
- [0003](0003-value-objects-constructible-invalid.md), [0007](0007-calendar-session-model.md).
