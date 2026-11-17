//! Intraday level-map domain types (Elephant Edge / EE+).
//!
//! The level map is the session's canonical map of support/resistance levels —
//! Elephant percentile zones, open-interest walls, the gamma flip, max pain,
//! round-number strikes, and order blocks — each with a price band, a side, a
//! kind, and a confluence score. Zero-I/O, `Decimal`-only, WASM-clean.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Maximum confluence score a [`Level`] can carry (`0..=4`).
pub const MAX_LEVEL_SCORE: u8 = 4;

/// Kind of a level in the intraday level map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LevelKind {
    /// Elephant percentile support zone 1 (closest support).
    ElephantSupport1,
    /// Elephant percentile support zone 2.
    ElephantSupport2,
    /// Elephant percentile resistance zone 1 (closest resistance).
    ElephantResistance1,
    /// Elephant percentile resistance zone 2.
    ElephantResistance2,
    /// Previous day's open-high-low-close reference level.
    PreviousDayOhlc,
    /// Call-side open-interest wall.
    OiWallCall,
    /// Put-side open-interest wall.
    OiWallPut,
    /// Gamma flip — the strike where cumulative GEX changes sign.
    GammaFlip,
    /// Call-side gamma wall.
    GammaWallCall,
    /// Put-side gamma wall.
    GammaWallPut,
    /// Max-pain strike.
    MaxPain,
    /// Round-number strike (psychological level).
    StrikeRound,
    /// Order-block zone.
    OrderBlock,
}

/// Side of a level: whether it acts as support (below) or resistance (above).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LevelSide {
    /// Support — price is expected to find buyers here.
    Support,
    /// Resistance — price is expected to find sellers here.
    Resistance,
}

/// Stable identifier of a [`Level`], referenced by signals and positioning.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct LevelId(pub u64);

/// A single level on the intraday map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Level {
    /// Stable level identifier.
    pub id: LevelId,
    /// What kind of level this is.
    pub kind: LevelKind,
    /// Whether this level is support or resistance.
    pub side: LevelSide,
    /// Price band `(low, high)` with `low <= high`; a degenerate band
    /// (`low == high`) denotes an exact strike/level.
    pub band: (Decimal, Decimal),
    /// Confluence score in `0..=4` (how many independent signals agree).
    pub score: u8,
}

impl Level {
    /// Validated constructor.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] when `band.0 > band.1` or
    /// `score > MAX_LEVEL_SCORE`.
    pub fn new(
        id: LevelId,
        kind: LevelKind,
        side: LevelSide,
        band: (Decimal, Decimal),
        score: u8,
    ) -> Result<Self, CoreError> {
        if band.0 > band.1 {
            return Err(CoreError::Validation(format!(
                "level band low must not exceed high: {} > {}",
                band.0, band.1
            )));
        }
        if score > MAX_LEVEL_SCORE {
            return Err(CoreError::Validation(format!(
                "level confluence score must be 0..={MAX_LEVEL_SCORE}: {score}"
            )));
        }
        Ok(Self {
            id,
            kind,
            side,
            band,
            score,
        })
    }
}

/// The session's intraday level map for one instrument.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct LevelMap {
    /// All levels on the map, in no guaranteed order.
    pub levels: Vec<Level>,
}

impl LevelMap {
    /// Create a level map from a list of levels.
    pub fn new(levels: Vec<Level>) -> Self {
        Self { levels }
    }

    /// Look up a level by its stable identifier.
    pub fn get(&self, id: LevelId) -> Option<&Level> {
        self.levels.iter().find(|level| level.id == id)
    }

    /// All support levels, in map order.
    pub fn support_levels(&self) -> impl Iterator<Item = &Level> {
        self.levels
            .iter()
            .filter(|level| level.side == LevelSide::Support)
    }

    /// All resistance levels, in map order.
    pub fn resistance_levels(&self) -> impl Iterator<Item = &Level> {
        self.levels
            .iter()
            .filter(|level| level.side == LevelSide::Resistance)
    }
}
