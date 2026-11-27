//! Deterministic generation of the wire JSON Schema and a recursive
//! backward-compatibility diff over generated schemas.

use std::collections::BTreeSet;

use schemars::r#gen::{SchemaGenerator, SchemaSettings};
use serde_json::{Map, Value};

use crate::*;

fn register_all(generator: &mut SchemaGenerator) {
    generator.subschema_for::<AccountId>();
    generator.subschema_for::<AlgoId>();
    generator.subschema_for::<Bar>();
    generator.subschema_for::<BarRequest>();
    generator.subschema_for::<BreakevenTrigger>();
    generator.subschema_for::<CancelRequest>();
    generator.subschema_for::<ChartPattern>();
    generator.subschema_for::<ClassificationFilter>();
    generator.subschema_for::<ClientOrderId>();
    generator.subschema_for::<ConnectorState>();
    generator.subschema_for::<CountryCode>();
    generator.subschema_for::<Currency>();
    generator.subschema_for::<DatasetId>();
    generator.subschema_for::<DatedInstrument>();
    generator.subschema_for::<Decision>();
    generator.subschema_for::<DepthLevel>();
    generator.subschema_for::<DomainEvent>();
    generator.subschema_for::<EntryRule>();
    generator.subschema_for::<Exchange>();
    generator.subschema_for::<ExecutionEvent>();
    generator.subschema_for::<FeeBreakdown>();
    generator.subschema_for::<FillReport>();
    generator.subschema_for::<FilterOp>();
    generator.subschema_for::<GammaExposure>();
    generator.subschema_for::<IndexId>();
    generator.subschema_for::<Industry>();
    generator.subschema_for::<Instrument>();
    generator.subschema_for::<InstrumentId>();
    generator.subschema_for::<InstrumentKind>();
    generator.subschema_for::<Level>();
    generator.subschema_for::<LevelKind>();
    generator.subschema_for::<LevelMap>();
    generator.subschema_for::<LevelSide>();
    generator.subschema_for::<LogicalOp>();
    generator.subschema_for::<MaKind>();
    generator.subschema_for::<MarketCapBucket>();
    generator.subschema_for::<MarketEvent>();
    generator.subschema_for::<MetricDefinition>();
    generator.subschema_for::<MetricKeySpec>();
    generator.subschema_for::<MetricRef>();
    generator.subschema_for::<MinorUnit>();
    generator.subschema_for::<ModifyRequest>();
    generator.subschema_for::<Money>();
    generator.subschema_for::<OptionChainSnapshot>();
    generator.subschema_for::<OptionType>();
    generator.subschema_for::<OrderAck>();
    generator.subschema_for::<OrderBlock>();
    generator.subschema_for::<OrderEvent>();
    generator.subschema_for::<OrderEventKind>();
    generator.subschema_for::<OrderRequest>();
    generator.subschema_for::<OrderSide>();
    generator.subschema_for::<OrderStatus>();
    generator.subschema_for::<OrderType>();
    generator.subschema_for::<PatternKind>();
    generator.subschema_for::<PatternPivot>();
    generator.subschema_for::<PatternStage>();
    generator.subschema_for::<PlanSubject>();
    generator.subschema_for::<PlannedTarget>();
    generator.subschema_for::<PlannerInputs>();
    generator.subschema_for::<PortError>();
    generator.subschema_for::<PortErrorKind>();
    generator.subschema_for::<PortfolioAccount>();
    generator.subschema_for::<PortfolioGreeks>();
    generator.subschema_for::<PortfolioHolding>();
    generator.subschema_for::<PortfolioMode>();
    generator.subschema_for::<PortfolioPosition>();
    generator.subschema_for::<PortfolioSnapshot>();
    generator.subschema_for::<PortfolioSource>();
    generator.subschema_for::<PortfolioTotals>();
    generator.subschema_for::<PositionIntent>();
    generator.subschema_for::<PositionSide>();
    generator.subschema_for::<PositionSizingRequest>();
    generator.subschema_for::<PositionSizingResult>();
    generator.subschema_for::<PredicateValue>();
    generator.subschema_for::<Price>();
    generator.subschema_for::<PriceBand>();
    generator.subschema_for::<Quantity>();
    generator.subschema_for::<Reason>();
    generator.subschema_for::<ReasonGroup>();
    generator.subschema_for::<ReasonPolarity>();
    generator.subschema_for::<RejectReason>();
    generator.subschema_for::<RelativeStat>();
    generator.subschema_for::<RelativeTo>();
    generator.subschema_for::<RiskBudget>();
    generator.subschema_for::<ScreenerFilterGroup>();
    generator.subschema_for::<ScreenerFilterItem>();
    generator.subschema_for::<ScreenerFilterPredicate>();
    generator.subschema_for::<ScreenerRow>();
    generator.subschema_for::<ScreenerScanRequest>();
    generator.subschema_for::<ScreenerScanResponse>();
    generator.subschema_for::<Sector>();
    generator.subschema_for::<SessionWindow>();
    generator.subschema_for::<Signal>();
    generator.subschema_for::<SignalDirection>();
    generator.subschema_for::<SignalHorizon>();
    generator.subschema_for::<SignalKind>();
    generator.subschema_for::<SignalScanRequest>();
    generator.subschema_for::<SignalScanResponse>();
    generator.subschema_for::<SizingMethod>();
    generator.subschema_for::<SizingSpec>();
    generator.subschema_for::<SortSpec>();
    generator.subschema_for::<StopRule>();
    generator.subschema_for::<StrategySignal>();
    generator.subschema_for::<StrikeGamma>();
    generator.subschema_for::<StrikePositioning>();
    generator.subschema_for::<SubscriptionParams>();
    generator.subschema_for::<TargetLadder>();
    generator.subschema_for::<TargetLeg>();
    generator.subschema_for::<TargetTrigger>();
    generator.subschema_for::<Tick>();
    generator.subschema_for::<TickRequest>();
    generator.subschema_for::<TimeInForce>();
    generator.subschema_for::<TimeStop>();
    generator.subschema_for::<TraceContext>();
    generator.subschema_for::<TradeId>();
    generator.subschema_for::<TradePlan>();
    generator.subschema_for::<TradePlanSpec>();
    generator.subschema_for::<TradingState>();
    generator.subschema_for::<TrailRule>();
    generator.subschema_for::<TrailSpec>();
    generator.subschema_for::<TransactionId>();
    generator.subschema_for::<VenueOrderId>();
    generator.subschema_for::<WireEnvelope<serde_json::Value>>();
}

/// Generates the full draft-07 wire schema for every `JsonSchema` type in the
/// crate. Output is deterministic (definitions sorted by name).
pub fn domain_schema() -> Value {
    let mut generator = SchemaSettings::draft07().into_generator();
    register_all(&mut generator);
    let mut defs = Map::new();
    let mut sorted: Vec<(&String, &schemars::schema::Schema)> =
        generator.definitions().iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    for (name, schema) in sorted {
        // Schema serialization is infallible; Null is an unreachable fallback.
        defs.insert(
            name.clone(),
            serde_json::to_value(schema).unwrap_or(Value::Null),
        );
    }
    let mut root = Map::new();
    root.insert(
        "$schema".into(),
        Value::from("http://json-schema.org/draft-07/schema#"),
    );
    root.insert("title".into(), Value::from("tantra-core wire schema"));
    root.insert(
        "description".into(),
        Value::from("Generated from tantra-core derives by `make schema`; do not edit by hand."),
    );
    root.insert("schema_version".into(), Value::from(SCHEMA_VERSION));
    root.insert("definitions".into(), Value::Object(defs));
    Value::Object(root)
}

/// Lists backward-incompatible changes between `old` and `new` generated
/// schemas (reader compatibility: current code must accept payloads that were
/// valid under `old`). Each message names the JSON path of the violation.
pub fn breaking_changes(old: &Value, new: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let empty = Map::new();
    let old_defs = old
        .get("definitions")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let new_defs = new
        .get("definitions")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    for (name, od) in old_defs {
        let path = format!("definitions/{name}");
        match new_defs.get(name) {
            None => out.push(format!("{path}: definition removed")),
            Some(nd) => diff(od, nd, &path, &mut out),
        }
    }
    out
}

fn type_set(v: &Value) -> Option<BTreeSet<String>> {
    match v.get("type")? {
        Value::String(s) => Some(BTreeSet::from([s.clone()])),
        Value::Array(a) => Some(
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect(),
        ),
        _ => None,
    }
}

fn enum_set(v: &Value) -> Option<Vec<Value>> {
    if let Some(a) = v.get("enum").and_then(Value::as_array) {
        return Some(a.clone());
    }
    v.get("const").map(|c| vec![c.clone()])
}

fn str_set(v: &Value, key: &str) -> BTreeSet<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn variants(v: &Value) -> Vec<&Value> {
    ["oneOf", "anyOf"]
        .iter()
        .filter_map(|k| v.get(*k).and_then(Value::as_array))
        .flatten()
        .collect()
}

fn is_object_variant(v: &Value) -> bool {
    v.get("properties").is_some()
        || v.get("required").is_some()
        || type_set(v).is_some_and(|t| t.contains("object"))
}

/// Object-variant signature: sorted `required` plus single-value tag props.
fn object_sig(v: &Value) -> String {
    let required: Vec<String> = str_set(v, "required").into_iter().collect();
    let mut tags = Vec::new();
    if let Some(props) = v.get("properties").and_then(Value::as_object) {
        for (k, p) in props {
            if let Some(e) = enum_set(p)
                && e.len() == 1
            {
                tags.push(format!("{k}={}", e[0]));
            }
        }
    }
    format!("obj:{}|{}", required.join(","), tags.join(","))
}

fn other_sig(v: &Value) -> String {
    if let Some(r) = v.get("$ref").and_then(Value::as_str) {
        return format!("ref:{r}");
    }
    if let Some(t) = type_set(v) {
        return format!("type:{}", t.into_iter().collect::<Vec<_>>().join(","));
    }
    let mut c = v.clone();
    if let Some(o) = c.as_object_mut() {
        o.remove("description");
        o.remove("title");
    }
    format!("json:{c}")
}

fn find_variant<'a>(old: &Value, news: &[&'a Value]) -> Option<&'a Value> {
    if let Some(oe) = old.get("enum").and_then(Value::as_array) {
        return news.iter().copied().find(|n| {
            n.get("enum")
                .and_then(Value::as_array)
                .is_some_and(|ne| oe.iter().all(|x| ne.contains(x)))
        });
    }
    if is_object_variant(old) {
        let sig = object_sig(old);
        return news
            .iter()
            .copied()
            .find(|n| is_object_variant(n) && object_sig(n) == sig);
    }
    let sig = other_sig(old);
    news.iter().copied().find(|n| other_sig(n) == sig)
}

fn diff(old: &Value, new: &Value, path: &str, out: &mut Vec<String>) {
    let (Some(o), Some(n)) = (old.as_object(), new.as_object()) else {
        if old != &Value::Bool(true) && new == &Value::Bool(false) {
            out.push(format!("{path}: schema now rejects everything"));
        }
        return;
    };

    // $ref: compared by target only; the target definition is diffed itself.
    if let Some(or) = o.get("$ref") {
        if n.get("$ref") != Some(or) {
            out.push(format!(
                "{path}: $ref changed from {or} to {:?}",
                n.get("$ref")
            ));
        }
        return;
    }

    // type narrowing (absent type == any; integer widens into number)
    match (type_set(old), type_set(new)) {
        (Some(ot), Some(nt)) => {
            for t in &ot {
                let ok = nt.contains(t) || (t == "integer" && nt.contains("number"));
                if !ok {
                    out.push(format!("{path}/type: type '{t}' no longer accepted"));
                }
            }
        }
        (None, Some(nt)) => out.push(format!(
            "{path}/type: narrowed from any to {}",
            nt.into_iter().collect::<Vec<_>>().join(",")
        )),
        _ => {}
    }

    // enum / const narrowing
    match (enum_set(old), enum_set(new)) {
        (Some(oe), Some(ne)) => {
            for v in oe.iter().filter(|v| !ne.contains(v)) {
                out.push(format!("{path}/enum: value {v} removed"));
            }
        }
        (None, Some(_)) => out.push(format!("{path}/enum: narrowed from any value to an enum")),
        _ => {}
    }

    // format
    match (o.get("format"), n.get("format")) {
        (Some(of), Some(nf)) if of != nf => {
            out.push(format!("{path}/format: changed from {of} to {nf}"));
        }
        (None, Some(nf)) => out.push(format!("{path}/format: added {nf}")),
        _ => {}
    }

    // additionalProperties
    match (o.get("additionalProperties"), n.get("additionalProperties")) {
        (None | Some(Value::Bool(true)), Some(Value::Bool(false))) => {
            out.push(format!("{path}/additionalProperties: now forbidden"));
        }
        (Some(oa), Some(na)) if oa.is_object() && na.is_object() => {
            diff(oa, na, &format!("{path}/additionalProperties"), out);
        }
        _ => {}
    }

    // properties + required
    let old_required = str_set(old, "required");
    let new_required = str_set(new, "required");
    for r in new_required.difference(&old_required) {
        out.push(format!("{path}/required: '{r}' is newly required"));
    }
    if let Some(op) = o.get("properties").and_then(Value::as_object) {
        let np = n.get("properties").and_then(Value::as_object);
        for (k, ov) in op {
            let p = format!("{path}/properties/{k}");
            match np.and_then(|m| m.get(k)) {
                None => out.push(format!("{p}: property removed")),
                Some(nv) => diff(ov, nv, &p, out),
            }
        }
    }

    // items
    match (o.get("items"), n.get("items")) {
        (Some(Value::Array(oa)), Some(Value::Array(na))) => {
            for (i, (ov, nv)) in oa.iter().zip(na).enumerate() {
                diff(ov, nv, &format!("{path}/items/{i}"), out);
            }
        }
        (Some(oi), Some(ni)) if oi.is_object() && ni.is_object() => {
            diff(oi, ni, &format!("{path}/items"), out);
        }
        _ => {}
    }

    // allOf by index
    if let (Some(oa), Some(na)) = (
        o.get("allOf").and_then(Value::as_array),
        n.get("allOf").and_then(Value::as_array),
    ) {
        for (i, (ov, nv)) in oa.iter().zip(na).enumerate() {
            diff(ov, nv, &format!("{path}/allOf/{i}"), out);
        }
    }

    // oneOf / anyOf variants
    let old_vars = variants(old);
    let new_vars = variants(new);
    if !old_vars.is_empty() && !new_vars.is_empty() {
        for (i, ov) in old_vars.iter().enumerate() {
            let p = format!("{path}/variants/{i}");
            match find_variant(ov, &new_vars) {
                None => out.push(format!("{p}: variant removed or changed incompatibly")),
                Some(nv) => diff(ov, nv, &p, out),
            }
        }
    }
}
