//! Market-agnostic classification value objects (Plan 0004 Phase A).
//!
//! These slim, jurisdiction-neutral newtypes and enums define the shared wire
//! vocabulary across `tantra-core` consumers (`tantra-data`, `tantra-runtime`,
//! `tantra-strategies`, `tantra-ui`, `tantra-ffi`):
//! - [`Sector`] and [`Industry`]: validated, whitespace-normalized labels
//! - [`IndexId`]: validated uppercase ASCII index identifier (e.g. `"NIFTY500"`, `"SENSEX"`, `"SP500"`)
//! - [`MarketCapBucket`]: bare market-cap tier (`large`, `mid`, `small`, `micro`)
//!
//! Jurisdiction-specific rules (such as SEBI/AMFI rank cutoffs, NSE 4-level
//! hierarchy validation, and editorial themes) live in `tantra-data` /
//! `tantra-market-finance`, keeping L0 domain-pure.

use std::fmt;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::CoreError;

/// Maximum Unicode scalar length for a normalized [`Sector`] or [`Industry`] label.
pub const MAX_CLASSIFICATION_LEN: usize = 80;

/// Maximum ASCII byte length for a normalized [`IndexId`].
pub const MAX_INDEX_ID_LEN: usize = 40;

fn normalize_label(kind: &str, raw: &str) -> Result<String, CoreError> {
    if raw.chars().any(|c| c.is_control() && c != '\t') {
        return Err(CoreError::Validation(format!(
            "{kind} cannot contain control characters"
        )));
    }
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Err(CoreError::Validation(format!("{kind} cannot be empty")));
    }
    if normalized.chars().count() > MAX_CLASSIFICATION_LEN {
        return Err(CoreError::Validation(format!(
            "{kind} exceeds maximum length of {MAX_CLASSIFICATION_LEN} characters"
        )));
    }
    Ok(normalized)
}

/// Validated equity sector label (e.g. `"Financial Services"`, `"Information Technology"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[serde(transparent)]
pub struct Sector(String);

impl Sector {
    /// Construct a validated [`Sector`], trimming and collapsing internal whitespace.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] if the input is empty, contains control
    /// characters, or exceeds [`MAX_CLASSIFICATION_LEN`].
    pub fn try_new(raw: impl AsRef<str>) -> Result<Self, CoreError> {
        normalize_label("sector", raw.as_ref()).map(Self)
    }

    /// Borrow the canonical sector string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Sector {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for Sector {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl TryFrom<&str> for Sector {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl TryFrom<String> for Sector {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<Sector> for String {
    fn from(value: Sector) -> Self {
        value.0
    }
}

impl Serialize for Sector {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Sector {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::try_new(raw).map_err(serde::de::Error::custom)
    }
}

/// Validated equity industry label (e.g. `"IT - Software"`, `"Banks"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[serde(transparent)]
pub struct Industry(String);

impl Industry {
    /// Construct a validated [`Industry`], trimming and collapsing internal whitespace.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] if the input is empty, contains control
    /// characters, or exceeds [`MAX_CLASSIFICATION_LEN`].
    pub fn try_new(raw: impl AsRef<str>) -> Result<Self, CoreError> {
        normalize_label("industry", raw.as_ref()).map(Self)
    }

    /// Borrow the canonical industry string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Industry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Industry {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for Industry {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl TryFrom<&str> for Industry {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl TryFrom<String> for Industry {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<Industry> for String {
    fn from(value: Industry) -> Self {
        value.0
    }
}

impl Serialize for Industry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Industry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::try_new(raw).map_err(serde::de::Error::custom)
    }
}

/// Validated uppercase ASCII market index identifier (`"NIFTY50"`, `"NIFTY500"`, `"SENSEX"`, `"SP500"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[serde(transparent)]
pub struct IndexId(String);

impl IndexId {
    /// Construct a validated, canonical uppercase ASCII [`IndexId`].
    ///
    /// Accepts ASCII alphanumeric characters plus `'_'` and `'-'`, trimmed and
    /// normalized to uppercase.
    ///
    /// # Errors
    /// Returns [`CoreError::Validation`] if empty, exceeds [`MAX_INDEX_ID_LEN`],
    /// or contains unsupported characters.
    pub fn try_new(raw: impl AsRef<str>) -> Result<Self, CoreError> {
        let trimmed = raw.as_ref().trim();
        if trimmed.is_empty() {
            return Err(CoreError::Validation("index_id cannot be empty".to_owned()));
        }
        if trimmed.len() > MAX_INDEX_ID_LEN {
            return Err(CoreError::Validation(format!(
                "index_id exceeds maximum length of {MAX_INDEX_ID_LEN} characters"
            )));
        }
        if !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(CoreError::Validation(format!(
                "invalid index_id '{trimmed}': expected ASCII alphanumeric, '_' or '-'"
            )));
        }
        Ok(Self(trimmed.to_ascii_uppercase()))
    }

    /// Borrow the canonical uppercase ASCII index identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IndexId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for IndexId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for IndexId {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl TryFrom<&str> for IndexId {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl TryFrom<String> for IndexId {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<IndexId> for String {
    fn from(value: IndexId) -> Self {
        value.0
    }
}

impl Serialize for IndexId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for IndexId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::try_new(raw).map_err(serde::de::Error::custom)
    }
}

/// Jurisdiction-neutral market capitalization bucket (`large`, `mid`, `small`, `micro`).
///
/// Jurisdiction-specific cutoff logic (such as SEBI/AMFI rank cutoffs) lives
/// outside L0 in `tantra-data` / `tantra-market-finance`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MarketCapBucket {
    /// Large capitalization tier.
    Large,
    /// Mid capitalization tier.
    Mid,
    /// Small capitalization tier.
    Small,
    /// Micro capitalization tier.
    Micro,
}

impl MarketCapBucket {
    /// All cap-bucket variants in canonical descending size order.
    pub const ALL: &'static [Self] = &[Self::Large, Self::Mid, Self::Small, Self::Micro];

    /// Canonical wire string (`"large"`, `"mid"`, `"small"`, `"micro"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Large => "large",
            Self::Mid => "mid",
            Self::Small => "small",
            Self::Micro => "micro",
        }
    }
}

impl fmt::Display for MarketCapBucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MarketCapBucket {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "large" => Ok(Self::Large),
            "mid" => Ok(Self::Mid),
            "small" => Ok(Self::Small),
            "micro" => Ok(Self::Micro),
            other => Err(CoreError::Validation(format!(
                "unknown market cap bucket '{other}': expected large, mid, small, or micro"
            ))),
        }
    }
}
