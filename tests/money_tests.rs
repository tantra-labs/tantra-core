//! Money currency-safety tests.

use proptest::prelude::*;
use rust_decimal::{Decimal, RoundingStrategy};
use std::str::FromStr;
use tantra_core::*;

fn currency() -> impl Strategy<Value = Currency> {
    prop_oneof![
        Just(Currency::Inr),
        Just(Currency::Usd),
        Just(Currency::Eur),
        Just(Currency::Gbp),
    ]
}

fn amount() -> impl Strategy<Value = Decimal> {
    (-1_000_000_000_000i64..1_000_000_000_000i64, 0u32..=4).prop_map(|(n, s)| Decimal::new(n, s))
}

proptest! {
    #[test]
    fn prop_add_sub_roundtrip(a in amount(), b in amount(), c in currency()) {
        let (a, b) = (Money::new(a, c), Money::new(b, c));
        prop_assert_eq!(a.checked_add(b).unwrap().checked_sub(b).unwrap(), a);
    }

    #[test]
    fn prop_mismatch_errors(a in amount(), b in amount(), c1 in currency(), c2 in currency()) {
        prop_assume!(c1 != c2);
        let (x, y) = (Money::new(a, c1), Money::new(b, c2));
        let expected = CoreError::CurrencyMismatch { left: c1, right: c2 };
        prop_assert_eq!(x.checked_add(y), Err(expected));
        let expected = CoreError::CurrencyMismatch { left: c1, right: c2 };
        prop_assert_eq!(x.checked_sub(y), Err(expected));
    }
}

#[test]
fn notional_carries_currency() {
    let price = Price::from_rupees("12.50").unwrap();
    let qty = Quantity::from_u64(4);
    let m = Money::notional(price, qty, Currency::Usd);
    assert_eq!(m.currency(), Currency::Usd);
    assert_eq!(m.amount(), price.value() * qty.value());
}

#[test]
fn quantize_rounds_to_minor_unit() {
    let m = Money::new(Decimal::from_str("10.005").unwrap(), Currency::Inr);
    let up = m.quantize(RoundingStrategy::MidpointAwayFromZero);
    assert_eq!(up.amount(), Decimal::from_str("10.01").unwrap());
    let even = m.quantize(RoundingStrategy::MidpointNearestEven);
    assert_eq!(even.amount(), Decimal::from_str("10.00").unwrap());
    assert_eq!(up.quantize(RoundingStrategy::MidpointAwayFromZero), up);
    assert_eq!(even.quantize(RoundingStrategy::MidpointNearestEven), even);
}

#[test]
fn serde_wire_shape_unchanged() {
    let m = Money::inr(Decimal::from_str("1.50").unwrap());
    let v = serde_json::to_value(m).unwrap();
    let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, vec!["amount", "currency"]);
    let back: Money = serde_json::from_value(v).unwrap();
    assert_eq!(back, m);
}
