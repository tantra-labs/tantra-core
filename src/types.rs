//! Core value types for Indian equities (NSE, BSE, NFO, MCX, CDS) with Decimal precision.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::CoreError;

/// Supported exchanges in the Indian market.
///
/// This is the canonical [`Exchange`] vocabulary: downstream bounded contexts
/// (`tantra-ledger`, `tantra-market`, `tantra-exec`) must never redefine
/// their own exchange enums, only re-use this one.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Exchange {
    /// National Stock Exchange of India
    Nse,
    /// Bombay Stock Exchange
    Bse,
    /// NSE Futures & Options segment (derivatives)
    Nfo,
    /// Multi Commodity Exchange of India
    Mcx,
    /// Currency Derivatives Segment
    Cds,
}

impl Exchange {
    /// The exchange code as it appears on the wire (`"NSE"`, `"BSE"`, `"NFO"`, `"MCX"`, `"CDS"`).
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Nse => "NSE",
            Self::Bse => "BSE",
            Self::Nfo => "NFO",
            Self::Mcx => "MCX",
            Self::Cds => "CDS",
        }
    }

    /// Settlement currency of this venue. Every supported exchange is Indian and settles in INR;
    /// this is the single place a non-INR venue would declare its currency.
    pub const fn currency(&self) -> Currency {
        match self {
            Self::Nse | Self::Bse | Self::Nfo | Self::Mcx | Self::Cds => Currency::Inr,
        }
    }
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nse => write!(f, "NSE"),
            Self::Bse => write!(f, "BSE"),
            Self::Nfo => write!(f, "NFO"),
            Self::Mcx => write!(f, "MCX"),
            Self::Cds => write!(f, "CDS"),
        }
    }
}

impl FromStr for Exchange {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NSE" => Ok(Self::Nse),
            "BSE" => Ok(Self::Bse),
            "NFO" => Ok(Self::Nfo),
            "MCX" => Ok(Self::Mcx),
            "CDS" => Ok(Self::Cds),
            other => Err(CoreError::InvalidExchange(other.to_string())),
        }
    }
}

fn normalize_instrument_symbol(symbol: &str) -> Result<String, CoreError> {
    if !symbol.is_ascii() || symbol.chars().any(|c| c.is_control() || c == ':') {
        return Err(CoreError::Validation(format!(
            "InstrumentId symbol must be printable ASCII without ':': '{symbol}'"
        )));
    }
    let trimmed = symbol.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(
            "InstrumentId symbol cannot be empty".to_string(),
        ));
    }
    Ok(trimmed.to_ascii_uppercase())
}

#[derive(Deserialize)]
struct InstrumentIdRaw {
    exchange: Exchange,
    symbol: String,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    isin: Option<String>,
}

/// Canonical instrument identifier across all feeds and brokers.
///
/// Ordered by `(exchange, symbol, token, isin)` so it can key a `BTreeMap`
/// and sort a dataset deterministically.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "InstrumentIdRaw")]
pub struct InstrumentId {
    /// Exchange venue
    exchange: Exchange,
    /// Trading symbol (e.g., "RELIANCE", "TCS", "INFY") — always trimmed ASCII uppercase
    symbol: String,
    /// Broker-assigned instrument token (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    /// International Securities Identification Number (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    isin: Option<String>,
}

impl TryFrom<InstrumentIdRaw> for InstrumentId {
    type Error = CoreError;

    fn try_from(raw: InstrumentIdRaw) -> Result<Self, Self::Error> {
        let mut id = Self::try_new(raw.exchange, &raw.symbol)?;
        if let Some(token) = raw.token {
            id = id.with_token(token);
        }
        if let Some(isin) = raw.isin {
            id = id.with_isin(isin);
        }
        Ok(id)
    }
}

impl InstrumentId {
    /// Create a validated, normalized instrument ID with exchange and symbol.
    ///
    /// Trims surrounding whitespace and normalizes ASCII letters to uppercase.
    /// Rejects empty, non-ASCII, control characters, or symbols containing `:`.
    pub fn try_new(exchange: Exchange, symbol: impl AsRef<str>) -> Result<Self, CoreError> {
        let symbol = normalize_instrument_symbol(symbol.as_ref())?;
        Ok(Self {
            exchange,
            symbol,
            token: None,
            isin: None,
        })
    }

    /// Exchange venue.
    pub const fn exchange(&self) -> Exchange {
        self.exchange
    }

    /// Canonical normalized trading symbol (trimmed ASCII uppercase).
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Broker-assigned instrument token, if known.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// International Securities Identification Number, if known.
    pub fn isin(&self) -> Option<&str> {
        self.isin.as_deref()
    }

    /// Attach a broker token to the instrument ID.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Attach an ISIN to the instrument ID.
    pub fn with_isin(mut self, isin: impl Into<String>) -> Self {
        self.isin = Some(isin.into());
        self
    }
}

impl fmt::Display for InstrumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.exchange, self.symbol)
    }
}

/** Exact decimal price (never floating-point).

# Construction invariants

`Price` must never be negative, no matter how it is constructed:

- [`Price::try_new`] is the validated primary constructor and unconditionally
  rejects negative decimals in both debug and release builds.
- [`Price::from_u64`] is an infallible constructor for integer prices.
- The `serde::Deserialize` implementation is a validating wire visitor, so a
  negative price in a JSON payload (LLM output, REST request, replay vector)
  is rejected at the boundary instead of entering the domain.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema, Default)]
#[serde(transparent)]
pub struct Price(Decimal);

impl Price {
    /// Zero price.
    pub const ZERO: Price = Price(Decimal::ZERO);

    /// Crate-internal unchecked constructor for call sites within `tantra-core`
    /// that mathematically guarantee non-negativity.
    #[allow(dead_code)]
    pub(crate) const fn new(d: Decimal) -> Self {
        debug_assert!(
            !d.is_sign_negative() || d.is_zero(),
            "Price cannot be negative"
        );
        Self(d)
    }

    /// Infallible constructor from integer rupees/units.
    pub const fn from_u64(n: u64) -> Self {
        Self(Decimal::from_parts(
            (n & 0xFFFF_FFFF) as u32,
            (n >> 32) as u32,
            0,
            false,
            0,
        ))
    }

    /// Create a validated, non-negative Price from an exact Decimal.
    ///
    /// Normalizes `-0.00` to `Decimal::ZERO`.
    ///
    /// # Errors
    /// Returns [`CoreError::NegativePrice`] when `d` is strictly negative.
    pub fn try_new(d: Decimal) -> Result<Self, CoreError> {
        if d < Decimal::ZERO {
            return Err(CoreError::NegativePrice(d.to_string()));
        }
        let normalized = if d.is_zero() && d.is_sign_negative() {
            Decimal::ZERO
        } else {
            d
        };
        Ok(Self(normalized))
    }

    /// Parse price in Rupees (e.g. "2450.75").
    pub fn from_rupees(s: &str) -> Result<Self, CoreError> {
        let d = Decimal::from_str(s).map_err(|_| CoreError::InvalidPrice(s.to_string()))?;
        Price::try_new(d).map_err(|_| CoreError::NegativePrice(s.to_string()))
    }

    /// Check if price is positive.
    pub fn is_positive(&self) -> bool {
        self.0 > Decimal::ZERO
    }

    /// Check if price conforms to a given tick size (e.g., 0.05 paise).
    pub fn is_tick_aligned(&self, tick_size: Decimal) -> bool {
        if tick_size <= Decimal::ZERO {
            return false;
        }
        (self.0 % tick_size) == Decimal::ZERO
    }

    /// Access the underlying exact decimal value.
    pub const fn value(&self) -> Decimal {
        self.0
    }

    /// Access the underlying exact decimal value (alias for [`Self::value`]).
    pub const fn get(&self) -> Decimal {
        self.0
    }

    /// Access the underlying exact decimal value (alias for [`Self::value`]).
    pub const fn as_decimal(&self) -> Decimal {
        self.0
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Serialize a [`Price`] as its exact decimal string form, matching the
/// wire representation of `Decimal` under `serde-with-str` (golden vectors
/// carry prices like `"2450.75"`).
impl serde::Serialize for Price {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.collect_str(&self.0)
    }
}

/// Deserializing a [`Price`] validates the non-negative invariant at the
/// serde boundary: JSON wire payloads (LLM decisions, REST requests, replay
/// vectors) cannot smuggle a negative price into the domain.
impl<'de> serde::Deserialize<'de> for Price {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PriceVisitor;

        impl<'de> serde::de::Visitor<'de> for PriceVisitor {
            type Value = Price;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a non-negative decimal price string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let d = Decimal::from_str(v).map_err(|_| {
                    serde::de::Error::custom(CoreError::InvalidPrice(v.to_string()))
                })?;
                Price::try_new(d).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(PriceVisitor)
    }
}

/** Exact decimal quantity (supports whole shares and lots).

# Construction invariants

`Quantity` must never be negative (zero quantities for flat/passthrough
positions are legal):

- [`Quantity::try_new`] is the validated primary constructor in both debug
  and release builds.
- [`Quantity::from_u64`] is an infallible constructor for integer quantities.
- Deserialization rejects negative wire payloads in both debug and release.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema, Default)]
#[serde(transparent)]
pub struct Quantity(Decimal);

impl Quantity {
    /// Zero quantity.
    pub const ZERO: Quantity = Quantity(Decimal::ZERO);

    /// Crate-internal unchecked constructor for call sites within `tantra-core`
    /// that mathematically guarantee non-negativity.
    #[allow(dead_code)]
    pub(crate) const fn new(d: Decimal) -> Self {
        debug_assert!(
            !d.is_sign_negative() || d.is_zero(),
            "Quantity cannot be negative"
        );
        Self(d)
    }

    /// Create quantity from integer units.
    pub const fn from_u64(n: u64) -> Self {
        Self(Decimal::from_parts(
            (n & 0xFFFF_FFFF) as u32,
            (n >> 32) as u32,
            0,
            false,
            0,
        ))
    }

    /// Create a validated, non-negative Quantity from an exact Decimal.
    ///
    /// Normalizes `-0.00` to `Decimal::ZERO`.
    ///
    /// # Errors
    /// Returns [`CoreError::NegativeQuantity`] when `d` is strictly negative.
    pub fn try_new(d: Decimal) -> Result<Self, CoreError> {
        if d < Decimal::ZERO {
            return Err(CoreError::NegativeQuantity(d.to_string()));
        }
        let normalized = if d.is_zero() && d.is_sign_negative() {
            Decimal::ZERO
        } else {
            d
        };
        Ok(Self(normalized))
    }

    /// Returns true if quantity is strictly greater than zero.
    pub fn is_positive(&self) -> bool {
        self.0 > Decimal::ZERO
    }

    /// Access the underlying exact decimal value.
    pub const fn value(&self) -> Decimal {
        self.0
    }

    /// Access the underlying exact decimal value (alias for [`Self::value`]).
    pub const fn get(&self) -> Decimal {
        self.0
    }

    /// Access the underlying exact decimal value (alias for [`Self::value`]).
    pub const fn as_decimal(&self) -> Decimal {
        self.0
    }

    /// Check if quantity is a multiple of a lot size.
    pub fn is_lot_multiple(&self, lot_size: u64) -> bool {
        if lot_size == 0 {
            return false;
        }
        let lot_dec = Decimal::from(lot_size);
        (self.0 % lot_dec) == Decimal::ZERO
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Serialize a [`Quantity`] as its exact decimal string form, matching the
/// wire representation of `Decimal` under `serde-with-str`.
impl serde::Serialize for Quantity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.collect_str(&self.0)
    }
}

/// Deserializing a [`Quantity`] rejects negative wire payloads at the serde
/// boundary in both debug and release builds.
impl<'de> serde::Deserialize<'de> for Quantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct QuantityVisitor;

        impl<'de> serde::de::Visitor<'de> for QuantityVisitor {
            type Value = Quantity;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a non-negative decimal quantity string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let d = Decimal::from_str(v).map_err(|_| {
                    serde::de::Error::custom(CoreError::InvalidQuantity(v.to_string()))
                })?;
                Quantity::try_new(d).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(QuantityVisitor)
    }
}

/// The display names of a currency's minor unit (`paisa` / `paise`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct MinorUnit {
    /// Singular name of the minor unit (`paisa`, `cent`, `penny`).
    pub singular: &'static str,
    /// Plural name of the minor unit (`paise`, `cents`, `pence`).
    pub plural: &'static str,
}

/// Currency enum.
///
/// The minor-unit table lives here, in the kernel, so every consumer (Rust
/// crates, the Python SDK through `tantra._tantra.currency_minor_units`) reads one
/// table instead of keeping its own copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
pub enum Currency {
    /// Indian Rupee
    #[default]
    Inr,
    /// United States Dollar
    Usd,
    /// Euro
    Eur,
    /// British Pound
    Gbp,
}

impl Currency {
    /// Every supported currency, in a stable order.
    pub const ALL: &'static [Currency] =
        &[Currency::Inr, Currency::Usd, Currency::Eur, Currency::Gbp];

    /// The ISO 4217 code (`INR`, `USD`, `EUR`, `GBP`).
    pub const fn code(self) -> &'static str {
        match self {
            Self::Inr => "INR",
            Self::Usd => "USD",
            Self::Eur => "EUR",
            Self::Gbp => "GBP",
        }
    }

    /// The number of decimal places in the major unit: one major unit is
    /// `10**minor_exponent` minor units.
    pub const fn minor_exponent(self) -> u8 {
        match self {
            Self::Inr | Self::Usd | Self::Eur | Self::Gbp => 2,
        }
    }

    /// The display names of this currency's minor unit.
    pub const fn minor_unit(self) -> MinorUnit {
        match self {
            Self::Inr => MinorUnit {
                singular: "paisa",
                plural: "paise",
            },
            Self::Usd | Self::Eur => MinorUnit {
                singular: "cent",
                plural: "cents",
            },
            Self::Gbp => MinorUnit {
                singular: "penny",
                plural: "pence",
            },
        }
    }

    /// Parses an ISO 4217 code, case-insensitively.
    pub fn from_code(code: &str) -> Result<Self, CoreError> {
        match code.to_uppercase().as_str() {
            "INR" => Ok(Self::Inr),
            "USD" => Ok(Self::Usd),
            "EUR" => Ok(Self::Eur),
            "GBP" => Ok(Self::Gbp),
            other => Err(CoreError::InvalidCurrency(other.to_string())),
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl FromStr for Currency {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_code(s)
    }
}

/// Explicit monetary amount with currency.
///
/// Fields are private so a currency can never be silently swapped; use
/// [`Money::amount`] / [`Money::currency`] to read them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Money {
    /// Exact amount
    amount: Decimal,
    /// Currency
    currency: Currency,
}

impl Money {
    /// Create an amount in a given currency.
    pub const fn new(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Zero amount in `currency`, for accumulators.
    pub const fn zero(currency: Currency) -> Self {
        Self::new(Decimal::ZERO, currency)
    }

    /// Create amount in INR.
    pub fn inr(amount: Decimal) -> Self {
        Self::new(amount, Currency::Inr)
    }

    /// Zero amount in INR — the default for fields with no cost data yet
    /// (e.g. [`crate::order::FillReport::fee`] before a cost calculation runs).
    pub fn zero_inr() -> Self {
        Self::inr(Decimal::ZERO)
    }

    /// The exact amount, in units of [`Money::currency`].
    pub const fn amount(&self) -> Decimal {
        self.amount
    }

    /// The currency this amount is denominated in.
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Calculate notional value from price and quantity.
    ///
    /// The caller passes the instrument's currency; nothing defaults to INR.
    pub fn notional(price: Price, qty: Quantity, currency: Currency) -> Self {
        Self::new(price.value() * qty.value(), currency)
    }

    /// Add two amounts of the same currency.
    ///
    /// # Errors
    /// [`CoreError::CurrencyMismatch`] if currencies differ; [`CoreError::Validation`] on overflow.
    pub fn checked_add(self, other: Money) -> Result<Money, CoreError> {
        self.require_same_currency(other)?;
        let amount = self
            .amount
            .checked_add(other.amount)
            .ok_or_else(|| CoreError::Validation("money overflow".into()))?;
        Ok(Money::new(amount, self.currency))
    }

    /// Subtract two amounts of the same currency.
    ///
    /// # Errors
    /// [`CoreError::CurrencyMismatch`] if currencies differ; [`CoreError::Validation`] on overflow.
    pub fn checked_sub(self, other: Money) -> Result<Money, CoreError> {
        self.require_same_currency(other)?;
        let amount = self
            .amount
            .checked_sub(other.amount)
            .ok_or_else(|| CoreError::Validation("money overflow".into()))?;
        Ok(Money::new(amount, self.currency))
    }

    /// Quantize to the currency's minor unit (paise/cents/pence) using `strategy`.
    pub fn quantize(self, strategy: rust_decimal::RoundingStrategy) -> Money {
        let dp = u32::from(self.currency.minor_exponent());
        Money::new(
            self.amount.round_dp_with_strategy(dp, strategy),
            self.currency,
        )
    }

    fn require_same_currency(self, other: Money) -> Result<(), CoreError> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(CoreError::CurrencyMismatch {
                left: self.currency,
                right: other.currency,
            })
        }
    }
}

#[derive(Deserialize)]
struct PriceBandRaw {
    lower: Price,
    upper: Price,
    reference_price: Price,
    #[serde(default)]
    band_pct: Option<Decimal>,
    #[serde(default)]
    as_of: Option<DateTime<Utc>>,
}

/// Daily scrip-wise or index circuit limit / price band.
///
/// Enforces `lower <= reference_price <= upper` and, when present,
/// `0 < band_pct <= 1` as a fraction (e.g. `0.05` for 5%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "PriceBandRaw")]
pub struct PriceBand {
    /// Lower price limit (lower circuit)
    lower: Price,
    /// Upper price limit (upper circuit)
    upper: Price,
    /// Previous close or baseline reference price
    reference_price: Price,
    /// Band percentage if known (e.g., 0.05 for 5%, 0.10 for 10%, 0.20 for 20%)
    band_pct: Option<Decimal>,
    /// When the exchange published this band (freshness stamp). `None` means
    /// unknown freshness; fail-closed gates treat that as stale
    /// (tantra-risk gap 0006 `StaleReferenceData`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    as_of: Option<DateTime<Utc>>,
}

impl TryFrom<PriceBandRaw> for PriceBand {
    type Error = CoreError;

    fn try_from(raw: PriceBandRaw) -> Result<Self, Self::Error> {
        let band = Self::try_from_parts(raw.lower, raw.upper, raw.reference_price, raw.band_pct)?;
        Ok(match raw.as_of {
            Some(ts) => band.with_as_of(ts),
            None => band,
        })
    }
}

impl PriceBand {
    /// Create a validated price band with explicit bounds and reference price.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] if `lower > upper` or if
    /// `reference_price` falls outside `[lower, upper]`.
    pub fn try_new(lower: Price, upper: Price, reference_price: Price) -> Result<Self, CoreError> {
        Self::try_from_parts(lower, upper, reference_price, None)
    }

    /// Create a validated price band including optional band percentage fraction (`0 < pct <= 1`).
    pub fn try_from_parts(
        lower: Price,
        upper: Price,
        reference_price: Price,
        band_pct: Option<Decimal>,
    ) -> Result<Self, CoreError> {
        if lower > upper {
            return Err(CoreError::Validation(format!(
                "PriceBand lower ({lower}) exceeds upper ({upper})"
            )));
        }
        if reference_price < lower || reference_price > upper {
            return Err(CoreError::Validation(format!(
                "PriceBand reference_price ({reference_price}) must lie within [{lower}, {upper}]"
            )));
        }
        if let Some(pct) = band_pct
            && (pct <= Decimal::ZERO || pct > Decimal::ONE)
        {
            return Err(CoreError::Validation(format!(
                "PriceBand band_pct ({pct}) must be a fraction in (0, 1]"
            )));
        }
        Ok(Self {
            lower,
            upper,
            reference_price,
            band_pct,
            as_of: None,
        })
    }

    /// Stamp the band with the time the exchange published it.
    #[must_use]
    pub const fn with_as_of(mut self, as_of: DateTime<Utc>) -> Self {
        self.as_of = Some(as_of);
        self
    }

    /// When the band was published, if known.
    pub const fn as_of(&self) -> Option<DateTime<Utc>> {
        self.as_of
    }

    /// Attach a validated band percentage fraction (`0 < band_pct <= 1`).
    pub fn try_with_band_pct(mut self, band_pct: Decimal) -> Result<Self, CoreError> {
        if band_pct <= Decimal::ZERO || band_pct > Decimal::ONE {
            return Err(CoreError::Validation(format!(
                "PriceBand band_pct ({band_pct}) must be a fraction in (0, 1]"
            )));
        }
        self.band_pct = Some(band_pct);
        Ok(self)
    }

    /// Lower circuit price limit.
    pub const fn lower(&self) -> Price {
        self.lower
    }

    /// Upper circuit price limit.
    pub const fn upper(&self) -> Price {
        self.upper
    }

    /// Baseline reference price (e.g. previous close).
    pub const fn reference_price(&self) -> Price {
        self.reference_price
    }

    /// Optional band percentage expressed as a fraction in `(0, 1]`.
    pub const fn band_pct(&self) -> Option<Decimal> {
        self.band_pct
    }

    /// Check if a given price falls within the inclusive circuit band `[lower, upper]`.
    pub fn contains(&self, price: Price) -> bool {
        price.0 >= self.lower.0 && price.0 <= self.upper.0
    }
}

impl fmt::Display for PriceBand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.lower, self.upper)
    }
}

/// Supported sovereign countries / market jurisdictions.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    Default,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum CountryCode {
    /// India (IN)
    #[default]
    In,
    /// United States of America (US)
    Us,
    /// United Kingdom (UK / GB)
    Uk,
    /// Japan (JP)
    Jp,
}

impl CountryCode {
    /// Two-letter country code (`"IN"`, `"US"`, `"UK"`, `"JP"`).
    pub const fn code(&self) -> &'static str {
        match self {
            Self::In => "IN",
            Self::Us => "US",
            Self::Uk => "UK",
            Self::Jp => "JP",
        }
    }
}

impl fmt::Display for CountryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl FromStr for CountryCode {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "IN" => Ok(Self::In),
            "US" => Ok(Self::Us),
            "UK" | "GB" => Ok(Self::Uk),
            "JP" => Ok(Self::Jp),
            other => Err(CoreError::InvalidCountry(other.to_string())),
        }
    }
}
