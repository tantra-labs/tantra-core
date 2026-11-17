//! Property-based invariants for `tantra_core::levels` (Plan 0003, Phase 1).

use proptest::prelude::*;
use rust_decimal::Decimal;
use tantra_core::{Level, LevelId, LevelKind, LevelMap, LevelSide};

proptest! {
    #[test]
    fn prop_valid_level_band_low_le_high_and_score_bounded(
        lo in any::<i64>(),
        hi in any::<i64>(),
        score in 0u8..=4u8,
    ) {
        let (low, high) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        let level = Level::new(
            LevelId(1),
            LevelKind::StrikeRound,
            LevelSide::Resistance,
            (Decimal::from(low), Decimal::from(high)),
            score,
        )
        .expect("generated level must be valid");
        prop_assert!(level.band.0 <= level.band.1);
        prop_assert!(level.score <= 4);
    }

    #[test]
    fn prop_level_rejects_inverted_band(lo in any::<i64>(), hi in any::<i64>()) {
        prop_assume!(lo > hi);
        let result = Level::new(
            LevelId(1),
            LevelKind::MaxPain,
            LevelSide::Support,
            (Decimal::from(lo), Decimal::from(hi)),
            2,
        );
        prop_assert!(result.is_err());
    }

    #[test]
    fn prop_level_rejects_score_over_four(
        lo in any::<i64>(),
        hi in any::<i64>(),
        score in 5u8..=u8::MAX,
    ) {
        let (low, high) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        let result = Level::new(
            LevelId(1),
            LevelKind::OrderBlock,
            LevelSide::Support,
            (Decimal::from(low), Decimal::from(high)),
            score,
        );
        prop_assert!(result.is_err());
    }

    #[test]
    fn prop_level_map_filters_partition_by_side(
        n_support in 0usize..8,
        n_resistance in 0usize..8,
    ) {
        let mut levels = Vec::new();
        let mut next_id = 0u64;
        for _ in 0..n_support {
            next_id += 1;
            levels.push(
                Level::new(
                    LevelId(next_id),
                    LevelKind::ElephantSupport1,
                    LevelSide::Support,
                    (Decimal::ZERO, Decimal::ONE),
                    1,
                )
                .unwrap(),
            );
        }
        for _ in 0..n_resistance {
            next_id += 1;
            levels.push(
                Level::new(
                    LevelId(next_id),
                    LevelKind::ElephantResistance1,
                    LevelSide::Resistance,
                    (Decimal::ZERO, Decimal::ONE),
                    1,
                )
                .unwrap(),
            );
        }
        let map = LevelMap::new(levels);
        prop_assert_eq!(map.support_levels().count(), n_support);
        prop_assert_eq!(map.resistance_levels().count(), n_resistance);
        prop_assert_eq!(
            map.support_levels().count() + map.resistance_levels().count(),
            map.levels.len()
        );
        for id in 1..=next_id {
            prop_assert!(map.get(LevelId(id)).is_some(), "level {id} must be findable");
        }
        prop_assert!(map.get(LevelId(next_id + 1)).is_none());
    }
}
