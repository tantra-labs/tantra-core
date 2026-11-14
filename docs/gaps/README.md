# Gap Index — `tantra-core`

Per workspace rule `.agents/rules/gap-tracking.md`:
- **Gaps** (`docs/gaps/XXXX-*.md`): individual missing features, stubs, or rule violations.
- **Plans** (`docs/plans/XXXX-*.md`): collections of gaps.
- Numbering is per-repo, sequential, never reused.

## Open Gaps

| # | Title | Severity | Status | Plan |
|---|-------|----------|--------|------|
| [0002](0002-intraday-level-map-and-options-positioning-domain.md) | No intraday level-map / options-positioning / stop-protection / signal domain types (Elephant Edge EE+) | High | Open | [0003](../plans/0003-intraday-level-map-domain-types.md) |

## Closed Gaps

| # | Title | Closed In |
|---|-------|-----------|
| 0001 | Phase 2 call-site consolidation: delegate `tantra-market-indicators` / `tantra-agent-analytics` return/vol math to `tantra_core::quant` | Closed 2026-11-14 (plan 0002 Phase A audit): **no 1:1 delegation sites** — `tantra-market-indicators` is streaming f64 by policy, `tantra-agent-analytics` is statistical f64, and `tantra_core::quant` is batch Decimal; forcing delegation would change public behavior. Full matrix + verdict in `docs/design/code-quant.md` §Downstream call-site audit. Revisit only if a new Decimal-bounded quant consumer (e.g. `tantra-ledger-performance` parity vectors) appears |
