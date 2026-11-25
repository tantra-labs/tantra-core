//! Hexagonal port traits (interfaces only, no implementations).
//!
//! `tantra-core` stays zero-I/O (`AGENTS.md` §2.5): these traits are abstract
//! contracts, not implementations. Real market-data connectors, execution
//! gateways, and position sizers live in downstream crates or default pure kernels,
//! injected as `Arc<dyn Trait>` per `AGENTS.md` §2.1. Every trait here is
//! object-safe for exactly that reason.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::instrument::{Instrument, VenueOrderId};
use crate::market::{Bar, DepthLevel, Tick};
use crate::order::{ModifyRequest, OrderAck, OrderRequest};
use crate::signal::Timeframe;
use crate::sizing::{PositionSizingRequest, PositionSizingResult};
use crate::types::InstrumentId;

pub use crate::calendar::{SessionWindow, TradingCalendar};

/// Coarse operational classification of a [`PortError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PortErrorKind {
    /// Transient failure; safe to retry with backoff.
    Retryable,
    /// Permanent failure for this request; retrying the same request will fail again.
    Permanent,
    /// Capability is unimplemented, unconfigured, or gated upstream (carries a gap reference).
    Unavailable,
}

/// Errors a port implementation can return.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PortError {
    /// The underlying connection could not be established or was lost.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    /// The operation did not complete in time.
    #[error("operation timed out")]
    Timeout,
    /// The requested resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The operation is not supported by this implementation.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// Calculation or validation failure in port adapter.
    #[error("calculation failed: {0}")]
    CalculationFailed(String),
    /// Capability is unavailable or not yet wired (paired with a tracked gap document).
    #[error("capability '{capability}' unavailable (see {gap_ref})")]
    Unavailable {
        /// Identifier of the unavailable capability.
        capability: &'static str,
        /// Reference to the gap or plan document tracking closure.
        gap_ref: &'static str,
    },
}

impl PortError {
    /// Construct an explicit unavailability error referencing the tracking gap document.
    #[must_use]
    pub const fn unavailable(capability: &'static str, gap_ref: &'static str) -> Self {
        Self::Unavailable {
            capability,
            gap_ref,
        }
    }

    /// Operational error class (`Retryable`, `Permanent`, or `Unavailable`).
    #[must_use]
    pub const fn kind(&self) -> PortErrorKind {
        match self {
            Self::ConnectionFailed(_) | Self::Timeout => PortErrorKind::Retryable,
            Self::NotFound(_) | Self::Unsupported(_) | Self::CalculationFailed(_) => {
                PortErrorKind::Permanent
            }
            Self::Unavailable { .. } => PortErrorKind::Unavailable,
        }
    }

    /// Whether a caller can safely retry the failed port operation.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self.kind(), PortErrorKind::Retryable)
    }
}

/// Result type for port operations.
pub type PortResult<T> = Result<T, PortError>;

/// Connection status of a market-data connector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ConnectorState {
    /// Not connected.
    Disconnected,
    /// Connection in progress.
    Connecting,
    /// Connected and healthy.
    Connected,
    /// Connected but degraded.
    Degraded {
        /// Why it's degraded.
        reason: String,
    },
}

/// A request for historical bars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BarRequest {
    /// The instrument to fetch bars for.
    pub instrument: InstrumentId,
    /// Bar interval.
    pub timeframe: Timeframe,
    /// Start of the window, inclusive.
    pub from: DateTime<Utc>,
    /// End of the window, inclusive.
    pub to: DateTime<Utc>,
}

/// A request for historical ticks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TickRequest {
    /// The instrument to fetch ticks for.
    pub instrument: InstrumentId,
    /// Start of the window, inclusive.
    pub from: DateTime<Utc>,
    /// End of the window, inclusive.
    pub to: DateTime<Utc>,
}

/// Parameters for a live market-data subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SubscriptionParams {
    /// Instruments to subscribe to.
    pub instruments: Vec<InstrumentId>,
    /// Whether full market depth (L2) is wanted, not just top-of-book.
    pub depth: bool,
}

/// An instrument's reference data as of a specific date — corporate
/// actions and expiry rollovers change an instrument's metadata over time,
/// so lookups are always dated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DatedInstrument {
    /// The instrument record.
    pub instrument: Instrument,
    /// The date this record is valid as of.
    pub as_of: DateTime<Utc>,
}

/// An event pushed by a [`MarketDataConnector`] to its subscribers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MarketEvent {
    /// A tick (LTP, quote, or depth).
    Tick(Tick),
    /// A completed bar.
    Bar(Bar),
    /// The connector's own connection state changed.
    ConnectorStateChanged(ConnectorState),
}

/// Reads historical bars.
pub trait BarReader {
    /// Fetch bars matching the request.
    fn read_bars(&self, req: &BarRequest) -> PortResult<Vec<Bar>>;
}

/// Reads historical ticks.
pub trait TickReader {
    /// Fetch ticks matching the request.
    fn read_ticks(&self, req: &TickRequest) -> PortResult<Vec<Tick>>;
}

/// Reads the latest quote for an instrument.
pub trait QuoteReader {
    /// Fetch the most recent quote tick.
    fn latest_quote(&self, instrument: &InstrumentId) -> PortResult<Tick>;
}

/// Reads current market depth for an instrument.
pub trait DepthReader {
    /// Fetch the current depth book.
    fn depth(&self, instrument: &InstrumentId) -> PortResult<Vec<DepthLevel>>;
}

/// A live market-data connection (broker WebSocket, replay feed, etc).
pub trait MarketDataConnector {
    /// Current connection state.
    fn state(&self) -> ConnectorState;
    /// Subscribe to the given instruments.
    fn subscribe(&mut self, params: &SubscriptionParams) -> PortResult<()>;
    /// Drain whatever market events have arrived since the last poll.
    fn poll(&mut self) -> PortResult<Vec<MarketEvent>>;
}

/// Instrument master-data lookup.
pub trait InstrumentMaster {
    /// Look up an instrument's dated reference data.
    fn lookup(&self, id: &InstrumentId) -> PortResult<DatedInstrument>;
    /// Search instruments by symbol prefix.
    fn search(&self, symbol_prefix: &str) -> PortResult<Vec<InstrumentId>>;
}

/// Order execution against a broker or simulator.
pub trait ExecutionGateway {
    /// Submit a new order.
    fn submit(&mut self, req: &OrderRequest) -> PortResult<OrderAck>;
    /// Cancel an existing order by broker/venue order ID.
    fn cancel(&mut self, order_id: &VenueOrderId) -> PortResult<()>;
    /// Modify an existing order.
    fn modify(&mut self, req: &ModifyRequest) -> PortResult<OrderAck>;
}

/// Hexagonal port for dynamic position sizing.
///
/// Decouples signal generation from sizing algorithms. Object-safe for
/// `Arc<dyn PositionSizer>` dependency injection with zero I/O and zero floats.
/// Concrete sizing adapters (backed by [`crate::sizing::calculate_position_size`]
/// or richer, credit-aware kernels) live in downstream bounded contexts, not in
/// the Shared Kernel — the kernel owns the grammar (`SizingMethod`,
/// [`crate::sizing::RiskBudget`]) and pure math, never runnable plugs.
pub trait PositionSizer: Send + Sync {
    /// Calculate position sizing from request parameters.
    fn calculate_size(
        &self,
        request: &PositionSizingRequest,
    ) -> Result<PositionSizingResult, PortError>;
}
