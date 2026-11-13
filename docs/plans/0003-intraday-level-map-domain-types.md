# 0003 — Intraday Level-Map, Options-Positioning & Stop-Protection Domain Types

**Status**: Proposed
**Target Scope**: `tantra-core`
**Tracks**: [gap 0002](../gaps/0002-intraday-level-map-and-options-positioning-domain.md)
**Master plan**: root [`docs/plans/0005-elephant-edge-strategy-platform.md`](../../../docs/plans/0005-elephant-edge-strategy-platform.md)

## 0. Objective

Add the canonical kernel types the Elephant Edge (EE+) platform serializes
everywhere: the session level map (Elephant zones, OI walls, gamma flip,
Order Blocks, previous-day levels), option-chain/gamma snapshots, stop
protection (SL/TSL), strategy signals, and positioning intents. Zero-I/O,
immutable, Decimal-only, `JsonSchema`-derived wire contracts, WASM-clean.

## 1. Verified Current State

- `tantra-core` ships value objects, `Decision`, and `quant`; **no level-map /
  options-positioning / stop-protection / signal types exist** (gap 0002).
- `tantra-market` gaps 0001/0002 closed 2026-11-13: chain reading and OI
  stores exist **outside** the kernel; their DTOs are chat-tool-shaped
  (`OptionContract`, `OiBuildupSnapshot`), not kernel value objects — this
  plan complements rather than duplicates them (kernel types are the wire
  contract; the market stores remain the providers).

## 2. Phases

### Phase 1 — Level map (`src/levels.rs`)
`LevelKind`, `LevelSide`, `Level { kind, side, band, score }`, `LevelMap` with
golden vectors + `proptest` invariants (band `low <= high`, `score <= 4`).

### Phase 2 — Options positioning & gamma (`src/options.rs`)
`OptionChainSnapshot`, `StrikePositioning` (Decimal OI/ΔOI/IV/delta/gamma),
`GammaExposure` (`per_strike`, `net`, `flip`); invariants: no negative OI/IV,
flip within the struck band.

### Phase 3 — Order Blocks & stop protection (`src/order_blocks.rs`, `src/stop_protection.rs`)
`OrderBlock`, `TrailMode`, `StopProtection`, `StopUpdate`; invariants: stop
never inside the level band, `activation_after_r > 0`.

### Phase 4 — Signals & positioning (`src/signals.rs`)
`SignalKind`, `StrategySignal`, `PositionIntent`; `ref_levels` non-empty,
quantity > 0, `confidence` in 0–100.

### Phase 5 — Wire contracts
`make codegen` regenerates schemas; OpenAPI/MCP JSON for the level map is
served by `tantra-runtime-api-rest` and consumed by `tantra-agent` plan 0011
and `tantra-ui` plan 0006.

## 3. Quality Gates

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --profile prod
  -- -D warnings`, `cargo test --workspace --profile prod`, `make prod`
  end-of-turn (build-discipline rule).
- Tests live in `tests/` only (Parallel Test Directory Rule).
- No floats; no `SystemTime::now()`/`Instant::now()` (zero clock, temporal
  determinism — all types carry explicit `DateTime<Utc>` where time matters).

## 4. Related

- Strategy spec: [`tantra-strategies/docs/plans/0001-elephant-edge-strategy.md`](../../../tantra-strategies/docs/plans/0001-elephant-edge-strategy.md)
- SDK mirror: `tantra-strategies` gap 0006 (`sdk/levels.py` mirrors these 1:1)
- UI consumption: `tantra-ui` plan 0006
