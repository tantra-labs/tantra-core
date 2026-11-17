//! Unit tests for `tantra_core::levels` (Plan 0003, Phase 1).

use rust_decimal::Decimal;
use tantra_core::{CoreError, Level, LevelId, LevelKind, LevelMap, LevelSide};

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

#[test]
fn level_new_accepts_valid_level() {
    let level = Level::new(
        LevelId(7),
        LevelKind::GammaFlip,
        LevelSide::Support,
        (dec("2470.00"), dec("2470.00")),
        4,
    )
    .unwrap();
    assert_eq!(level.id, LevelId(7));
    assert_eq!(level.kind, LevelKind::GammaFlip);
    assert_eq!(level.side, LevelSide::Support);
    assert_eq!(level.band, (dec("2470.00"), dec("2470.00")));
    assert_eq!(level.score, 4);
}

#[test]
fn level_new_rejects_inverted_band() {
    let err = Level::new(
        LevelId(1),
        LevelKind::ElephantSupport1,
        LevelSide::Support,
        (dec("2500.00"), dec("2400.00")),
        2,
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn level_new_rejects_score_above_four() {
    let err = Level::new(
        LevelId(1),
        LevelKind::MaxPain,
        LevelSide::Support,
        (dec("2400.00"), dec("2500.00")),
        5,
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[test]
fn level_new_allows_degenerate_zero_width_band() {
    let level = Level::new(
        LevelId(1),
        LevelKind::StrikeRound,
        LevelSide::Support,
        (dec("2500.00"), dec("2500.00")),
        0,
    )
    .unwrap();
    assert_eq!(level.band.0, level.band.1);
}

#[test]
fn level_map_get_and_filters() {
    let levels = vec![
        Level::new(
            LevelId(1),
            LevelKind::ElephantSupport1,
            LevelSide::Support,
            (dec("2448.00"), dec("2458.00")),
            4,
        )
        .unwrap(),
        Level::new(
            LevelId(2),
            LevelKind::ElephantSupport2,
            LevelSide::Support,
            (dec("2421.00"), dec("2432.00")),
            3,
        )
        .unwrap(),
        Level::new(
            LevelId(3),
            LevelKind::ElephantResistance1,
            LevelSide::Resistance,
            (dec("2490.00"), dec("2500.00")),
            4,
        )
        .unwrap(),
        Level::new(
            LevelId(4),
            LevelKind::OiWallCall,
            LevelSide::Resistance,
            (dec("2500.00"), dec("2500.00")),
            3,
        )
        .unwrap(),
    ];
    let map = LevelMap::new(levels);

    assert!(map.get(LevelId(1)).is_some());
    assert_eq!(
        map.get(LevelId(1)).unwrap().kind,
        LevelKind::ElephantSupport1
    );
    assert!(map.get(LevelId(99)).is_none());

    let support: Vec<LevelId> = map.support_levels().map(|l| l.id).collect();
    assert_eq!(support, vec![LevelId(1), LevelId(2)]);

    let resistance: Vec<LevelId> = map.resistance_levels().map(|l| l.id).collect();
    assert_eq!(resistance, vec![LevelId(3), LevelId(4)]);
}

#[test]
fn level_serde_roundtrip() {
    let level = Level::new(
        LevelId(1),
        LevelKind::OrderBlock,
        LevelSide::Support,
        (dec("2462.00"), dec("2472.00")),
        2,
    )
    .unwrap();
    let json = serde_json::to_string(&level).unwrap();
    let decoded: Level = serde_json::from_str(&json).unwrap();
    assert_eq!(level, decoded);
}
