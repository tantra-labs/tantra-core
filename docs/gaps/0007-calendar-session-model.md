# 0007 — `TradingCalendar` has a fixed UTC offset, a single unvalidated session per date and no day navigation

**Status**: open
**Filed**: 2026-11-23 — workspace design review (root plan 0021) · **Kind**: design-flaw · **Severity**: medium
**Plan**: [0006](../plans/0006-design-review-2026-11.md)

## What's stubbed or hardcoded

- `src/calendar.rs:49` `fn timezone_offset(&self) -> FixedOffset` allows one offset per calendar. That is fine for IST equities but not for the CDS/MCX evening session or any non-India venue now that `CountryCode` exists (`src/types.rs:543`).
- `src/calendar.rs:12-17` `SessionWindow { open, close }` has pub fields and no `open < close` check. `contains` is inclusive on close (`:27`), so 15:30:00 counts as open although NSE continuous trading closes at 15:30 exclusive.
- No pre-open, closing-auction or special (Muhurat) sessions: `session_for_date` returns a single `Option<SessionWindow>` (`:42`).
- The trait has no `next_trading_day` / `add_trading_days`, so every consumer re-implements them (see the downstream calendars in [0006](0006-canonical-priceband-and-calendar-trait.md)).

## Why

MCX runs 09:00-23:30/23:55 with overnight windows (`close < open`), which `contains` rejects. End-of-session orders are judged "open" one tick late. Confidence on the venue-specific cases is medium; the half-open and missing-navigation defects are certain.

## How to close it

1. Add a `SessionKind` enum and `sessions_for_date -> Vec<SessionWindow>`.
2. Make windows half-open `[open, close)`; add `SessionWindow::try_new`, allowing wrap-over only through an explicit flag.
3. Add provided methods `next_trading_day`, `prev_trading_day`, `trading_days_between`.
4. Keep `FixedOffset` but add `venue_tz: &'static str` for the schema.
5. Keep `session_for_date` as a deprecated shim for one release.

**Verification**: new `tests/calendar_tests.rs` with a table-driven test calendar and a `proptest` monotonicity check; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `make test`, `make prod`.

## Related

- Root gap 0019 (calendar single source) and root gap 0020 (IST trading day vs UTC).
- [0006](0006-canonical-priceband-and-calendar-trait.md) depends on this shape; [0003](0003-value-objects-constructible-invalid.md) shares the `try_new` pattern.
