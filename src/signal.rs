//! Strategy/indicator signal wire contracts.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::types::{InstrumentId, Price};

/// Standard bar/candle interval shared across signals, bars, and screener predicates.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Timeframe {
    /// 1 minute.
    #[serde(alias = "1", alias = "1m", alias = "M1")]
    M1,
    /// 5 minutes.
    #[serde(alias = "5", alias = "5m", alias = "M5")]
    M5,
    /// 15 minutes.
    #[serde(alias = "15", alias = "15m", alias = "M15")]
    M15,
    /// 30 minutes.
    #[serde(alias = "30", alias = "30m", alias = "M30")]
    M30,
    /// 1 hour.
    #[serde(alias = "60", alias = "1h", alias = "H1")]
    H1,
    /// 2 hours.
    #[serde(alias = "120", alias = "2h", alias = "H2")]
    H2,
    /// 4 hours.
    #[serde(alias = "240", alias = "4h", alias = "H4")]
    H4,
    /// 1 day.
    #[serde(alias = "1D", alias = "1d", alias = "D1")]
    D1,
    /// 1 week.
    #[serde(alias = "1W", alias = "1w", alias = "W1")]
    W1,
    /// 1 calendar month.
    #[serde(alias = "1M", alias = "1mo", alias = "Month1", alias = "MONTH1")]
    Month1,
}

impl Timeframe {
    /// Parses canonical (`m1`, `d1`, ...) or screener-wire (`1m`, `60`, `1h`, `1D`, `1W`, `1M`)
    /// timeframe strings.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] if `text` is not a supported timeframe.
    pub fn parse_wire(text: &str) -> Result<Self, CoreError> {
        match text.trim() {
            "m1" | "M1" | "1" | "1m" => Ok(Self::M1),
            "m5" | "M5" | "5" | "5m" => Ok(Self::M5),
            "m15" | "M15" | "15" | "15m" => Ok(Self::M15),
            "m30" | "M30" | "30" | "30m" => Ok(Self::M30),
            "h1" | "H1" | "60" | "1h" => Ok(Self::H1),
            "h2" | "H2" | "120" | "2h" => Ok(Self::H2),
            "h4" | "H4" | "240" | "4h" => Ok(Self::H4),
            "d1" | "D1" | "1D" | "1d" => Ok(Self::D1),
            "w1" | "W1" | "1W" | "1w" => Ok(Self::W1),
            "month1" | "Month1" | "MONTH1" | "1M" | "1mo" => Ok(Self::Month1),
            other => Err(CoreError::Validation(format!(
                "unsupported timeframe: '{other}'"
            ))),
        }
    }

    /// Canonical snake-case label (`"m1"`, `"d1"`, `"month1"`, ...).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::M1 => "m1",
            Self::M5 => "m5",
            Self::M15 => "m15",
            Self::M30 => "m30",
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H4 => "h4",
            Self::D1 => "d1",
            Self::W1 => "w1",
            Self::Month1 => "month1",
        }
    }

    /// Compact screener wire label (`"1m"`, `"1h"`, `"1D"`, `"1W"`, `"1M"`, ...).
    #[must_use]
    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H2 => "2h",
            Self::H4 => "4h",
            Self::D1 => "1D",
            Self::W1 => "1W",
            Self::Month1 => "1M",
        }
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_wire_str())
    }
}

impl FromStr for Timeframe {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_wire(s)
    }
}

/// The directional bias a signal expresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SignalDirection {
    /// Expects price to rise.
    Bullish,
    /// Expects price to fall.
    Bearish,
    /// No directional edge.
    Neutral,
}

/// The holding horizon a signal is meant for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SignalHorizon {
    /// Closed out same day.
    Intraday,
    /// Held days to a few weeks.
    Swing,
    /// Held weeks to months.
    Positional,
}

/// A single strategy/indicator signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Signal {
    /// The instrument this signal is about.
    pub instrument: InstrumentId,
    /// Directional bias.
    pub direction: SignalDirection,
    /// Intended holding horizon.
    pub horizon: SignalHorizon,
    /// Bar interval the signal was computed on.
    pub timeframe: Timeframe,
    /// Confidence, in `[0, 1]`.
    pub strength: Decimal,
    /// Strategy/algo id that generated this signal.
    pub generated_by: String,
    /// When the signal was generated.
    pub timestamp: DateTime<Utc>,
    /// Target entry price for trade execution, if recommended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_price: Option<Price>,
    /// Recommended stop loss price level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_loss: Option<Price>,
    /// Recommended take profit price level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take_profit: Option<Price>,
    /// Current or computed volatility (e.g. ATR).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volatility_atr: Option<Decimal>,
}

impl Signal {
    /// Creates a new signal with core attributes and execution levels unset.
    pub fn new(
        instrument: InstrumentId,
        direction: SignalDirection,
        horizon: SignalHorizon,
        timeframe: Timeframe,
        strength: Decimal,
        generated_by: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            instrument,
            direction,
            horizon,
            timeframe,
            strength,
            generated_by: generated_by.into(),
            timestamp,
            entry_price: None,
            stop_loss: None,
            take_profit: None,
            volatility_atr: None,
        }
    }

    /// Attach a recommended entry price.
    pub fn with_entry_price(mut self, price: Price) -> Self {
        self.entry_price = Some(price);
        self
    }

    /// Attach a recommended stop loss level.
    pub fn with_stop_loss(mut self, stop_loss: Price) -> Self {
        self.stop_loss = Some(stop_loss);
        self
    }

    /// Attach a recommended take profit level.
    pub fn with_take_profit(mut self, take_profit: Price) -> Self {
        self.take_profit = Some(take_profit);
        self
    }

    /// Attach a volatility estimate (e.g. ATR).
    pub fn with_volatility_atr(mut self, volatility_atr: Decimal) -> Self {
        self.volatility_atr = Some(volatility_atr);
        self
    }
}

/// Request to scan for signals matching the given criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SignalScanRequest {
    /// Instruments to scan. Empty means the full universe.
    pub universe: Vec<InstrumentId>,
    /// Restrict to this horizon, if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<SignalHorizon>,
    /// Restrict to this direction, if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<SignalDirection>,
    /// Restrict to this timeframe, if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<Timeframe>,
}

/// The matching signals for a [`SignalScanRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SignalScanResponse {
    /// Matching signals.
    pub signals: Vec<Signal>,
}
