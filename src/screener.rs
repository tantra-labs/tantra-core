//! Converged screener / scan wire contracts (Plan 0004 Phase B).
//!
//! Unifies the platform's screener DSL into one recursive, `Decimal`-safe AST
//! shared by `tantra-core`, `tantra-runtime-api`, `tantra-market-indicators`,
//! `tantra-ui`, and `tantra-ffi` (Python SDK). Zero floats (`f32`/`f64`) ever
//! appear in wire structs or `PredicateValue` deserialization.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::de::value::MapAccessDeserializer;
use serde::{Deserialize, Deserializer, Serialize};

use crate::classification::{IndexId, Industry, MarketCapBucket, Sector};
use crate::error::CoreError;
use crate::signal::Timeframe;
use crate::types::InstrumentId;

/// Reporting or sampling period for a screener metric.
///
/// Accepts both fundamental periods (`Snapshot`, `Ttm`, `Fy`, `Fq`, `H1`, `Current`)
/// and cadence periods (`Intraday`, `Daily`, `Weekly`, `Monthly`), plus uppercase and
/// legacy `d1`/`w1` aliases during migration.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MetricPeriod {
    /// Point-in-time balance-sheet or latest fundamental snapshot.
    #[serde(alias = "SNAPSHOT", alias = "Snapshot")]
    Snapshot,
    /// Trailing twelve months.
    #[serde(alias = "TTM", alias = "Ttm")]
    Ttm,
    /// Fiscal year.
    #[serde(alias = "FY", alias = "Fy")]
    Fy,
    /// Fiscal quarter.
    #[serde(alias = "FQ", alias = "Fq")]
    Fq,
    /// First half / semi-annual period.
    #[serde(alias = "H1", alias = "HalfYear")]
    H1,
    /// Current / latest bar or reporting interval.
    #[serde(alias = "CURRENT", alias = "Current")]
    Current,
    /// Intraday sampling period.
    #[serde(alias = "Intraday", alias = "INTRADAY")]
    Intraday,
    /// Daily sampling period (v8 `d1` alias).
    #[serde(
        alias = "Daily",
        alias = "DAILY",
        alias = "D1",
        alias = "d1",
        alias = "1D",
        alias = "1d"
    )]
    Daily,
    /// Weekly sampling period (v8 `w1` alias).
    #[serde(
        alias = "Weekly",
        alias = "WEEKLY",
        alias = "W1",
        alias = "w1",
        alias = "1W",
        alias = "1w"
    )]
    Weekly,
    /// Monthly sampling period.
    #[serde(alias = "Monthly", alias = "MONTHLY", alias = "1M", alias = "1mo")]
    Monthly,
}

impl MetricPeriod {
    /// Alias for [`MetricPeriod::Daily`] (ergonomic migration from v8 `Timeframe::D1`).
    pub const D1: Self = Self::Daily;
    /// Alias for [`MetricPeriod::Weekly`] (ergonomic migration from v8 `Timeframe::W1`).
    pub const W1: Self = Self::Weekly;
}

/// Boolean combinator for a [`ScreenerFilterGroup`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
pub enum LogicalOp {
    /// All child items must match (vacuously true when empty).
    #[default]
    #[serde(rename = "AND", alias = "and", alias = "And")]
    And,
    /// At least one child item must match.
    #[serde(rename = "OR", alias = "or", alias = "Or")]
    Or,
}

impl LogicalOp {
    /// Wire text (`"AND"` or `"OR"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
        }
    }
}

impl fmt::Display for LogicalOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Unified comparison operator for a screener filter predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    /// Equal to (`==`).
    Eq,
    /// Not equal to (`!=`); accepts legacy `"neq"` alias on deserialization.
    #[serde(alias = "neq")]
    Ne,
    /// Greater than (`>`).
    Gt,
    /// Greater than or equal to (`>=`).
    Gte,
    /// Less than (`<`).
    Lt,
    /// Less than or equal to (`<=`).
    Lte,
    /// Inclusively between `[low, high]`.
    Between,
    /// Membership in a list of values.
    In,
    /// Non-membership in a list of values.
    NotIn,
    /// Case-insensitive substring/pattern match.
    Like,
    /// Contains element / substring match.
    Has,
    /// Previous bar <= RHS and current bar > RHS.
    CrossesAbove,
    /// Previous bar >= RHS and current bar < RHS.
    CrossesBelow,
}

impl FilterOp {
    /// Canonical snake-case operator string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::Between => "between",
            Self::In => "in",
            Self::NotIn => "not_in",
            Self::Like => "like",
            Self::Has => "has",
            Self::CrossesAbove => "crosses_above",
            Self::CrossesBelow => "crosses_below",
        }
    }
}

impl fmt::Display for FilterOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Key naming a metric with optional reporting period and bar timeframe dimensions.
///
/// Accepts legacy v8 `"metric"` field name via `#[serde(alias = "metric")]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MetricKeySpec {
    /// Canonical metric key (e.g. `"rsi_14"`, `"pe"`, `"close"`).
    #[serde(alias = "metric")]
    pub key: String,
    /// Optional fundamental/sampling period dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<MetricPeriod>,
    /// Optional bar interval dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<Timeframe>,
}

impl MetricKeySpec {
    /// Create a bare metric specification without explicit dimensions.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            period: None,
            timeframe: None,
        }
    }

    /// Attach a [`MetricPeriod`] dimension.
    #[must_use]
    pub fn with_period(mut self, period: MetricPeriod) -> Self {
        self.period = Some(period);
        self
    }

    /// Attach a [`Timeframe`] dimension.
    #[must_use]
    pub fn with_timeframe(mut self, timeframe: Timeframe) -> Self {
        self.timeframe = Some(timeframe);
        self
    }
}

/// Describes a metric the screener exposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MetricDefinition {
    /// The metric's key.
    pub key: MetricKeySpec,
    /// Human-readable description.
    pub description: String,
    /// Unit, if applicable (e.g. `"%"`, `"INR"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// A reference to a metric value, used inside filters and sort specs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct MetricRef(pub MetricKeySpec);

impl MetricRef {
    /// Create a [`MetricRef`] with no explicit period/timeframe overrides.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(MetricKeySpec::new(key))
    }

    /// Borrow the underlying metric key string.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.0.key
    }
}

/// Peer/benchmark reference group for relative comparisons (e.g. `pe < sector_median(pe)`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelativeTo {
    /// Compare against instruments in the same [`Sector`].
    Sector,
    /// Compare against instruments in the same [`Industry`].
    Industry,
    /// Compare against an [`IndexId`] benchmark (e.g. `NIFTY500`).
    Index(IndexId),
}

/// Cross-sectional or relative return statistic computed over a [`RelativeTo`] scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelativeStat {
    /// Peer median of the metric.
    Median,
    /// Peer arithmetic mean of the metric.
    Mean,
    /// Return delta vs benchmark (`metric_return - benchmark_return`).
    ReturnDelta,
}

/// Classification constraints optionally attached to a [`ScreenerFilterGroup`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClassificationFilter {
    /// Restrict to instruments in this sector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector: Option<Sector>,
    /// Restrict to instruments in this industry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub industry: Option<Industry>,
    /// Restrict to instruments in this market-cap bucket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_bucket: Option<MarketCapBucket>,
    /// Restrict to constituents of this index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<IndexId>,
    /// Opaque thematic tag (defined outside L0, e.g. `"EV"`, `"PLI"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

impl ClassificationFilter {
    /// Returns `true` when all filter fields are unset.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sector.is_none()
            && self.industry.is_none()
            && self.cap_bucket.is_none()
            && self.index.is_none()
            && self.theme.is_none()
    }
}

/// `Decimal`-safe operand for a [`ScreenerFilterPredicate`].
///
/// IEEE-754 floating-point values (`f32`/`f64`) and non-finite literals (`NaN`,
/// `Infinity`) are strictly rejected at the deserialization boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum PredicateValue {
    /// Exact decimal number (serialized as string).
    Decimal(Decimal),
    /// Boolean scalar.
    Bool(bool),
    /// Metric reference (`{"key": "SMA200"}`).
    MetricRef(MetricRef),
    /// List of operands (used by `Between`, `In`, `NotIn`).
    List(Vec<PredicateValue>),
    /// String literal (used by `Like`, `Has`, `Eq`, `In`, etc.).
    String(String),
}

impl PredicateValue {
    /// Returns the decimal value if this operand is [`PredicateValue::Decimal`].
    #[must_use]
    pub const fn as_decimal(&self) -> Option<Decimal> {
        match self {
            Self::Decimal(d) => Some(*d),
            _ => None,
        }
    }

    /// Returns the boolean value if this operand is [`PredicateValue::Bool`].
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the metric reference if this operand is [`PredicateValue::MetricRef`].
    #[must_use]
    pub const fn as_metric_ref(&self) -> Option<&MetricRef> {
        match self {
            Self::MetricRef(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the slice of items if this operand is [`PredicateValue::List`].
    #[must_use]
    pub fn as_list(&self) -> Option<&[PredicateValue]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// Returns the string slice if this operand is [`PredicateValue::String`].
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

impl From<Decimal> for PredicateValue {
    fn from(value: Decimal) -> Self {
        Self::Decimal(value)
    }
}

impl From<bool> for PredicateValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<MetricRef> for PredicateValue {
    fn from(value: MetricRef) -> Self {
        Self::MetricRef(value)
    }
}

impl From<Vec<PredicateValue>> for PredicateValue {
    fn from(value: Vec<PredicateValue>) -> Self {
        Self::List(value)
    }
}

impl From<&str> for PredicateValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for PredicateValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

fn is_non_finite_float_literal(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "nan" | "+nan" | "-nan" | "inf" | "+inf" | "-inf" | "infinity" | "+infinity" | "-infinity"
    )
}

impl<'de> Deserialize<'de> for PredicateValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PredicateValueVisitor;

        impl<'de> serde::de::Visitor<'de> for PredicateValueVisitor {
            type Value = PredicateValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(
                    "an exact decimal string, integer, bool, MetricRef object, list, or string (no floats/NaN)",
                )
            }

            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(PredicateValue::Bool(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(PredicateValue::Decimal(Decimal::from(v)))
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(PredicateValue::Decimal(Decimal::from(v)))
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Err(E::custom(format!(
                    "floating-point number '{v}' is forbidden in PredicateValue; pass an exact decimal string instead"
                )))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                if is_non_finite_float_literal(v) {
                    return Err(E::custom(format!(
                        "non-finite float literal '{v}' is forbidden in PredicateValue"
                    )));
                }
                if let Ok(d) = Decimal::from_str(v) {
                    return Ok(PredicateValue::Decimal(d));
                }
                Ok(PredicateValue::String(v.to_owned()))
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                self.visit_str(&v)
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(elem) = seq.next_element()? {
                    items.push(elem);
                }
                Ok(PredicateValue::List(items))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> Result<Self::Value, A::Error> {
                let metric_ref = MetricRef::deserialize(MapAccessDeserializer::new(map))?;
                Ok(PredicateValue::MetricRef(metric_ref))
            }
        }

        deserializer.deserialize_any(PredicateValueVisitor)
    }
}

/// One filter condition in a screener scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ScreenerFilterPredicate {
    /// Metric being filtered (with optional period and timeframe dimensions).
    pub metric: MetricRef,
    /// Comparison operator.
    pub op: FilterOp,
    /// Comparison value (`Decimal`, `String`, `Bool`, `MetricRef`, or `List`).
    pub value: PredicateValue,
    /// Optional peer/benchmark scope for relative comparisons (e.g. `pe < sector_median(pe)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_to: Option<RelativeTo>,
    /// Optional aggregation statistic for relative comparisons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_stat: Option<RelativeStat>,
}

impl ScreenerFilterPredicate {
    /// Create a predicate comparing `metric` against `value` with `op`.
    #[must_use]
    pub fn new(metric: MetricRef, op: FilterOp, value: PredicateValue) -> Self {
        Self {
            metric,
            op,
            value,
            relative_to: None,
            relative_stat: None,
        }
    }

    /// Create an inclusive `Between` predicate `[low, high]`.
    #[must_use]
    pub fn between(metric: MetricRef, low: PredicateValue, high: PredicateValue) -> Self {
        Self {
            metric,
            op: FilterOp::Between,
            value: PredicateValue::List(vec![low, high]),
            relative_to: None,
            relative_stat: None,
        }
    }

    /// Attach a peer/benchmark relative comparison scope and statistic.
    #[must_use]
    pub fn with_relative(mut self, relative_to: RelativeTo, relative_stat: RelativeStat) -> Self {
        self.relative_to = Some(relative_to);
        self.relative_stat = Some(relative_stat);
        self
    }

    /// Extract `[low, high]` bounds if `op == FilterOp::Between` and `value` is a 2-element list.
    #[must_use]
    pub fn between_bounds(&self) -> Option<(&PredicateValue, &PredicateValue)> {
        if self.op != FilterOp::Between {
            return None;
        }
        match &self.value {
            PredicateValue::List(items) if items.len() == 2 => Some((&items[0], &items[1])),
            _ => None,
        }
    }

    /// Validate structural invariants of this predicate (non-empty key, valid operand shape).
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] if the key is empty or the operand shape
    /// is incompatible with `self.op`.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.metric.0.key.trim().is_empty() {
            return Err(CoreError::Validation(
                "screener predicate metric key cannot be empty".to_owned(),
            ));
        }
        let is_numeric_or_metric = |v: &PredicateValue| {
            matches!(v, PredicateValue::Decimal(_) | PredicateValue::MetricRef(_))
        };
        match self.op {
            FilterOp::Between => {
                let bounds = self
                    .value
                    .as_list()
                    .filter(|l| l.len() == 2)
                    .ok_or_else(|| {
                        CoreError::Validation(
                            "`between` requires PredicateValue::List of exactly 2 operands"
                                .to_owned(),
                        )
                    })?;
                if !bounds.iter().all(is_numeric_or_metric) {
                    return Err(CoreError::Validation(
                        "`between` bounds must be Decimal or MetricRef values".to_owned(),
                    ));
                }
            }
            FilterOp::In | FilterOp::NotIn => {
                if self.value.as_list().is_none() {
                    return Err(CoreError::Validation(format!(
                        "`{}` requires PredicateValue::List",
                        self.op
                    )));
                }
            }
            FilterOp::CrossesAbove | FilterOp::CrossesBelow => {
                if !is_numeric_or_metric(&self.value) {
                    return Err(CoreError::Validation(format!(
                        "`{}` requires a Decimal or MetricRef operand",
                        self.op
                    )));
                }
            }
            FilterOp::Gt | FilterOp::Gte | FilterOp::Lt | FilterOp::Lte => {
                if !is_numeric_or_metric(&self.value) {
                    return Err(CoreError::Validation(format!(
                        "`{}` requires a Decimal or MetricRef operand",
                        self.op
                    )));
                }
            }
            FilterOp::Like | FilterOp::Has => {
                if matches!(self.value, PredicateValue::List(_)) {
                    return Err(CoreError::Validation(format!(
                        "`{}` requires a scalar operand",
                        self.op
                    )));
                }
            }
            FilterOp::Eq | FilterOp::Ne => {}
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScreenerFilterPredicateWire {
    #[serde(default)]
    metric: Option<MetricRef>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    period: Option<MetricPeriod>,
    #[serde(default)]
    timeframe: Option<Timeframe>,
    op: FilterOp,
    value: PredicateValue,
    #[serde(default)]
    value_upper: Option<Decimal>,
    #[serde(default)]
    relative_to: Option<RelativeTo>,
    #[serde(default)]
    relative_stat: Option<RelativeStat>,
}

impl<'de> Deserialize<'de> for ScreenerFilterPredicate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = ScreenerFilterPredicateWire::deserialize(deserializer)?;
        let metric = match (wire.metric, wire.key) {
            (Some(mut m), None) => {
                if m.0.period.is_none() {
                    m.0.period = wire.period;
                }
                if m.0.timeframe.is_none() {
                    m.0.timeframe = wire.timeframe;
                }
                m
            }
            (None, Some(key)) => MetricRef(MetricKeySpec {
                key,
                period: wire.period,
                timeframe: wire.timeframe,
            }),
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "specify either `metric` or flat `key`, not both",
                ));
            }
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "missing metric specification (`metric` or `key`)",
                ));
            }
        };

        let value = match (wire.op, wire.value, wire.value_upper) {
            (FilterOp::Between, PredicateValue::Decimal(lo), Some(hi)) => {
                PredicateValue::List(vec![
                    PredicateValue::Decimal(lo),
                    PredicateValue::Decimal(hi),
                ])
            }
            (FilterOp::Between, other, Some(hi)) => {
                return Err(serde::de::Error::custom(format!(
                    "legacy `value_upper` ({hi}) can only pair with scalar lower bound, got {other:?}"
                )));
            }
            (_, _, Some(_)) => {
                return Err(serde::de::Error::custom(
                    "`value_upper` is only valid when `op` is `between`",
                ));
            }
            (_, v, None) => v,
        };

        Ok(Self {
            metric,
            op: wire.op,
            value,
            relative_to: wire.relative_to,
            relative_stat: wire.relative_stat,
        })
    }
}

/// One node in a [`ScreenerFilterGroup`]: either a leaf predicate or a nested group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ScreenerFilterItem {
    /// A single predicate condition.
    Predicate(ScreenerFilterPredicate),
    /// A nested filter group combined with `AND` / `OR`.
    Group(Box<ScreenerFilterGroup>),
}

/// Recursive boolean group of filter predicates and nested groups, with optional
/// classification scoping.
///
/// Accepts legacy v8 flat payloads (`{"predicates": [...], "match_all": true}`)
/// at deserialization time and upgrades them to the canonical recursive form.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, JsonSchema)]
pub struct ScreenerFilterGroup {
    /// Combinator applied across `items` (`AND` or `OR`).
    #[serde(default)]
    pub operator: LogicalOp,
    /// Predicates and nested groups.
    #[serde(default)]
    pub items: Vec<ScreenerFilterItem>,
    /// Optional classification scope (sector, industry, cap bucket, index, theme).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<ClassificationFilter>,
}

impl ScreenerFilterGroup {
    /// Create an empty `AND` group (matches any instrument).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create an `AND` group from the given items.
    #[must_use]
    pub fn and(items: Vec<ScreenerFilterItem>) -> Self {
        Self {
            operator: LogicalOp::And,
            items,
            classification: None,
        }
    }

    /// Create an `OR` group from the given items.
    #[must_use]
    pub fn or(items: Vec<ScreenerFilterItem>) -> Self {
        Self {
            operator: LogicalOp::Or,
            items,
            classification: None,
        }
    }

    /// Attach a [`ClassificationFilter`] scope to this group.
    #[must_use]
    pub fn with_classification(mut self, classification: ClassificationFilter) -> Self {
        self.classification = Some(classification);
        self
    }

    /// Flatten and collect references to all leaf [`ScreenerFilterPredicate`]s in pre-order.
    #[must_use]
    pub fn predicates(&self) -> Vec<&ScreenerFilterPredicate> {
        let mut out = Vec::new();
        self.collect_predicates_into(&mut out);
        out
    }

    fn collect_predicates_into<'a>(&'a self, out: &mut Vec<&'a ScreenerFilterPredicate>) {
        for item in &self.items {
            match item {
                ScreenerFilterItem::Predicate(p) => out.push(p),
                ScreenerFilterItem::Group(g) => g.collect_predicates_into(out),
            }
        }
    }

    /// Collect all referenced metric keys (both LHS and `PredicateValue::MetricRef` RHS)
    /// in deterministic first-seen order without duplicates.
    #[must_use]
    pub fn metric_keys(&self) -> Vec<String> {
        fn push_unique(key: &str, keys: &mut Vec<String>) {
            if !keys.iter().any(|k| k == key) {
                keys.push(key.to_owned());
            }
        }
        fn collect_value(val: &PredicateValue, keys: &mut Vec<String>) {
            match val {
                PredicateValue::MetricRef(m) => push_unique(&m.0.key, keys),
                PredicateValue::List(items) => {
                    for item in items {
                        collect_value(item, keys);
                    }
                }
                _ => {}
            }
        }
        fn walk(group: &ScreenerFilterGroup, keys: &mut Vec<String>) {
            for item in &group.items {
                match item {
                    ScreenerFilterItem::Predicate(p) => {
                        push_unique(&p.metric.0.key, keys);
                        collect_value(&p.value, keys);
                    }
                    ScreenerFilterItem::Group(nested) => walk(nested, keys),
                }
            }
        }
        let mut keys = Vec::new();
        walk(self, &mut keys);
        keys
    }

    /// Validate all predicates and nested groups recursively.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] if any predicate in the tree is invalid.
    pub fn validate(&self) -> Result<(), CoreError> {
        for item in &self.items {
            match item {
                ScreenerFilterItem::Predicate(p) => p.validate()?,
                ScreenerFilterItem::Group(nested) => nested.validate()?,
            }
        }
        Ok(())
    }
}

const fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScreenerFilterGroupV8Flat {
    predicates: Vec<ScreenerFilterPredicate>,
    #[serde(default = "default_true")]
    match_all: bool,
    #[serde(default)]
    classification: Option<ClassificationFilter>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScreenerFilterGroupCanonical {
    #[serde(default)]
    operator: LogicalOp,
    #[serde(default)]
    items: Vec<ScreenerFilterItem>,
    #[serde(default)]
    classification: Option<ClassificationFilter>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ScreenerFilterGroupWire {
    V8Flat(ScreenerFilterGroupV8Flat),
    Canonical(ScreenerFilterGroupCanonical),
}

impl<'de> Deserialize<'de> for ScreenerFilterGroup {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match ScreenerFilterGroupWire::deserialize(deserializer)? {
            ScreenerFilterGroupWire::V8Flat(v8) => Ok(Self {
                operator: if v8.match_all {
                    LogicalOp::And
                } else {
                    LogicalOp::Or
                },
                items: v8
                    .predicates
                    .into_iter()
                    .map(ScreenerFilterItem::Predicate)
                    .collect(),
                classification: v8.classification,
            }),
            ScreenerFilterGroupWire::Canonical(c) => Ok(Self {
                operator: c.operator,
                items: c.items,
                classification: c.classification,
            }),
        }
    }
}

/// How to order screener results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SortSpec {
    /// Metric to sort by.
    pub metric: MetricRef,
    /// `true` for descending order.
    pub descending: bool,
}

/// Request to scan instruments against a converged filter tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ScreenerScanRequest {
    /// Instruments to scan. `None` means the full universe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub universe: Option<Vec<InstrumentId>>,
    /// Filter tree to apply.
    pub filters: ScreenerFilterGroup,
    /// Sort order for the result set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<SortSpec>,
    /// Maximum rows to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// One instrument's row in a screener result, keyed by metric name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ScreenerRow {
    /// The matching instrument.
    pub instrument: InstrumentId,
    /// Metric name → value, for every metric the request asked about.
    pub metrics: BTreeMap<String, Decimal>,
}

/// The matching rows for a [`ScreenerScanRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ScreenerScanResponse {
    /// Matching rows, after sort/limit.
    pub rows: Vec<ScreenerRow>,
    /// Total rows that matched the filter, before `limit` was applied.
    pub total_matched: u32,
}
