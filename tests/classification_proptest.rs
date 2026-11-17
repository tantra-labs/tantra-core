//! Property-based invariants for `tantra_core::classification` (Plan 0004).

use proptest::prelude::*;
use tantra_core::classification::{
    IndexId, Industry, MAX_CLASSIFICATION_LEN, MAX_INDEX_ID_LEN, MarketCapBucket, Sector,
};

proptest! {
    #[test]
    fn prop_sector_and_industry_roundtrip_and_idempotence(
        words in prop::collection::vec("[A-Za-z0-9&-]{1,12}", 1..5),
        leading in "[ \\t]{0,3}",
        trailing in "[ \\t]{0,3}",
    ) {
        let raw = format!("{leading}{}{trailing}", words.join("   "));
        prop_assume!(words.join(" ").chars().count() <= MAX_CLASSIFICATION_LEN);

        let sector = Sector::try_new(&raw).expect("valid normalized sector");
        prop_assert_eq!(sector.as_str(), words.join(" "));
        let re_parsed = Sector::try_new(sector.as_str()).unwrap();
        prop_assert_eq!(&sector, &re_parsed);

        let json = serde_json::to_string(&sector).unwrap();
        let decoded: Sector = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(sector, decoded);

        let industry = Industry::try_new(&raw).expect("valid normalized industry");
        let ind_json = serde_json::to_string(&industry).unwrap();
        let ind_decoded: Industry = serde_json::from_str(&ind_json).unwrap();
        prop_assert_eq!(industry, ind_decoded);
    }

    #[test]
    fn prop_index_id_uppercases_ascii_and_roundtrips(
        raw in "[a-zA-Z0-9_-]{1,32}",
    ) {
        prop_assume!(raw.len() <= MAX_INDEX_ID_LEN);
        let id = IndexId::try_new(&raw).unwrap();
        prop_assert_eq!(id.as_str(), raw.to_ascii_uppercase());
        prop_assert!(id.as_str().bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_' || b == b'-'));

        let json = serde_json::to_string(&id).unwrap();
        let decoded: IndexId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id, decoded);
    }

    #[test]
    fn prop_market_cap_bucket_serde_invariance(idx in 0usize..4) {
        let bucket = MarketCapBucket::ALL[idx];
        let json = serde_json::to_string(&bucket).unwrap();
        let decoded: MarketCapBucket = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(bucket, decoded);
    }
}
