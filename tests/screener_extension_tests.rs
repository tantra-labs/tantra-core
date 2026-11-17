//! TDD tests for the converged Screener DSL in `tantra_core::screener` (Plan 0004 Phase B).

use std::str::FromStr;

use rust_decimal::Decimal;
use schemars::schema_for;
use tantra_core::classification::{IndexId, Industry, MarketCapBucket, Sector};
use tantra_core::screener::{
    ClassificationFilter, FilterOp, LogicalOp, MetricDefinition, MetricKeySpec, MetricPeriod,
    MetricRef, PredicateValue, RelativeStat, RelativeTo, ScreenerFilterGroup, ScreenerFilterItem,
    ScreenerFilterPredicate, ScreenerScanRequest, ScreenerScanResponse,
};
use tantra_core::signal::Timeframe;
use tantra_core::types::{Exchange, InstrumentId};

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

#[test]
fn recursive_and_or_tree_with_crosses_and_classification_roundtrips() {
    let sma50 = MetricRef(MetricKeySpec::new("SMA50").with_timeframe(Timeframe::D1));
    let sma200 = MetricRef(MetricKeySpec::new("SMA200").with_timeframe(Timeframe::D1));
    let rsi = MetricRef(
        MetricKeySpec::new("rsi_14")
            .with_period(MetricPeriod::Current)
            .with_timeframe(Timeframe::D1),
    );
    let pe = MetricRef(MetricKeySpec::new("pe").with_period(MetricPeriod::Ttm));

    let nested_or = ScreenerFilterGroup::or(vec![
        ScreenerFilterItem::Predicate(ScreenerFilterPredicate::new(
            sma50,
            FilterOp::CrossesAbove,
            PredicateValue::MetricRef(sma200),
        )),
        ScreenerFilterItem::Predicate(ScreenerFilterPredicate::between(
            rsi,
            PredicateValue::Decimal(dec("40")),
            PredicateValue::Decimal(dec("65")),
        )),
    ]);

    let root = ScreenerFilterGroup::and(vec![
        ScreenerFilterItem::Predicate(
            ScreenerFilterPredicate::new(pe.clone(), FilterOp::Lt, PredicateValue::MetricRef(pe))
                .with_relative(RelativeTo::Sector, RelativeStat::Median),
        ),
        ScreenerFilterItem::Group(Box::new(nested_or)),
    ])
    .with_classification(ClassificationFilter {
        sector: Some(Sector::try_new("Information Technology").unwrap()),
        industry: Some(Industry::try_new("IT - Software").unwrap()),
        cap_bucket: Some(MarketCapBucket::Large),
        index: Some(IndexId::try_new("NIFTY50").unwrap()),
        theme: Some("AI_EXPORT".to_string()),
    });

    root.validate().expect("valid recursive tree");

    let req = ScreenerScanRequest {
        universe: Some(vec![InstrumentId::new(Exchange::Nse, "TCS")]),
        filters: root,
        sort: None,
        limit: Some(25),
    };

    let json = serde_json::to_string(&req).unwrap();
    let decoded: ScreenerScanRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, req);
}

#[test]
fn relative_to_index_return_delta_roundtrips() {
    let ret = MetricRef(MetricKeySpec::new("return_3m").with_period(MetricPeriod::Ttm));
    let pred = ScreenerFilterPredicate::new(ret, FilterOp::Gt, PredicateValue::Decimal(dec("0")))
        .with_relative(
            RelativeTo::Index(IndexId::try_new("NIFTY500").unwrap()),
            RelativeStat::ReturnDelta,
        );

    let json = serde_json::to_string(&pred).unwrap();
    let decoded: ScreenerFilterPredicate = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, pred);
    assert_eq!(
        decoded.relative_to,
        Some(RelativeTo::Index(IndexId::try_new("NIFTY500").unwrap()))
    );
    assert_eq!(decoded.relative_stat, Some(RelativeStat::ReturnDelta));
}

#[test]
fn legacy_v8_flat_group_and_neq_alias_deserialize_cleanly() {
    let legacy_v8 = serde_json::json!({
        "universe": [{"exchange": "NSE", "symbol": "RELIANCE"}],
        "filters": {
            "predicates": [
                {
                    "metric": {"metric": "rsi_14", "period": "d1"},
                    "op": "gt",
                    "value": "70"
                },
                {
                    "metric": {"metric": "pe", "period": "d1"},
                    "op": "between",
                    "value": "10",
                    "value_upper": "30"
                },
                {
                    "metric": {"metric": "flag", "period": "d1"},
                    "op": "neq",
                    "value": "0"
                }
            ],
            "match_all": true
        },
        "limit": 50
    });

    let req: ScreenerScanRequest = serde_json::from_value(legacy_v8).unwrap();
    assert_eq!(req.filters.operator, LogicalOp::And);
    assert_eq!(req.filters.items.len(), 3);
    assert_eq!(req.limit, Some(50));

    let preds = req.filters.predicates();
    assert_eq!(preds[0].metric.0.key, "rsi_14");
    assert_eq!(preds[0].metric.0.period, Some(MetricPeriod::Daily));
    assert_eq!(preds[0].op, FilterOp::Gt);
    assert_eq!(preds[0].value, PredicateValue::Decimal(dec("70")));

    assert_eq!(preds[1].op, FilterOp::Between);
    assert_eq!(
        preds[1].value,
        PredicateValue::List(vec![
            PredicateValue::Decimal(dec("10")),
            PredicateValue::Decimal(dec("30"))
        ])
    );

    assert_eq!(preds[2].op, FilterOp::Ne);
}

#[test]
fn flat_python_and_runtime_predicate_syntax_deserializes() {
    let wire_json = serde_json::json!({
        "operator": "OR",
        "items": [
            {
                "key": "SMA50",
                "op": "crosses_below",
                "value": {"key": "SMA200"},
                "timeframe": "1D"
            },
            {
                "key": "sector",
                "op": "in",
                "value": ["IT", "Banking"]
            },
            {
                "key": "name",
                "op": "like",
                "value": "RELIANCE"
            },
            {
                "key": "is_fno",
                "op": "eq",
                "value": true
            }
        ]
    });

    let group: ScreenerFilterGroup = serde_json::from_value(wire_json).unwrap();
    assert_eq!(group.operator, LogicalOp::Or);
    assert_eq!(group.items.len(), 4);
    group.validate().unwrap();
}

#[test]
fn predicate_value_rejects_floats_and_nan_infinity() {
    // Raw IEEE-754 float in JSON must be rejected at the serde boundary.
    let raw_float = r#"{"metric":{"key":"close"},"op":"gt","value":150.75}"#;
    assert!(
        serde_json::from_str::<ScreenerFilterPredicate>(raw_float).is_err(),
        "raw JSON float must be rejected; exact decimal string required"
    );

    // Non-finite numeric literals as strings must be rejected.
    for bad in ["NaN", "nan", "Infinity", "-Infinity", "inf", "-inf"] {
        let payload = serde_json::json!({
            "metric": {"key": "close"},
            "op": "gt",
            "value": bad
        });
        assert!(
            serde_json::from_value::<ScreenerFilterPredicate>(payload).is_err(),
            "non-finite literal {bad:?} must be rejected"
        );
    }

    // Exact decimal strings and exact integer literals succeed as Decimal.
    let str_dec: ScreenerFilterPredicate =
        serde_json::from_str(r#"{"metric":{"key":"close"},"op":"gt","value":"150.75"}"#).unwrap();
    assert_eq!(str_dec.value, PredicateValue::Decimal(dec("150.75")));

    let int_dec: ScreenerFilterPredicate =
        serde_json::from_str(r#"{"metric":{"key":"close"},"op":"gt","value":150}"#).unwrap();
    assert_eq!(int_dec.value, PredicateValue::Decimal(dec("150")));
}

#[test]
fn validation_rejects_mismatched_operator_shapes() {
    // Between requires 2 operands.
    let bad_between = ScreenerFilterPredicate::new(
        MetricRef::new("pe"),
        FilterOp::Between,
        PredicateValue::Decimal(dec("10")),
    );
    assert!(bad_between.validate().is_err());

    // In / NotIn require a list.
    let bad_in = ScreenerFilterPredicate::new(
        MetricRef::new("sector"),
        FilterOp::In,
        PredicateValue::String("IT".to_string()),
    );
    assert!(bad_in.validate().is_err());

    // CrossesAbove requires Decimal or MetricRef, not String/Bool.
    let bad_cross = ScreenerFilterPredicate::new(
        MetricRef::new("SMA50"),
        FilterOp::CrossesAbove,
        PredicateValue::String("SMA200".to_string()),
    );
    assert!(bad_cross.validate().is_err());
}

#[test]
fn screener_wire_types_generate_json_schemas() {
    let _ = schema_for!(ScreenerScanRequest);
    let _ = schema_for!(ScreenerScanResponse);
    let _ = schema_for!(ScreenerFilterGroup);
    let _ = schema_for!(ScreenerFilterPredicate);
    let _ = schema_for!(MetricDefinition);
    let _ = schema_for!(ClassificationFilter);
    let _ = schema_for!(RelativeTo);
    let _ = schema_for!(RelativeStat);
}
