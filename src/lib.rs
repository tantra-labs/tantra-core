//! # व्यापार・核 tantra-core
//!
//! Shared-kernel canonical types for the Tantra Indian equities trading platform.
//!
//! The kernel is deliberately slim: it owns only the ubiquitous language —
//! value objects, aggregate IDs, wire contracts, the closed [`Decision`]
//! grammar, and [`DomainEvent`]s. Bounded contexts own everything else.
//!
//! Provides:
//! - Canonical strongly-typed aggregate IDs ([`ClientOrderId`], [`AlgoId`], [`AccountId`], [`TransactionId`])
//! - Canonical [`Exchange`] and [`InstrumentId`], covering NSE, BSE, NFO, MCX, and CDS
//! - Market-agnostic classification value objects ([`Sector`], [`Industry`], [`IndexId`], [`MarketCapBucket`])
//! - Exact decimal financial math ([`Price`], [`Quantity`], [`Money`])
//! - Canonical market data [`Tick`], [`DepthLevel`], [`Bar`]
//! - Canonical [`OrderRequest`], [`OrderAck`], [`FillReport`]
//! - Safe, closed, strictly-validated AI [`Decision`]
//! - Scrip-wise [`PriceBand`] limits for Indian market microstructure
//! - Converged recursive Screener DSL ([`ScreenerFilterGroup`], [`ScreenerFilterPredicate`], [`PredicateValue`], [`ClassificationFilter`])
//! - Canonical dynamic position sizing grammar ([`SizingMethod`], [`RiskBudget`], [`PositionSizingRequest`], [`PositionSizingResult`])
//! - Statistical overfitting & tail-risk validation gates (`analytics::ValidationBlock` — DSR, PBO, walk-forward, holdout, block-bootstrap Monte Carlo, cost stress)
//! - Tactical DDD first-class aggregate events ([`DomainEvent`])
//! - Zero-dependency W3C distributed trace context carrier ([`TraceContext`])
//! - Abstract exchange trading calendar port ([`TradingCalendar`])
//! - EE+ intraday level map, options positioning & gamma, order blocks, and
//!   strategy signals/positioning ([`LevelMap`], [`OptionChainSnapshot`],
//!   [`GammaExposure`], [`OrderBlock`], [`StrategySignal`], [`PositionIntent`])

pub mod analytics;
pub mod calendar;
pub mod chart_pattern;
pub mod classification;
pub mod decision;
pub mod error;
pub mod events;
pub mod ids;
pub mod instrument;
pub mod levels;
pub mod market;
pub mod options;
pub mod order;
pub mod order_blocks;
pub mod portfolio;
pub mod ports;
pub mod quant;
pub mod screener;
pub mod signal;
pub mod signals;
pub mod sizing;
pub mod trade_plan;
pub mod types;
pub mod wire;

pub use analytics::{
    ValidationBlock,
    cost_stress::{CostStressConfig, CostStressResult, compute_cost_stress},
    distribution::{SplitMix64, norm_inv},
    dsr::{DsrConfig, DsrResult, compute_dsr, expected_max_sharpe},
    holdout::{HoldoutConfig, HoldoutResult, compute_holdout},
    monte_carlo::{MonteCarloConfig, MonteCarloResult, run_block_bootstrap},
    pbo::{PboConfig, PboResult, compute_pbo},
    walk_forward::{WalkForwardConfig, WalkForwardResult, compute_walk_forward},
};
pub use calendar::{SessionWindow, TradingCalendar};
pub use chart_pattern::{
    ChartPattern, MAX_PATTERN_QUALITY, PatternKind, PatternPivot, PatternStage,
};
pub use classification::{IndexId, Industry, MarketCapBucket, Sector};
pub use decision::Decision;
pub use error::CoreError;
pub use events::{DomainEvent, ExecutionEvent, Reason, ReasonGroup, ReasonPolarity, TradingState};
pub use ids::{AccountId, AlgoId, ClientOrderId, TransactionId};
pub use instrument::{
    DatasetId, Instrument, InstrumentKind, InstrumentProfile, OptionType, PositionSide, TradeId,
    VenueOrderId,
};
pub use levels::{Level, LevelId, LevelKind, LevelMap, LevelSide};
pub use market::{Bar, DepthLevel, Tick};
pub use options::{
    GammaExposure, OptionChainSnapshot, StrikeGamma, StrikePositioning, gamma_flip_strike,
};
pub use order::{
    CancelRequest, FeeBreakdown, FillReport, ModifyRequest, OrderAck, OrderEvent, OrderEventKind,
    OrderRequest, OrderSide, OrderStatus, OrderType, RejectReason, TimeInForce,
};
pub use order_blocks::OrderBlock;
pub use portfolio::{
    PortfolioAccount, PortfolioGreeks, PortfolioHolding, PortfolioMode, PortfolioPosition,
    PortfolioSnapshot, PortfolioSource, PortfolioTotals,
};
pub use ports::{
    BarReader, BarRequest, ConnectorState, DatedInstrument, DepthReader, ExecutionGateway,
    InstrumentMaster, MarketDataConnector, MarketEvent, PortError, PortResult, PositionSizer,
    QuoteReader, SubscriptionParams, TickReader, TickRequest,
};
pub use quant::{
    QuantError, QuantSeries, beta, correlation, ewma_volatility, historical_cvar, historical_var,
    log_returns, max_drawdown, realized_volatility, rolling_mean, rolling_stdev, sharpe_ratio,
    simple_returns, tracking_error,
};
pub use screener::{
    ClassificationFilter, FilterOp, LogicalOp, MetricDefinition, MetricKeySpec, MetricPeriod,
    MetricRef, PredicateValue, RelativeStat, RelativeTo, ScreenerFilterGroup, ScreenerFilterItem,
    ScreenerFilterPredicate, ScreenerRow, ScreenerScanRequest, ScreenerScanResponse, SortSpec,
};
pub use signal::{
    Signal, SignalDirection, SignalHorizon, SignalScanRequest, SignalScanResponse, Timeframe,
};
pub use signals::{PositionIntent, SignalKind, StrategySignal};
pub use sizing::{
    PositionSizingRequest, PositionSizingResult, RiskBudget, SizingMethod, calculate_position_size,
};
pub use trade_plan::{
    BreakevenTrigger, EntryRule, MAX_EXTENSION_ATR, MaKind, PlanError, PlanSubject, PlannedTarget,
    PlannerInputs, SizingSpec, StopRule, TargetLadder, TargetLeg, TargetTrigger, TimeStop,
    TradePlan, TradePlanSpec, TradePlanner, TrailRule, TrailSpec,
};
pub use types::{
    CountryCode, Currency, Exchange, InstrumentId, MinorUnit, Money, Price, PriceBand, Quantity,
};
pub use wire::TraceContext;

/// Version of the JSON wire contract, owned here because this crate owns the
/// wire types.
///
/// The generated JSON Schema, the OpenAPI document, the MCP tool schemas and
/// the Python stubs all carry it; a change to any wire type bumps it.
///
/// v6: added instrument master data, portfolio snapshots, signals, the
/// screener, cross-subsystem domain events, and the `ports` module —
/// the real home for symbols that used to live in the now-empty
/// `tantra-ledger-{entities,messages,ports}` marker crates.
///
/// v7: added `FillReport::fee`/`fee_breakdown` (`FeeBreakdown`) — the
/// contract for per-fill transaction costs; the statutory calculation
/// itself lives in `tantra-market-finance::india::costs`.
///
/// v8: added dynamic position sizing contracts (`SizingMethod`, `RiskBudget`,
/// `PositionSizingRequest`, `PositionSizingResult`, `PositionSizer`), enriched
/// `Signal` with execution/stop levels, first-class tactical `DomainEvent`
/// aggregate envelope, and W3C `TraceContext` distributed observability carrier.
///
/// v9: added the EE+ intraday level map (`Level`, `LevelMap`, `LevelKind`,
/// `LevelSide`, `LevelId`), options positioning & gamma (`OptionChainSnapshot`,
/// `StrikePositioning`, `StrikeGamma`, `GammaExposure` with the gamma flip),
/// order blocks (`OrderBlock`), and EE+ strategy signals / position intents
/// (`StrategySignal`, `SignalKind`, `PositionIntent`). Stop/trail rules are
/// deliberately excluded (delegated to the `TradePlan` slice, plan 0005).
///
/// v10: added the EE+ trade-planner wire contracts: the trade-plan/spec
/// grammar (`EntryRule`, `StopRule`, `TrailRule`, `MaKind`,
/// `BreakevenTrigger`, `TimeStop`, `TrailSpec`, `TargetTrigger`, `TargetLeg`,
/// `TargetLadder`, `SizingSpec`, `TradePlanSpec`, `PlanSubject`,
/// `PlannerInputs`, `PlannedTarget`, `TradePlan`, `PlanError`, `TradePlanner`),
/// chart patterns (`ChartPattern`, `PatternKind`, `PatternStage`,
/// `PatternPivot`), and the `RiskBudget` weight/ADV caps plus
/// `PositionSizingRequest::adv20` liquidity extension.
///
/// Migration note (Tier 0, `honba-core` → `tantra-core`, still wire-compatible
/// with v8): added the canonical aggregate IDs module (`ids`), `Exchange::{Nfo,
/// Cds}` variants, hardened `Price`/`Quantity` deserialization to reject
/// negative values at the serde boundary, and (Plan 0004) added slim
/// classification VOs (`Sector`, `Industry`, `IndexId`, `MarketCapBucket`)
/// plus the converged recursive Screener DSL with deserialization shims so
/// all v8 flat screener payloads continue to deserialize unchanged.
pub const SCHEMA_VERSION: u32 = 10;

/// Semantic version of the REST endpoint surface.
pub const API_VERSION: &str = "v1";

// Re-export foundational external crates for version consistency
pub use chrono;
pub use rust_decimal;
pub use uuid;
