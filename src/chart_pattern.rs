//! Chart-pattern domain types for pattern-driven trade planning (EE+).
//!
//! [`ChartPattern`] is the validated, geometry-consistent wire description of a
//! detected chart pattern (flag, pennant, triangle, Darvas box, cup-and-handle,
//! VCP, multi-bar candlestick, ...). Detection lives in `tantra-market` (plan
//! 0002); this crate owns only the contract. Zero-I/O, `Decimal`-only,
//! WASM-clean.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::signal::{SignalDirection, Timeframe};
use crate::types::Price;

/// Maximum chart-pattern quality score (`0..=100`).
pub const MAX_PATTERN_QUALITY: u8 = 100;

/// Kind of a chart pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    /// Flag continuation pattern.
    Flag,
    /// Pennant continuation pattern.
    Pennant,
    /// Ascending triangle.
    TriangleAscending,
    /// Descending triangle.
    TriangleDescending,
    /// Symmetric triangle.
    TriangleSymmetric,
    /// Rectangle consolidation.
    Rectangle,
    /// Darvas box breakout.
    DarvasBox,
    /// Double top reversal.
    DoubleTop,
    /// Double bottom reversal.
    DoubleBottom,
    /// Head-and-shoulders top.
    HeadAndShoulders,
    /// Inverse head-and-shoulders bottom.
    InverseHeadAndShoulders,
    /// Wedge (rising or falling).
    Wedge,
    /// Cup-and-handle continuation.
    CupAndHandle,
    /// Volatility Contraction Pattern (VCP).
    Vcp,
    /// Multi-bar candlestick setup.
    MultiBarCandlestick,
}

/// Lifecycle stage of a chart pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PatternStage {
    /// Pattern is fully formed but the breakout has not yet confirmed on a
    /// close; the planner enters with a resting breakout trigger.
    FormingArmed,
    /// The breakout has confirmed on a close; the planner enters at market or
    /// on a pullback limit.
    BreakoutConfirmed,
}

impl PatternStage {
    /// Transition to the confirmed stage. Idempotent: a confirmed stage stays
    /// confirmed.
    #[must_use]
    pub const fn confirm(self) -> Self {
        match self {
            Self::FormingArmed => Self::BreakoutConfirmed,
            Self::BreakoutConfirmed => Self::BreakoutConfirmed,
        }
    }

    /// Returns `true` when the pattern breakout has confirmed.
    #[must_use]
    pub const fn is_confirmed(self) -> bool {
        matches!(self, Self::BreakoutConfirmed)
    }
}

/// A single swing pivot used to describe a pattern's geometry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PatternPivot {
    /// Bar timestamp the pivot printed on (injected — zero wall clock).
    pub bar_ts: DateTime<Utc>,
    /// Pivot price.
    pub price: Price,
    /// Pivot kind label (`"high"`, `"low"`, `"close"`, ...).
    pub kind: String,
}

/// A validated chart pattern with consistent breakout geometry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChartPattern {
    /// Pattern kind.
    pub kind: PatternKind,
    /// Direction the pattern resolves in.
    pub direction: SignalDirection,
    /// Bar interval the pattern was detected on.
    pub timeframe: Timeframe,
    /// Lifecycle stage.
    pub stage: PatternStage,
    /// Swing pivots composing the pattern.
    pub pivots: Vec<PatternPivot>,
    /// Price that must be taken out to confirm the pattern.
    pub breakout_level: Price,
    /// Price that, if taken out first, invalidates the pattern (always on the
    /// losing side of [`ChartPattern::breakout_level`]).
    pub invalidation_level: Price,
    /// Price target implied by the pattern's measured height, when known
    /// (always on the winning side of [`ChartPattern::breakout_level`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured_target: Option<Price>,
    /// Pattern quality in `0..=100`.
    pub quality: u8,
    /// When the pattern first formed.
    pub formed_from: DateTime<Utc>,
    /// When the pattern was armed for a breakout entry.
    pub armed_at: DateTime<Utc>,
    /// When the breakout confirmed on a close, if it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_at: Option<DateTime<Utc>>,
}

impl ChartPattern {
    /// Validated constructor.
    ///
    /// Enforces pattern geometry consistency:
    /// - `invalidation_level` sits strictly on the losing side of `breakout_level`
    ///   (below it for [`SignalDirection::Bullish`], above it for
    ///   [`SignalDirection::Bearish`]),
    /// - `measured_target`, when `Some`, sits strictly on the winning side,
    /// - `formed_from <= armed_at`,
    /// - `quality <= 100`.
    ///
    /// Geometry checks are skipped for [`SignalDirection::Neutral`] (no
    /// canonical winning/losing side).
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] on any violated invariant.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: PatternKind,
        direction: SignalDirection,
        timeframe: Timeframe,
        stage: PatternStage,
        pivots: Vec<PatternPivot>,
        breakout_level: Price,
        invalidation_level: Price,
        measured_target: Option<Price>,
        quality: u8,
        formed_from: DateTime<Utc>,
        armed_at: DateTime<Utc>,
        confirmed_at: Option<DateTime<Utc>>,
    ) -> Result<Self, CoreError> {
        if formed_from > armed_at {
            return Err(CoreError::Validation(format!(
                "pattern formed_from must not be after armed_at: {formed_from} > {armed_at}"
            )));
        }
        if quality > MAX_PATTERN_QUALITY {
            return Err(CoreError::Validation(format!(
                "pattern quality must be 0..={MAX_PATTERN_QUALITY}: {quality}"
            )));
        }
        match direction {
            SignalDirection::Bullish => {
                if invalidation_level >= breakout_level {
                    return Err(CoreError::Validation(format!(
                        "bullish pattern invalidation ({invalidation_level}) must sit below breakout level ({breakout_level})"
                    )));
                }
                if let Some(target) = measured_target {
                    if target <= breakout_level {
                        return Err(CoreError::Validation(format!(
                            "bullish pattern measured target ({target}) must sit above breakout level ({breakout_level})"
                        )));
                    }
                }
            }
            SignalDirection::Bearish => {
                if invalidation_level <= breakout_level {
                    return Err(CoreError::Validation(format!(
                        "bearish pattern invalidation ({invalidation_level}) must sit above breakout level ({breakout_level})"
                    )));
                }
                if let Some(target) = measured_target {
                    if target >= breakout_level {
                        return Err(CoreError::Validation(format!(
                            "bearish pattern measured target ({target}) must sit below breakout level ({breakout_level})"
                        )));
                    }
                }
            }
            SignalDirection::Neutral => {
                // No canonical winning/losing side; geometry is unconstrained.
            }
        }
        Ok(Self {
            kind,
            direction,
            timeframe,
            stage,
            pivots,
            breakout_level,
            invalidation_level,
            measured_target,
            quality,
            formed_from,
            armed_at,
            confirmed_at,
        })
    }

    /// The pattern's measured height: `|breakout_level - invalidation_level|`.
    #[must_use]
    pub fn measured_height(&self) -> Decimal {
        (self.breakout_level.value() - self.invalidation_level.value()).abs()
    }
}
