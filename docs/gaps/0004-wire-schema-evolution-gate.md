# 0004 — Wire-schema compatibility gate covers only v7/v8 top-level keys; no frozen snapshot for SCHEMA_VERSION 10

**Status**: open
**Filed**: 2026-11-23 — workspace design review (root plan 0021) · **Kind**: deployment · **Severity**: high
**Plan**: [0006](../plans/0006-design-review-2026-11.md)

## What's stubbed or hardcoded

- `schema/compatibility/` contains only `v7_domain_schema.json` and `v8_domain_schema.json`, while `src/lib.rs:165` has `SCHEMA_VERSION = 10`.
- `tests/schema_compatibility_test.rs:60-100` compares only top-level `required` names and `properties` keys of the root object. Field removal, type change, enum-variant removal, or a new required field inside a DTO is not detected.
- `tests/schema_compatibility_test.rs:8-52` `test_all_canonical_types_implement_valid_json_schema` hand-lists types; new DTOs (e.g. `ClientOrderId`, `AccountId`, position types) are not forced in.
- `src/lib.rs:168` `API_VERSION = "v1"` while root design 0006 made `/api/v2` canonical; the constant is consumed in `tantra-runtime-api(-rest)`, `tantra-ffi-codegen` and `tantra-runtime-strategy`, so it is stale/ambiguous.
- No wire-level version field: only `TraceContext` (`src/wire.rs`) exists; payload envelopes carry no `schema_version`.
- `schema/domain/domain_schema.json` is a committed artefact (~22 KB); nothing verifies it is regenerated from derive output (`git diff --exit-code` after the generator).

## Why

With independently deployed runtime, agent and UI versions, a field renamed in v11 passes CI and breaks an older consumer at runtime. This is the main blocker for multi-version rollout.

## How to close it

1. Freeze `schema/compatibility/v9` and `v10` snapshots now, and at each `SCHEMA_VERSION` bump (Makefile target `schema-freeze`).
2. Replace the key-level check with a recursive JSON-Schema diff over full `$defs`. Allowed: add optional field/variant. Forbidden: remove/rename, optional to required, narrowing a type.
3. Auto-enumerate DTOs from `domain_schema.json` instead of the hand list.
4. Define the `SCHEMA_VERSION` bump policy and a `WireEnvelope<T> { schema_version, payload }`.
5. Split or deprecate `API_VERSION` (document the v1 alias vs v2).
6. Add N-1/N-2 payload corpora under `schema/golden/<version>/` and run them against current deserializers; add a CI `git diff --exit-code` regeneration check for `domain_schema.json`.

**Verification**: new `tests/schema_compat_recursive.rs` and golden round-trip of N-1/N-2 payloads; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make check`, `make test`, `make prod`.

## Related

- Root plan `docs/plans/0022-future-deployment-architecture.md` (independent version rollout) and root plan `docs/plans/0023-build-ci-release-engineering.md` (CI regeneration check, semver gate).
- Root plan 0019 P4.6 is a different gate (architecture verification), not this one.
- [0010](0010-decimal-wire-scale-pinning.md) (golden vectors for decimal scale feed this corpus).
