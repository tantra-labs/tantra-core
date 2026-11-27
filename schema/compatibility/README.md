# Frozen wire-schema snapshots

- `v{N}_domain_schema.json`: the generated schema (`tantra_core::schema::domain_schema()`) frozen when `SCHEMA_VERSION` was bumped to N.
- `schema/domain/domain_schema.json` is the live generated schema. Regenerate with `make schema`; CI runs `make schema-check`.
- `schema/golden/v{N}/<Type>__<case>.json` holds payload goldens per version. Corpus files inside the support window must deserialize; files for the current version must also round-trip.

## Bump policy

Bump `SCHEMA_VERSION` on any wire-visible change, then run `make schema-freeze` (regenerates, freezes `v{N}`, creates the golden dir). Only additive changes are allowed within the N-2 window (`MIN_SUPPORTED_SCHEMA_VERSION`); `tests/schema_compat_recursive.rs` fails on removed definitions/fields/variants/enum values, newly required fields, narrowed types, changed `$ref`/`format`, and newly forbidden additional properties.

## Legacy snapshots

- `v7` and `v8` are hand-written (older `properties`/`required` shape with `#/components/schemas` refs, no `definitions`). They never matched the derive output (for example, `Tick` has always been a `type`/`data`-tagged enum, but they show it as a flat field bag), so no test checks against them. They are kept as historical record only. The old key-level check in `tests/schema_compatibility_test.rs` was retired with gap 0004.
- `v9` was never frozen and is not recoverable.
- `v10` is the first generated snapshot.
