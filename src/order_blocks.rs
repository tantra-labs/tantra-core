//! Order-block domain types (Elephant Edge / EE+).
//!
//! An order block is a footprint-candle imbalance zone with a direction,
//! feeding the [`crate::LevelMap`] and downstream planner inputs. Stop/trail
//! mechanics are delegated to the trade-plan slice (plan 0005,
//! `src/trade_plan.rs`) and do not live here.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::levels::LevelSide;

/// An order block: a footprint-candle imbalance zone with a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OrderBlock {
    /// When the order block was formed (injected — zero wall clock).
    pub ts: DateTime<Utc>,
    /// Which side the block faces (support vs resistance).
    pub side: LevelSide,
    /// Price zone `(low, high)` with `low <= high`.
    pub zone: (Decimal, Decimal),
}

impl OrderBlock {
    /// Validated constructor.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] when `zone.0 > zone.1`.
    pub fn new(
        ts: DateTime<Utc>,
        side: LevelSide,
        zone: (Decimal, Decimal),
    ) -> Result<Self, CoreError> {
        if zone.0 > zone.1 {
            return Err(CoreError::Validation(format!(
                "order-block zone low must not exceed high: {} > {}",
                zone.0, zone.1
            )));
        }
        Ok(Self { ts, side, zone })
    }
}
