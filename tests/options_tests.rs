//! Unit tests for `tantra_core::options` (Plan 0003, Phase 2).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use tantra_core::{
    CoreError, Exchange, GammaExposure, InstrumentId, OptionChainSnapshot, Price, StrikeGamma,
    StrikePositioning,
};

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn strike(rupees: i64) -> Price {
    Price::try_new(Decimal::from(rupees)).unwrap()
}

/// Build a positioning with explicit OI/IV/gamma values (OI change and deltas
/// are fixed at representative values, including a negative put-delta and a
/// negative put OI change).
fn make(
    call_oi: Decimal,
    put_oi: Decimal,
    call_iv: Decimal,
    put_iv: Decimal,
    call_gamma: Decimal,
    put_gamma: Decimal,
) -> Result<StrikePositioning, CoreError> {
    StrikePositioning::new(
        strike(2500),
        call_oi,
        put_oi,
        dec("8000"),
        dec("-5000"),
        call_iv,
        put_iv,
        dec("0.52"),
        dec("-0.48"),
        call_gamma,
        put_gamma,
    )
}

#[test]
fn strike_positioning_accepts_valid_values() {
    let s = make(
        dec("120000"),
        dec("140000"),
        dec("0.185"),
        dec("0.178"),
        dec("0.0045"),
        dec("0.0045"),
    )
    .unwrap();
    assert_eq!(s.strike, strike(2500));
    assert_eq!(s.call_oi, dec("120000"));
    assert_eq!(s.put_oi, dec("140000"));
    assert_eq!(s.call_oi_change, dec("8000"));
    assert_eq!(s.put_oi_change, dec("-5000"));
    assert_eq!(s.call_iv, dec("0.185"));
    assert_eq!(s.put_delta, dec("-0.48"));
    assert_eq!(s.call_gamma, dec("0.0045"));
}

#[test]
fn strike_positioning_rejects_negative_oi_iv_gamma() {
    // Each non-negativity-constrained field, one at a time.
    assert!(
        make(
            dec("-1"),
            dec("140000"),
            dec("0.185"),
            dec("0.178"),
            dec("0.0045"),
            dec("0.0045"),
        )
        .is_err()
    );
    assert!(
        make(
            dec("120000"),
            dec("-1"),
            dec("0.185"),
            dec("0.178"),
            dec("0.0045"),
            dec("0.0045"),
        )
        .is_err()
    );
    assert!(
        make(
            dec("120000"),
            dec("140000"),
            dec("-0.01"),
            dec("0.178"),
            dec("0.0045"),
            dec("0.0045"),
        )
        .is_err()
    );
    assert!(
        make(
            dec("120000"),
            dec("140000"),
            dec("0.185"),
            dec("-0.01"),
            dec("0.0045"),
            dec("0.0045"),
        )
        .is_err()
    );
    assert!(
        make(
            dec("120000"),
            dec("140000"),
            dec("0.185"),
            dec("0.178"),
            dec("-0.1"),
            dec("0.0045"),
        )
        .is_err()
    );
    assert!(
        make(
            dec("120000"),
            dec("140000"),
            dec("0.185"),
            dec("0.178"),
            dec("0.0045"),
            dec("-0.1"),
        )
        .is_err()
    );
}

#[test]
fn strike_positioning_negative_returns_validation_error() {
    let err = make(
        dec("-1"),
        dec("140000"),
        dec("0.185"),
        dec("0.178"),
        dec("0.0045"),
        dec("0.0045"),
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn strike_positioning_allows_negative_oi_change_and_put_delta() {
    // OI *changes* and put deltas are negative in real chains; the invariant
    // only constrains OI/IV/gamma magnitudes.
    let s = make(
        dec("120000"),
        dec("140000"),
        dec("0.185"),
        dec("0.178"),
        dec("0.0045"),
        dec("0.0045"),
    )
    .unwrap();
    assert!(s.call_oi >= Decimal::ZERO);
    assert!(s.put_oi >= Decimal::ZERO);
    assert!(s.call_iv >= Decimal::ZERO);
    assert!(s.put_iv >= Decimal::ZERO);
    assert!(s.call_gamma >= Decimal::ZERO);
    assert!(s.put_gamma >= Decimal::ZERO);
}

#[test]
fn strike_positioning_serde_roundtrip() {
    let s = make(
        dec("120000"),
        dec("140000"),
        dec("0.185"),
        dec("0.178"),
        dec("0.0045"),
        dec("0.0045"),
    )
    .unwrap();
    let json = serde_json::to_string(&s).unwrap();
    let decoded: StrikePositioning = serde_json::from_str(&json).unwrap();
    assert_eq!(s, decoded);
}

fn strike_gamma(rupees: i64, gex: &str) -> StrikeGamma {
    StrikeGamma {
        strike: strike(rupees),
        gamma: dec("0.10"),
        oi: dec("100"),
        gex: dec(gex),
    }
}

#[test]
fn gamma_flip_detects_negative_to_positive_crossing() {
    let exposure = GammaExposure::compute(vec![
        strike_gamma(2500, "-10.0"),
        strike_gamma(2520, "6.0"),
        strike_gamma(2530, "8.0"),
    ]);
    // cumulative GEX: -10 -> -4 -> +4; sign flips between 2520 and 2530.
    assert_eq!(exposure.flip, dec("2530.0"));
    assert_eq!(exposure.net, dec("4.0"));
    assert_eq!(exposure.total_gex(), dec("4.0"));
}

#[test]
fn gamma_flip_detects_positive_to_negative_crossing() {
    let exposure = GammaExposure::compute(vec![
        strike_gamma(2500, "10.0"),
        strike_gamma(2520, "-15.0"),
        strike_gamma(2530, "-5.0"),
    ]);
    // cumulative GEX: 10 -> -5 -> -10; sign flips between 2500 and 2520.
    assert_eq!(exposure.flip, dec("2520.0"));
    assert_eq!(exposure.net, dec("-10.0"));
    assert_eq!(exposure.total_gex(), dec("-10.0"));
}

#[test]
fn gamma_flip_sorts_strikes_ascending_before_walking() {
    let exposure = GammaExposure::compute(vec![
        strike_gamma(2530, "8.0"),
        strike_gamma(2500, "-10.0"),
        strike_gamma(2520, "6.0"),
    ]);
    assert_eq!(exposure.flip, dec("2530.0"));
}

#[test]
fn gamma_flip_returns_zero_when_no_crossing() {
    let exposure =
        GammaExposure::compute(vec![strike_gamma(2500, "10.0"), strike_gamma(2520, "5.0")]);
    assert_eq!(exposure.flip, Decimal::ZERO);
    assert_eq!(exposure.net, dec("15.0"));
    assert_eq!(exposure.total_gex(), dec("15.0"));
}

#[test]
fn gamma_exposure_empty_chain() {
    let exposure = GammaExposure::compute(vec![]);
    assert_eq!(exposure.net, Decimal::ZERO);
    assert_eq!(exposure.flip, Decimal::ZERO);
    assert_eq!(exposure.total_gex(), Decimal::ZERO);
}

#[test]
fn option_chain_snapshot_serde_roundtrip() {
    let chain = OptionChainSnapshot::new(
        DateTime::parse_from_rfc3339("2026-01-15T09:30:00Z")
            .unwrap()
            .with_timezone(&Utc),
        InstrumentId::try_new(Exchange::Nse, "RELIANCE").unwrap(),
        NaiveDate::from_ymd_opt(2026, 1, 29).unwrap(),
        vec![
            make(
                dec("120000"),
                dec("140000"),
                dec("0.185"),
                dec("0.178"),
                dec("0.0045"),
                dec("0.0045"),
            )
            .unwrap(),
            make(
                dec("98000"),
                dec("110000"),
                dec("0.192"),
                dec("0.181"),
                dec("0.0048"),
                dec("0.0048"),
            )
            .unwrap(),
        ],
    );
    let json = serde_json::to_string(&chain).unwrap();
    let decoded: OptionChainSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(chain, decoded);
    assert_eq!(decoded.underlying.symbol(), "RELIANCE");
}
