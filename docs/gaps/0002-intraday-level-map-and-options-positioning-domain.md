# Gap 0002 — No Intraday Level-Map / Stop-Protection / Options-Positioning Domain Types

**Status**: open — tracked by [plan 0003](../plans/0003-intraday-level-map-domain-types.md)

## What's Missing

`tantra-core` has the canonical `Price`/`Quantity`/`Money` value objects, the
`Decision` grammar, and `quant` primitives, but **no domain types** for the
intraday strategy layer that Elephant Edge (EE+) needs:

- **Level map**: no `Level` / `LevelMap` type for support/resistance levels
  (Elephant percentile zones, OI walls, previous-day OHLC, gamma flip,
  max pain, round-number strikes, Order Blocks) with a band, side, kind, and
  confluence score.
- **Option-chain snapshot**: no `OptionChainSnapshot` (expiry, as-of,
  per-strike call/put OI, ΔOI, IV, delta, gamma) and no `GammaExposure` /
  `GammaFlip` types — `tantra-market`'s closed gaps 0001/0002 provide chain
  reading and OI *stores*, but not a canonical in-memory/`Decision`-adjacent
  type.
- **Order Block**: no `OrderBlock` value object (footprint candle → zone,
  side).
- **Stop protection**: no `StopProtection` type for SL + trailing-stop specs
  (trail mode, distance, activation) and no `StopUpdate` event.
- **Signal & positioning**: no `StrategySignal` (side, kind, ref levels,
  confidence) or `PositionIntent` (instrument, side, quantity, risk caps).

## Why

The workspace rule "Consolidation Over Duplication" (`.agents/AGENTS.md` §4.1)
and the canonical-wire-contract invariant (§2.1) require these to exist **once,
in the kernel**, so the Python SDK (`tantra-strategies` gap 0006), the Rust
REST API (`tantra-runtime-api-rest`), the chat tool surface (`tantra-agent`
plan 0011), and the UI (`tantra-ui` plan 0006) all serialize the **same**
schema. Without them, each repo invents its own struct/DTO → drift risk across
Rust↔Python↔UI, exactly the duplication the kernel exists to prevent.

## How to Close It

1. `src/levels.rs` — `LevelKind` enum
   (`ElephantSupport1..2`, `ElephantResistance1..2`, `PreviousDayOHLC`,
   `OiWallCall`, `OiWallPut`, `GammaFlip`, `GammaWallCall`, `GammaWallPut`,
   `MaxPain`, `StrikeRound`, `OrderBlock`), `LevelSide` (Support/Resistance),
   `Level { kind, side, band: (Decimal, Decimal), score: u8 }`, `LevelMap`.
2. `src/options.rs` — `OptionChainSnapshot`, `StrikePositioning`
   (call/put OI, `oi_change`, iv, delta, gamma — all `Decimal`),
   `GammaExposure` (`per_strike`, `net`, `flip: Decimal`).
3. `src/order_blocks.rs` — `OrderBlock { ts, side, zone: (Decimal, Decimal) }`.
4. `src/stop_protection.rs` — `TrailMode` (`Off|Atr|Swing`),
   `StopProtection { initial_stop: Decimal, trail: TrailMode,
   trail_distance: Option<Decimal>, activation_after_r: Decimal }`,
   `StopUpdate { order_id, stop: Decimal, ts: DateTime<Utc> }`.
5. `src/signals.rs` — `SignalKind` (`Breakout|Reversal|Retest`),
   `StrategySignal { side: OrderSide, kind, ref_levels: Vec<LevelId>,
   confidence: Decimal }`, `PositionIntent { instrument: InstrumentId, side,
   quantity: Quantity, risk_cap: Money, max_fraction: Decimal }`.
6. All types: immutable, self-validating, `Decimal` (never `f64`),
   `JsonSchema` + `schemars` (wire contracts), compile to
   `wasm32-unknown-unknown`. Tests in `tests/` per the Parallel Test Directory
   Rule (unit + `proptest` for band/validation invariants + golden vectors).

## Verification

`cargo test --workspace --profile prod` green in `tantra-core`; the new types
export from `lib.rs`; JSON schemas regenerate via `make codegen`; golden
`tests/golden/*.json` cover a representative day's level map, chain snapshot,
gamma flip, stop update, and signal.
