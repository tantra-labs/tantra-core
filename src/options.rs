//! Options-chain positioning and gamma-exposure domain types (Elephant Edge / EE+).
//!
//! Per-strike open-interest / implied-volatility / Greek positioning and the
//! derived gamma-exposure surface (GEX), including the gamma flip — the strike
//! at which cumulative GEX changes sign. Zero-I/O, `Decimal`-only, WASM-clean.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::types::{InstrumentId, Price};

/// A snapshot of the option chain for one expiry of one underlying.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OptionChainSnapshot {
    /// When the snapshot was taken (injected by the caller — zero wall clock).
    pub as_of: DateTime<Utc>,
    /// The underlying instrument.
    pub underlying: InstrumentId,
    /// Expiry date of the struck series.
    pub expiry: NaiveDate,
    /// Per-strike positioning, one entry per strike.
    pub strikes: Vec<StrikePositioning>,
}

impl OptionChainSnapshot {
    /// Create a new chain snapshot.
    pub fn new(
        as_of: DateTime<Utc>,
        underlying: InstrumentId,
        expiry: NaiveDate,
        strikes: Vec<StrikePositioning>,
    ) -> Self {
        Self {
            as_of,
            underlying,
            expiry,
            strikes,
        }
    }
}

/// Per-strike open-interest, implied-volatility and Greek positioning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StrikePositioning {
    /// Strike price.
    pub strike: Price,
    /// Call open interest (contracts).
    pub call_oi: Decimal,
    /// Put open interest (contracts).
    pub put_oi: Decimal,
    /// Call open-interest change vs. the previous snapshot (may be negative).
    pub call_oi_change: Decimal,
    /// Put open-interest change vs. the previous snapshot (may be negative).
    pub put_oi_change: Decimal,
    /// Call implied volatility as a fraction (e.g. `0.185` for 18.5%).
    pub call_iv: Decimal,
    /// Put implied volatility as a fraction.
    pub put_iv: Decimal,
    /// Call delta.
    pub call_delta: Decimal,
    /// Put delta.
    pub put_delta: Decimal,
    /// Call gamma.
    pub call_gamma: Decimal,
    /// Put gamma.
    pub put_gamma: Decimal,
}

impl StrikePositioning {
    /// Validated constructor.
    ///
    /// OI *levels*, implied volatilities, and gamma must all be non-negative;
    /// OI *changes* and deltas may legitimately be negative (OI unwinds, put
    /// deltas are negative in market convention).
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] when any OI, IV, or gamma value is
    /// negative.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        strike: Price,
        call_oi: Decimal,
        put_oi: Decimal,
        call_oi_change: Decimal,
        put_oi_change: Decimal,
        call_iv: Decimal,
        put_iv: Decimal,
        call_delta: Decimal,
        put_delta: Decimal,
        call_gamma: Decimal,
        put_gamma: Decimal,
    ) -> Result<Self, CoreError> {
        for (field, value) in [
            ("call_oi", call_oi),
            ("put_oi", put_oi),
            ("call_iv", call_iv),
            ("put_iv", put_iv),
            ("call_gamma", call_gamma),
            ("put_gamma", put_gamma),
        ] {
            if value < Decimal::ZERO {
                return Err(CoreError::Validation(format!(
                    "{field} must not be negative: {value}"
                )));
            }
        }
        Ok(Self {
            strike,
            call_oi,
            put_oi,
            call_oi_change,
            put_oi_change,
            call_iv,
            put_iv,
            call_delta,
            put_delta,
            call_gamma,
            put_gamma,
        })
    }
}

/// Gamma exposure of a single strike, derived from the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StrikeGamma {
    /// Strike price.
    pub strike: Price,
    /// Net gamma at this strike.
    pub gamma: Decimal,
    /// Open interest at this strike.
    pub oi: Decimal,
    /// Gamma exposure (GEX) contribution of this strike (may be negative on the
    /// put side).
    pub gex: Decimal,
}

/// Aggregate gamma-exposure surface across the chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GammaExposure {
    /// Per-strike GEX, one entry per strike.
    pub per_strike: Vec<StrikeGamma>,
    /// Net (total) GEX across the whole chain.
    pub net: Decimal,
    /// The gamma flip: the strike at which cumulative GEX changes sign, or
    /// `Decimal::ZERO` when no flip is detected.
    pub flip: Decimal,
}

impl GammaExposure {
    /// Compute the gamma-exposure surface from per-strike data.
    ///
    /// `net` is the algebraic sum of per-strike GEX; `flip` is derived by
    /// [`gamma_flip_strike`].
    pub fn compute(per_strike: Vec<StrikeGamma>) -> Self {
        let net: Decimal = per_strike.iter().map(|s| s.gex).sum();
        let flip = gamma_flip_strike(&per_strike);
        Self {
            per_strike,
            net,
            flip,
        }
    }

    /// Total GEX — the algebraic sum of every per-strike contribution.
    pub fn total_gex(&self) -> Decimal {
        self.per_strike.iter().map(|s| s.gex).sum()
    }
}

/// Compute the gamma flip: the first strike (walking the chain from lowest to
/// highest strike) at which the cumulative GEX changes sign.
///
/// The chain does not need to be pre-sorted; it is sorted by ascending strike
/// internally. Returns `Decimal::ZERO` when the cumulative GEX never changes
/// sign (no flip detected).
pub fn gamma_flip_strike(per_strike: &[StrikeGamma]) -> Decimal {
    let mut sorted: Vec<&StrikeGamma> = per_strike.iter().collect();
    sorted.sort_by_key(|s| s.strike);
    let mut cumulative = Decimal::ZERO;
    let mut previous: Option<Decimal> = None;
    for strike_gamma in sorted {
        cumulative += strike_gamma.gex;
        if let Some(prev) = previous {
            if (prev < Decimal::ZERO) != (cumulative < Decimal::ZERO) {
                return strike_gamma.strike.value();
            }
        }
        previous = Some(cumulative);
    }
    Decimal::ZERO
}
