use chrono::Utc;
use rust_decimal::Decimal;
use tantra_core::*;
use uuid::Uuid;

fn breakdown_with_gst(gst: Money) -> FeeBreakdown {
    FeeBreakdown {
        brokerage: Money::inr(Decimal::ONE),
        stt: Money::inr(Decimal::ONE),
        exchange_charges: Money::inr(Decimal::ONE),
        gst,
        stamp_duty: Money::inr(Decimal::ONE),
        sebi_charges: Money::inr(Decimal::ONE),
    }
}

fn usd_breakdown() -> FeeBreakdown {
    breakdown_with_gst(Money::new(Decimal::ONE, Currency::Usd))
}

#[test]
fn mixed_currency_total_errors() {
    assert_eq!(
        usd_breakdown().total(),
        Err(CoreError::CurrencyMismatch {
            left: Currency::Inr,
            right: Currency::Usd
        })
    );
}

#[test]
fn all_inr_total_is_exact_sum() {
    let b = breakdown_with_gst(Money::inr(Decimal::new(387, 2)));
    assert_eq!(b.total(), Ok(Money::inr(Decimal::new(887, 2))));
}

#[test]
fn with_fee_breakdown_propagates_error() {
    let fill = FillReport::new(
        "F-1",
        "O-1",
        Uuid::new_v4(),
        InstrumentId::try_new(Exchange::Nse, "RELIANCE").unwrap(),
        OrderSide::Buy,
        Price::from_rupees("100.00").unwrap(),
        Quantity::from_u64(1),
        "s",
        Utc::now(),
        "default",
    );
    assert!(matches!(
        fill.with_fee_breakdown(usd_breakdown()),
        Err(CoreError::CurrencyMismatch { .. })
    ));
}

#[test]
fn every_exchange_settles_in_inr() {
    for e in [
        Exchange::Nse,
        Exchange::Bse,
        Exchange::Nfo,
        Exchange::Mcx,
        Exchange::Cds,
    ] {
        assert_eq!(e.currency(), Currency::Inr);
    }
}

#[test]
fn money_zero_in_currency() {
    let z = Money::zero(Currency::Usd);
    assert_eq!(z.amount(), Decimal::ZERO);
    assert_eq!(z.currency(), Currency::Usd);
}
