//! Contract, property, and negative golden vector tests for kernel value-object
//! invariants (`tantra-core/docs/gaps/0003-value-objects-constructible-invalid.md`).

use std::collections::BTreeMap;
use std::str::FromStr;

use proptest::prelude::*;
use rust_decimal::Decimal;
use tantra_core::*;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn px(s: &str) -> Price {
    Price::try_new(dec(s)).unwrap()
}

fn qty(s: &str) -> Quantity {
    Quantity::try_new(dec(s)).unwrap()
}

#[test]
fn price_and_quantity_try_new_enforce_non_negative_and_normalize_neg_zero() {
    assert!(Price::try_new(dec("-0.0001")).is_err());
    assert!(Price::try_new(dec("-100")).is_err());
    assert!(Quantity::try_new(dec("-0.0001")).is_err());
    assert!(Quantity::try_new(dec("-1")).is_err());

    let neg_zero_p = Price::try_new(dec("-0.00")).unwrap();
    assert_eq!(neg_zero_p.value(), Decimal::ZERO);
    assert_eq!(neg_zero_p.get(), Decimal::ZERO);
    assert_eq!(neg_zero_p.as_decimal(), Decimal::ZERO);
    assert!(!neg_zero_p.value().is_sign_negative());

    let neg_zero_q = Quantity::try_new(dec("-0.00")).unwrap();
    assert_eq!(neg_zero_q.value(), Decimal::ZERO);
    assert_eq!(neg_zero_q.get(), Decimal::ZERO);
    assert_eq!(neg_zero_q.as_decimal(), Decimal::ZERO);
    assert!(!neg_zero_q.value().is_sign_negative());

    assert_eq!(Price::from_u64(250).value(), Decimal::from(250));
    assert_eq!(Quantity::from_u64(75).value(), Decimal::from(75));
}

#[test]
fn price_band_try_new_and_serde_reject_invalid_vectors() {
    // Valid boundary cases
    let ok = PriceBand::try_new(px("900.00"), px("1100.00"), px("1000.00"))
        .unwrap()
        .try_with_band_pct(dec("0.10"))
        .unwrap();
    assert_eq!(ok.lower(), px("900.00"));
    assert_eq!(ok.upper(), px("1100.00"));
    assert_eq!(ok.reference_price(), px("1000.00"));
    assert_eq!(ok.band_pct(), Some(dec("0.10")));
    assert!(ok.contains(px("900.00")));
    assert!(ok.contains(px("1100.00")));
    assert!(!ok.contains(px("899.99")));

    // Degenerate zero-width band with ref == lower == upper is valid
    assert!(PriceBand::try_new(px("100"), px("100"), px("100")).is_ok());

    // lower > upper
    assert!(PriceBand::try_new(px("1100"), px("900"), px("1000")).is_err());
    // reference outside [lower, upper]
    assert!(PriceBand::try_new(px("900"), px("1100"), px("899.95")).is_err());
    assert!(PriceBand::try_new(px("900"), px("1100"), px("1100.05")).is_err());

    // invalid band_pct (must be fraction in (0, 1])
    let base = PriceBand::try_new(px("900"), px("1100"), px("1000")).unwrap();
    assert!(base.try_with_band_pct(dec("0")).is_err());
    assert!(base.try_with_band_pct(dec("-0.05")).is_err());
    assert!(base.try_with_band_pct(dec("1.01")).is_err());
    assert!(base.try_with_band_pct(dec("10")).is_err());
    assert!(base.try_with_band_pct(dec("1.0")).is_ok());

    // Negative golden JSON payloads rejected on Deserialize
    let invalid_jsons = [
        serde_json::json!({
            "lower": "1100.00",
            "upper": "900.00",
            "reference_price": "1000.00"
        }),
        serde_json::json!({
            "lower": "900.00",
            "upper": "1100.00",
            "reference_price": "1200.00"
        }),
        serde_json::json!({
            "lower": "900.00",
            "upper": "1100.00",
            "reference_price": "1000.00",
            "band_pct": "10.0"
        }),
        serde_json::json!({
            "lower": "900.00",
            "upper": "1100.00",
            "reference_price": "1000.00",
            "band_pct": "0"
        }),
        serde_json::json!({
            "lower": "-10.00",
            "upper": "1100.00",
            "reference_price": "1000.00"
        }),
    ];
    for payload in invalid_jsons {
        assert!(
            serde_json::from_value::<PriceBand>(payload.clone()).is_err(),
            "expected deserialization error for invalid PriceBand payload: {payload}"
        );
    }
}

#[test]
fn instrument_id_normalizes_and_validates_on_try_new_and_serde() {
    let canonical = InstrumentId::try_new(Exchange::Nse, "RELIANCE").unwrap();
    let lower_deser: InstrumentId = serde_json::from_value(serde_json::json!({
        "exchange": "NSE",
        "symbol": "  reliance  "
    }))
    .unwrap();
    assert_eq!(lower_deser, canonical);
    assert_eq!(lower_deser.symbol(), "RELIANCE");

    let mut map = BTreeMap::new();
    map.insert(canonical.clone(), 1);
    assert_eq!(map.get(&lower_deser), Some(&1));

    // Empty / whitespace / invalid symbols rejected everywhere
    for bad in ["", "   ", "\nTCS", "TCS\0", "NSE:TCS", "₹RELIANCE"] {
        assert!(
            InstrumentId::try_new(Exchange::Nse, bad).is_err(),
            "try_new must reject invalid symbol {bad:?}"
        );
        let json = serde_json::json!({
            "exchange": "NSE",
            "symbol": bad
        });
        assert!(
            serde_json::from_value::<InstrumentId>(json).is_err(),
            "Deserialize must reject invalid symbol {bad:?}"
        );
    }
}

#[test]
fn instrument_try_new_and_serde_reject_non_positive_lot_or_tick_size() {
    let id = InstrumentId::try_new(Exchange::Nse, "INFY").unwrap();
    let ok = Instrument::try_equity(id.clone(), qty("1"), dec("0.05")).unwrap();
    assert_eq!(ok.id(), &id);
    assert_eq!(ok.lot_size(), qty("1"));
    assert_eq!(ok.tick_size(), dec("0.05"));

    // Zero or negative tick_size / zero lot_size must fail
    assert!(Instrument::try_equity(id.clone(), Quantity::ZERO, dec("0.05")).is_err());
    assert!(Instrument::try_equity(id.clone(), qty("1"), Decimal::ZERO).is_err());
    assert!(Instrument::try_equity(id.clone(), qty("1"), dec("-0.05")).is_err());
    assert!(
        Instrument::try_new(
            id.clone(),
            InstrumentKind::Future,
            Quantity::ZERO,
            dec("0.05")
        )
        .is_err()
    );

    let invalid_jsons = [
        serde_json::json!({
            "id": { "exchange": "NSE", "symbol": "INFY" },
            "kind": "equity",
            "lot_size": "0",
            "tick_size": "0.05"
        }),
        serde_json::json!({
            "id": { "exchange": "NSE", "symbol": "INFY" },
            "kind": "equity",
            "lot_size": "1",
            "tick_size": "0"
        }),
        serde_json::json!({
            "id": { "exchange": "NSE", "symbol": "INFY" },
            "kind": "equity",
            "lot_size": "1",
            "tick_size": "-0.05"
        }),
        serde_json::json!({
            "id": { "exchange": "NSE", "symbol": "" },
            "kind": "equity",
            "lot_size": "1",
            "tick_size": "0.05"
        }),
    ];
    for payload in invalid_jsons {
        assert!(
            serde_json::from_value::<Instrument>(payload.clone()).is_err(),
            "expected deserialization error for invalid Instrument payload: {payload}"
        );
    }
}

proptest! {
    #[test]
    fn prop_arbitrary_decimals_never_yield_invalid_price_or_quantity(
        mantissa in any::<i64>(),
        scale in 0u32..8,
    ) {
        let d = Decimal::new(mantissa, scale);
        match Price::try_new(d) {
            Ok(p) => {
                prop_assert!(p.value() >= Decimal::ZERO);
                prop_assert!(!p.value().is_sign_negative());
                let json = serde_json::to_string(&p).unwrap();
                let decoded: Price = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(p, decoded);
            }
            Err(_) => prop_assert!(d < Decimal::ZERO),
        }

        match Quantity::try_new(d) {
            Ok(q) => {
                prop_assert!(q.value() >= Decimal::ZERO);
                prop_assert!(!q.value().is_sign_negative());
                let json = serde_json::to_string(&q).unwrap();
                let decoded: Quantity = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(q, decoded);
            }
            Err(_) => prop_assert!(d < Decimal::ZERO),
        }

        let s = serde_json::Value::String(d.to_string());
        if d < Decimal::ZERO {
            prop_assert!(serde_json::from_value::<Price>(s.clone()).is_err());
            prop_assert!(serde_json::from_value::<Quantity>(s).is_err());
        }
    }

    #[test]
    fn prop_arbitrary_decimals_never_yield_invalid_price_band(
        l in -1000i64..10_000,
        u in -1000i64..10_000,
        r in -1000i64..10_000,
        pct_bps in -500i64..15_000,
    ) {
        let dl = Decimal::new(l, 2);
        let du = Decimal::new(u, 2);
        let dr = Decimal::new(r, 2);
        let dpct = Decimal::new(pct_bps, 4);

        let json = serde_json::json!({
            "lower": dl.to_string(),
            "upper": du.to_string(),
            "reference_price": dr.to_string(),
            "band_pct": dpct.to_string(),
        });

        let should_succeed = dl >= Decimal::ZERO
            && du >= dl
            && dr >= dl
            && dr <= du
            && dpct > Decimal::ZERO
            && dpct <= Decimal::ONE;

        match serde_json::from_value::<PriceBand>(json) {
            Ok(band) => {
                prop_assert!(should_succeed);
                prop_assert!(band.lower() <= band.upper());
                prop_assert!(band.reference_price() >= band.lower());
                prop_assert!(band.reference_price() <= band.upper());
                let pct = band.band_pct().unwrap();
                prop_assert!(pct > Decimal::ZERO && pct <= Decimal::ONE);
            }
            Err(_) => {
                prop_assert!(!should_succeed);
            }
        }
    }
}
