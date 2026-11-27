//! Wire-schema evolution gate: regenerated-schema freshness, recursive
//! backward-compatibility of frozen snapshots, type registration, versioned
//! envelope and golden corpus.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tantra_core::schema::{breaking_changes, domain_schema};
use tantra_core::{
    CoreError, Decision, LevelMap, MIN_SUPPORTED_SCHEMA_VERSION, OptionChainSnapshot, PriceBand,
    SCHEMA_VERSION, Tick, TraceContext, TradePlan, WireEnvelope,
};

const SCHEMA_PATH: &str = "schema/domain/domain_schema.json";

fn rendered() -> String {
    let mut s = serde_json::to_string_pretty(&domain_schema()).expect("schema serializes");
    s.push('\n');
    s
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

#[test]
fn domain_schema_is_regenerated() {
    let fresh = rendered();
    if std::env::var("TANTRA_SCHEMA_BLESS").as_deref() == Ok("1") {
        fs::write(SCHEMA_PATH, &fresh).expect("write domain schema");
        return;
    }
    let on_disk = fs::read_to_string(SCHEMA_PATH).unwrap_or_default();
    assert!(
        on_disk == fresh,
        "{SCHEMA_PATH} is stale: run `make schema`"
    );
}

fn snapshot_version(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    name.strip_prefix('v')?
        .strip_suffix("_domain_schema.json")?
        .parse()
        .ok()
}

#[test]
fn frozen_snapshots_are_backward_compatible() {
    let current = domain_schema();
    let current_snapshot = PathBuf::from(format!(
        "schema/compatibility/v{SCHEMA_VERSION}_domain_schema.json"
    ));
    assert!(
        current_snapshot.exists(),
        "missing {current_snapshot:?}: run `make schema-freeze`"
    );
    let mut violations = Vec::new();
    for entry in fs::read_dir("schema/compatibility").expect("compat dir") {
        let path = entry.expect("dir entry").path();
        let Some(k) = snapshot_version(&path) else {
            continue;
        };
        if k < MIN_SUPPORTED_SCHEMA_VERSION {
            continue;
        }
        let frozen = read_json(&path);
        if frozen.get("definitions").is_none() {
            continue; // legacy hand-written shape
        }
        for v in breaking_changes(&frozen, &current) {
            violations.push(format!("v{k}: {v}"));
        }
    }
    assert!(
        violations.is_empty(),
        "breaking wire-schema changes:\n{}",
        violations.join("\n")
    );
}

/// Types whose schema is inlined at every use site (newtypes / transparent
/// wrappers that schemars does not emit as named definitions). They are still
/// registered in `schema::register_all`, but cannot appear in `definitions`.
const INLINE_ONLY: &[&str] = &[
    "DatasetId",
    "IndexId",
    "Industry",
    "MetricRef",
    "Price",
    "Quantity",
    "Sector",
    "TradeId",
    "VenueOrderId",
];

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read src dir") {
        let p = entry.expect("entry").path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn every_jsonschema_type_is_registered() {
    let schema = domain_schema();
    let defs = schema["definitions"].as_object().expect("definitions");
    let mut files = Vec::new();
    collect_rs(Path::new("src"), &mut files);
    let mut missing = Vec::new();
    let mut found = 0;
    for f in files {
        let text = fs::read_to_string(&f).expect("read src");
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim_start();
            let rest = t
                .strip_prefix("pub struct ")
                .or_else(|| t.strip_prefix("pub enum "));
            let Some(rest) = rest else { continue };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            // Walk upward through attributes / doc comments to find derives.
            let mut derives_schema = false;
            let mut j = i;
            while j > 0 {
                j -= 1;
                let prev = lines[j].trim();
                if prev.starts_with("#[") || prev.starts_with("///") || prev.is_empty() {
                    if prev.starts_with("#[derive") && prev.contains("JsonSchema") {
                        derives_schema = true;
                    }
                    if prev.is_empty() {
                        break;
                    }
                } else if prev.starts_with("//") || prev.starts_with(')') || prev.ends_with(")]") {
                    continue;
                } else {
                    break;
                }
            }
            if derives_schema {
                found += 1;
                let registered = defs
                    .keys()
                    .any(|k| k == &name || k.starts_with(&format!("{name}_")));
                if INLINE_ONLY.contains(&name.as_str()) {
                    assert!(
                        !registered,
                        "{name} is now a named definition: drop it from INLINE_ONLY"
                    );
                } else if !registered {
                    missing.push(format!("{name} ({})", f.display()));
                }
            }
        }
    }
    assert!(found > 100, "scanner found only {found} JsonSchema types");
    assert!(
        missing.is_empty(),
        "JsonSchema types missing from schema::register_all: {missing:?}"
    );
}

// ---- breaking_changes unit tests -------------------------------------------

fn wrap(def: Value) -> Value {
    json!({"definitions": {"T": def}})
}

fn assert_breaks(old: Value, new: Value) {
    let v = breaking_changes(&wrap(old), &wrap(new));
    assert!(!v.is_empty(), "expected a breaking change");
    assert!(v.iter().all(|m| m.contains("definitions/T")), "{v:?}");
}

fn assert_ok(old: Value, new: Value) {
    let v = breaking_changes(&wrap(old), &wrap(new));
    assert!(v.is_empty(), "unexpected: {v:?}");
}

#[test]
fn breaking_removed_field() {
    assert_breaks(
        json!({"type":"object","properties":{"a":{"type":"string"},"b":{"type":"string"}}}),
        json!({"type":"object","properties":{"a":{"type":"string"}}}),
    );
}

#[test]
fn breaking_optional_becomes_required() {
    assert_breaks(
        json!({"type":"object","properties":{"a":{"type":"string"}}}),
        json!({"type":"object","properties":{"a":{"type":"string"}},"required":["a"]}),
    );
}

#[test]
fn breaking_new_required_field() {
    assert_breaks(
        json!({"type":"object","properties":{"a":{"type":"string"}}}),
        json!({"type":"object","properties":{"a":{"type":"string"},"b":{"type":"string"}},"required":["b"]}),
    );
}

#[test]
fn breaking_narrowed_type() {
    assert_breaks(json!({"type":["string","null"]}), json!({"type":"string"}));
}

#[test]
fn breaking_removed_enum_value() {
    assert_breaks(
        json!({"type":"string","enum":["A","B"]}),
        json!({"type":"string","enum":["A"]}),
    );
}

#[test]
fn breaking_removed_variant() {
    assert_breaks(
        json!({"oneOf":[
            {"type":"object","required":["a"],"properties":{"a":{"type":"string"}}},
            {"type":"object","required":["b"],"properties":{"b":{"type":"string"}}}]}),
        json!({"oneOf":[
            {"type":"object","required":["a"],"properties":{"a":{"type":"string"}}}]}),
    );
}

#[test]
fn breaking_variants_collapsed_into_single_shape() {
    assert_breaks(
        json!({"oneOf":[
            {"type":"object","required":["a"],"properties":{"a":{"type":"string"}}},
            {"type":"object","required":["b"],"properties":{"b":{"type":"string"}}}]}),
        json!({"type":"object","required":["a"],"properties":{"a":{"type":"string"}}}),
    );
}

#[test]
fn breaking_ref_change() {
    assert_breaks(
        json!({"type":"object","properties":{"a":{"$ref":"#/definitions/X"}}}),
        json!({"type":"object","properties":{"a":{"$ref":"#/definitions/Y"}}}),
    );
}

#[test]
fn breaking_removed_definition() {
    let old = json!({"definitions": {"T": {"type":"string"}}});
    let new = json!({"definitions": {}});
    assert!(!breaking_changes(&old, &new).is_empty());
}

#[test]
fn breaking_additional_properties_forbidden() {
    assert_breaks(
        json!({"type":"object"}),
        json!({"type":"object","additionalProperties":false}),
    );
}

#[test]
fn allowed_additive_changes() {
    assert_ok(
        json!({"type":"object","properties":{"a":{"type":"string"}}}),
        json!({"type":"object","properties":{"a":{"type":"string","description":"d"},"b":{"type":"string"}}}),
    );
    assert_ok(
        json!({"oneOf":[{"type":"object","required":["a"],"properties":{"a":{"type":"string"}}}]}),
        json!({"oneOf":[
            {"type":"object","required":["a"],"properties":{"a":{"type":"string"}}},
            {"type":"object","required":["c"],"properties":{"c":{"type":"string"}}}]}),
    );
    assert_ok(
        json!({"type":"string","enum":["A"]}),
        json!({"type":"string","enum":["A","B"]}),
    );
    let old = json!({"definitions": {"T": {"type":"string"}}});
    let new = json!({"definitions": {"T": {"type":"string"}, "U": {"type":"string"}}});
    assert!(breaking_changes(&old, &new).is_empty());
}

// ---- WireEnvelope -----------------------------------------------------------

fn trace() -> TraceContext {
    TraceContext::new_root([1; 16], [2; 8])
}

#[test]
fn wire_envelope_round_trips() {
    let env = WireEnvelope::new(trace());
    assert_eq!(env.schema_version(), SCHEMA_VERSION);
    let json = serde_json::to_string(&env).expect("serialize");
    let back: WireEnvelope<TraceContext> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, env);
    assert_eq!(back.payload(), &trace());
    assert_eq!(back.into_payload(), trace());
}

fn envelope_json(version: u32) -> Value {
    json!({"schema_version": version, "payload": serde_json::to_value(trace()).expect("value")})
}

#[test]
fn wire_envelope_rejects_out_of_window_versions() {
    for v in [SCHEMA_VERSION + 1, MIN_SUPPORTED_SCHEMA_VERSION - 1] {
        let err = serde_json::from_value::<WireEnvelope<TraceContext>>(envelope_json(v))
            .expect_err("must reject");
        let expected = CoreError::UnsupportedSchemaVersion {
            found: v,
            min: MIN_SUPPORTED_SCHEMA_VERSION,
            max: SCHEMA_VERSION,
        };
        assert_eq!(
            err.to_string().split(" at line").next(),
            Some(expected.to_string().as_str())
        );
        assert_eq!(tantra_core::wire::validate_schema_version(v), Err(expected));
    }
    for v in MIN_SUPPORTED_SCHEMA_VERSION..=SCHEMA_VERSION {
        serde_json::from_value::<WireEnvelope<TraceContext>>(envelope_json(v))
            .expect("in-window version accepted");
    }
}

#[test]
fn wire_envelope_rejects_unknown_field() {
    let mut v = envelope_json(SCHEMA_VERSION);
    v["extra"] = json!(1);
    assert!(serde_json::from_value::<WireEnvelope<TraceContext>>(v).is_err());
}

// ---- golden corpus ----------------------------------------------------------

fn check<T: DeserializeOwned + Serialize>(value: &Value, round_trip: bool, name: &str) {
    let parsed: T = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("{name}: deserialize failed: {e}"));
    if round_trip {
        let back = serde_json::to_value(&parsed).expect("serialize");
        assert_eq!(&back, value, "{name}: round-trip mismatch");
    }
}

#[test]
fn golden_corpus_deserializes() {
    for k in MIN_SUPPORTED_SCHEMA_VERSION..=SCHEMA_VERSION {
        let dir = PathBuf::from(format!("schema/golden/v{k}"));
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for entry in rd {
            let path = entry.expect("entry").path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).expect("stem");
            let type_name = stem.split("__").next().expect("type name");
            let value = read_json(&path);
            let rt = k == SCHEMA_VERSION;
            let name = path.display().to_string();
            match type_name {
                "Tick" => check::<Tick>(&value, rt, &name),
                "PriceBand" => check::<PriceBand>(&value, rt, &name),
                "Decision" => check::<Decision>(&value, rt, &name),
                "LevelMap" => check::<LevelMap>(&value, rt, &name),
                "OptionChainSnapshot" => check::<OptionChainSnapshot>(&value, rt, &name),
                "TradePlan" => check::<TradePlan>(&value, rt, &name),
                other => panic!("{name}: unknown golden type '{other}'"),
            }
        }
    }
}
