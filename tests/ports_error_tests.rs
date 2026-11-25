//! Contract tests for typed [`PortError`], [`PortErrorKind`],
//! [`ExecutionGateway::cancel`], and [`PortfolioAccount::account_id`]
//! (`tantra-core/docs/gaps/0005-typed-port-errors.md`).

use chrono::{TimeZone, Utc};
use schemars::schema_for;
use serde_json::{Value, json};
use tantra_core::{
    AccountId, Exchange, ExecutionGateway, InstrumentId, ModifyRequest, Money, OrderAck,
    OrderRequest, OrderSide, OrderStatus, OrderType, PortError, PortErrorKind, PortResult,
    PortfolioAccount, PortfolioMode, PortfolioPosition, PortfolioSnapshot, PortfolioSource,
    PortfolioTotals, PositionSide, Price, Quantity, VenueOrderId,
};

#[test]
fn port_error_unavailable_and_classification() {
    let err = PortError::unavailable("live-quotes", "tantra-market/docs/gaps/0003");
    assert_eq!(err.kind(), PortErrorKind::Unavailable);
    assert!(!err.is_retryable());
    assert!(err.to_string().contains("live-quotes"));
    assert!(err.to_string().contains("tantra-market/docs/gaps/0003"));

    let retryable = [
        PortError::ConnectionFailed("reset".to_string()),
        PortError::Timeout,
    ];
    for e in &retryable {
        assert_eq!(e.kind(), PortErrorKind::Retryable);
        assert!(e.is_retryable());
    }

    let permanent = [
        PortError::NotFound("INFY".to_string()),
        PortError::Unsupported("margin_pledge".to_string()),
        PortError::CalculationFailed("negative variance".to_string()),
    ];
    for e in &permanent {
        assert_eq!(e.kind(), PortErrorKind::Permanent);
        assert!(!e.is_retryable());
    }
}

#[test]
fn calculation_failed_display_does_not_say_sizing() {
    let err = PortError::CalculationFailed("dividend overflow".to_string());
    let rendered = err.to_string();
    assert!(
        rendered.starts_with("calculation failed:"),
        "unexpected display: {rendered}"
    );
}

#[test]
fn port_error_serializes_and_matches_golden_schema() {
    let err = PortError::unavailable("options-chain", "tantra-runtime/docs/gaps/0027");
    let wire = serde_json::to_value(&err).expect("PortError must serialize");
    assert_eq!(
        wire,
        json!({
            "unavailable": {
                "capability": "options-chain",
                "gap_ref": "tantra-runtime/docs/gaps/0027"
            }
        })
    );

    let generated = serde_json::to_value(schema_for!(PortError)).expect("schema serialization");
    let golden_raw = std::fs::read_to_string("schema/golden/port_error_schema_golden.json")
        .expect("schema/golden/port_error_schema_golden.json must exist");
    let golden: Value = serde_json::from_str(&golden_raw).expect("valid golden JSON");
    assert_eq!(generated, golden);
}

struct StubGateway {
    cancelled: Vec<VenueOrderId>,
}

impl ExecutionGateway for StubGateway {
    fn submit(&mut self, _req: &OrderRequest) -> PortResult<OrderAck> {
        Err(PortError::unavailable(
            "submit",
            "tantra-exec/docs/gaps/0004",
        ))
    }

    fn cancel(&mut self, order_id: &VenueOrderId) -> PortResult<()> {
        self.cancelled.push(order_id.clone());
        Ok(())
    }

    fn modify(&mut self, _req: &ModifyRequest) -> PortResult<OrderAck> {
        Err(PortError::Timeout)
    }
}

#[test]
fn execution_gateway_cancel_accepts_typed_venue_order_id() {
    let mut gw = StubGateway { cancelled: vec![] };
    let id = VenueOrderId::new("KITE-12345");
    gw.cancel(&id).expect("cancel succeeds");
    assert_eq!(gw.cancelled, vec![id]);

    let req = OrderRequest::new(
        InstrumentId::try_new(Exchange::Nse, "RELIANCE").unwrap(),
        OrderSide::Buy,
        OrderType::Market,
        Quantity::from_u64(1),
        "ALGO-1",
    );
    let submit_err = gw.submit(&req).unwrap_err();
    assert_eq!(submit_err.kind(), PortErrorKind::Unavailable);

    let ack = OrderAck::new(
        "KITE-1",
        req.client_order_id,
        OrderStatus::New,
        Utc.with_ymd_and_hms(2026, 11, 24, 4, 0, 0).unwrap(),
    );
    assert_eq!(ack.order_id, "KITE-1");
}

#[test]
fn portfolio_account_uses_account_id_with_transparent_wire_repr() {
    let snapshot = PortfolioSnapshot {
        account: PortfolioAccount {
            account_id: AccountId::new("ACC-42"),
            mode: PortfolioMode::Paper,
            source: PortfolioSource::Simulated,
        },
        positions: vec![PortfolioPosition {
            instrument: InstrumentId::try_new(Exchange::Nse, "TCS").unwrap(),
            side: PositionSide::Long,
            quantity: Quantity::from_u64(5),
            avg_price: Price::from_rupees("3500").unwrap(),
            unrealized_pnl: Money::inr(rust_decimal::Decimal::ZERO),
            realized_pnl: Money::inr(rust_decimal::Decimal::ZERO),
        }],
        holdings: vec![],
        totals: PortfolioTotals {
            cash: Money::inr(rust_decimal::Decimal::from(50_000)),
            margin_used: Money::inr(rust_decimal::Decimal::from(10_000)),
            margin_available: Money::inr(rust_decimal::Decimal::from(40_000)),
            total_pnl: Money::inr(rust_decimal::Decimal::ZERO),
            net_worth: Money::inr(rust_decimal::Decimal::from(50_000)),
        },
        greeks: None,
        timestamp: Utc.with_ymd_and_hms(2026, 11, 24, 4, 0, 0).unwrap(),
    };

    let v = serde_json::to_value(&snapshot).expect("serialize");
    assert_eq!(v["account"]["account_id"], Value::String("ACC-42".into()));
    let roundtrip: PortfolioSnapshot = serde_json::from_value(v).expect("deserialize");
    assert_eq!(roundtrip.account.account_id.as_str(), "ACC-42");
}

#[test]
#[ignore = "regenerate schema/golden/port_error_schema_golden.json"]
fn write_port_error_schema_golden() {
    let schema = serde_json::to_string_pretty(&schema_for!(PortError)).expect("serialize schema");
    std::fs::write(
        "schema/golden/port_error_schema_golden.json",
        format!("{schema}\n"),
    )
    .expect("write golden schema");
}
