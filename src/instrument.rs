//! Instrument reference/master data — distinct from [`crate::types::InstrumentId`],
//! which is only the lookup key.

use std::fmt;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::types::{CountryCode, Currency, InstrumentId, Price, Quantity};

/// Canonical alias for [`Instrument`] highlighting that it serves as the full reference-data profile.
pub type InstrumentProfile = Instrument;

/// Broad instrument classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    /// Cash equity.
    Equity,
    /// Futures contract.
    Future,
    /// Options contract.
    Option,
    /// Index (not directly tradable; underlying for derivatives).
    Index,
}

/// Option right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OptionType {
    /// Call option.
    Call,
    /// Put option.
    Put,
}

/// Direction of a held or closed position.
///
/// Distinct from [`crate::order::OrderSide`], which is the side of a single
/// order or fill, not the resulting net position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum PositionSide {
    /// Net long.
    Long,
    /// Net short.
    Short,
}

impl PositionSide {
    /// Returns the opposite position side.
    pub fn opposite(&self) -> Self {
        match self {
            Self::Long => Self::Short,
            Self::Short => Self::Long,
        }
    }
}

#[derive(Deserialize)]
struct InstrumentRaw {
    id: InstrumentId,
    kind: InstrumentKind,
    #[serde(default)]
    currency: Currency,
    #[serde(default)]
    country: CountryCode,
    lot_size: Quantity,
    tick_size: Decimal,
    #[serde(default)]
    expiry: Option<DateTime<Utc>>,
    #[serde(default)]
    strike: Option<Price>,
    #[serde(default)]
    option_type: Option<OptionType>,
    #[serde(default)]
    underlying: Option<InstrumentId>,
}

/// Full reference-data record / profile for a tradable instrument.
///
/// Enforces `lot_size > 0` and `tick_size > 0` in both [`Instrument::try_new`]
/// and deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "InstrumentRaw")]
pub struct Instrument {
    /// Canonical lookup key.
    id: InstrumentId,
    /// Broad classification.
    kind: InstrumentKind,
    /// Quoted / base currency for pricing and valuation.
    #[serde(default)]
    currency: Currency,
    /// Country / market jurisdiction code (e.g. IN, US, UK, JP).
    #[serde(default)]
    country: CountryCode,
    /// Exchange-mandated lot size (strictly positive).
    lot_size: Quantity,
    /// Minimum price movement (strictly positive).
    tick_size: Decimal,
    /// Expiry date, for futures and options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expiry: Option<DateTime<Utc>>,
    /// Strike price, for options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    strike: Option<Price>,
    /// Call or put, for options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    option_type: Option<OptionType>,
    /// Underlying instrument, for derivatives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    underlying: Option<InstrumentId>,
}

impl TryFrom<InstrumentRaw> for Instrument {
    type Error = CoreError;

    fn try_from(raw: InstrumentRaw) -> Result<Self, Self::Error> {
        let mut inst = Self::try_new(raw.id, raw.kind, raw.lot_size, raw.tick_size)?
            .with_currency(raw.currency)
            .with_country(raw.country);
        inst.expiry = raw.expiry;
        inst.strike = raw.strike;
        inst.option_type = raw.option_type;
        inst.underlying = raw.underlying;
        Ok(inst)
    }
}

impl Instrument {
    /// Create a validated instrument record.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] if `lot_size <= 0` or `tick_size <= 0`.
    pub fn try_new(
        id: InstrumentId,
        kind: InstrumentKind,
        lot_size: Quantity,
        tick_size: Decimal,
    ) -> Result<Self, CoreError> {
        if !lot_size.is_positive() {
            return Err(CoreError::Validation(format!(
                "Instrument lot_size must be positive, got {lot_size}"
            )));
        }
        if tick_size <= Decimal::ZERO {
            return Err(CoreError::Validation(format!(
                "Instrument tick_size must be positive, got {tick_size}"
            )));
        }
        Ok(Self {
            id,
            kind,
            currency: Currency::default(),
            country: CountryCode::default(),
            lot_size,
            tick_size,
            expiry: None,
            strike: None,
            option_type: None,
            underlying: None,
        })
    }

    /// Create a validated cash-equity instrument record.
    pub fn try_equity(
        id: InstrumentId,
        lot_size: Quantity,
        tick_size: Decimal,
    ) -> Result<Self, CoreError> {
        Self::try_new(id, InstrumentKind::Equity, lot_size, tick_size)
    }

    /// Canonical lookup key.
    pub const fn id(&self) -> &InstrumentId {
        &self.id
    }

    /// Broad classification.
    pub const fn kind(&self) -> InstrumentKind {
        self.kind
    }

    /// Quoted / base currency for pricing and valuation.
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Country / market jurisdiction code.
    pub const fn country(&self) -> CountryCode {
        self.country
    }

    /// Exchange-mandated lot size (strictly positive).
    pub const fn lot_size(&self) -> Quantity {
        self.lot_size
    }

    /// Minimum price movement (strictly positive).
    pub const fn tick_size(&self) -> Decimal {
        self.tick_size
    }

    /// Expiry date, for futures and options.
    pub const fn expiry(&self) -> Option<DateTime<Utc>> {
        self.expiry
    }

    /// Strike price, for options.
    pub const fn strike(&self) -> Option<Price> {
        self.strike
    }

    /// Option right (call/put), for options.
    pub const fn option_type(&self) -> Option<OptionType> {
        self.option_type
    }

    /// Underlying instrument, for derivatives.
    pub const fn underlying(&self) -> Option<&InstrumentId> {
        self.underlying.as_ref()
    }

    /// Builder to set quoting currency.
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Builder to set country jurisdiction.
    pub fn with_country(mut self, country: CountryCode) -> Self {
        self.country = country;
        self
    }

    /// Builder to set expiry timestamp.
    pub fn with_expiry(mut self, expiry: DateTime<Utc>) -> Self {
        self.expiry = Some(expiry);
        self
    }

    /// Builder to set option strike price.
    pub fn with_strike(mut self, strike: Price) -> Self {
        self.strike = Some(strike);
        self
    }

    /// Builder to set option right.
    pub fn with_option_type(mut self, option_type: OptionType) -> Self {
        self.option_type = Some(option_type);
        self
    }

    /// Builder to set underlying instrument identifier.
    pub fn with_underlying(mut self, underlying: InstrumentId) -> Self {
        self.underlying = Some(underlying);
        self
    }
}

/// Typed identifier for a historical market-data dataset (distinct from any
/// single instrument or broker).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct DatasetId(pub String);

impl DatasetId {
    /// Wrap a raw dataset identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for DatasetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Typed identifier for a single executed trade/fill, distinct from the
/// platform's own order id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct TradeId(pub String);

impl TradeId {
    /// Wrap a raw trade identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Exchange/broker-assigned order identifier, distinct from this platform's
/// own `order_id` (see [`crate::order::OrderAck::order_id`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VenueOrderId(pub String);

impl VenueOrderId {
    /// Wrap a raw venue order identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for VenueOrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
