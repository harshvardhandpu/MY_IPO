//! Phase 2C: versioned ranking algorithm interface + deterministic dev algorithm.

use sanket_ranking::{
    DevRankingAlgorithm, RankedIpo, RankingAlgorithm, RecommendationError, RecommendedAllocation,
};

fn dev() -> DevRankingAlgorithm {
    DevRankingAlgorithm::new()
}

#[test]
fn dev_algorithm_reports_version() {
    assert_eq!(dev().version(), "dev-ranking-v001");
}

#[test]
fn dev_algorithm_requires_public_fields() {
    // The dev algorithm declares the public fields it needs; empty is fine for now.
    assert!(dev().required_public_fields().is_empty());
}

#[test]
fn first_ipo_gets_largest_share_and_rest_get_less() {
    // Dev algorithm: 50% / 30% of available accounts, deterministic.
    let ipos = vec!["Primary".to_owned(), "Secondary".to_owned()];
    let rec = dev().evaluate(ipos, 10).expect("evaluate");
    assert_eq!(rec.algorithm_version(), "dev-ranking-v001");
    let ranked: Vec<&RankedIpo> = rec.ipos().collect();
    assert_eq!(ranked.len(), 2);

    let a = &ranked[0];
    assert_eq!(a.typed_name(), "Primary");
    assert!(!a.skip());
    assert_eq!(a.recommended_account_count(), 5); // 50% of 10

    let b = &ranked[1];
    assert_eq!(b.typed_name(), "Secondary");
    assert!(!b.skip());
    assert_eq!(b.recommended_account_count(), 3); // 30% of 10
}

#[test]
fn deterministic_ranking_is_stable() {
    let ipos = vec![
        "Alpha IPO".to_owned(),
        "Beta IPO".to_owned(),
        "Gamma IPO".to_owned(),
    ];
    let r1 = dev().evaluate(ipos.clone(), 6).unwrap();
    let r2 = dev().evaluate(ipos, 6).unwrap();
    let s1: Vec<String> = r1.ipos().map(|i| i.typed_name().to_owned()).collect();
    let s2: Vec<String> = r2.ipos().map(|i| i.typed_name().to_owned()).collect();
    assert_eq!(s1, s2, "dev algorithm must be deterministic");
}

#[test]
fn five_three_one_and_skip_distribution() {
    // Exercise ranking / allocation / skip / reason response structures.
    let ipos = vec![
        "Wide IPO".to_owned(),
        "Mid IPO".to_owned(),
        "Narrow IPO".to_owned(),
        "Bad IPO".to_owned(),
    ];
    let rec = dev().evaluate(ipos, 10).unwrap();
    let counts: Vec<u32> = rec.ipos().map(|i| i.recommended_account_count()).collect();
    // Deterministic distribution over 10 available accounts: 5 / 3 / 1 / skip.
    assert_eq!(counts, vec![5, 3, 1, 0]);
    let bad = rec.ipos().last().unwrap();
    assert!(bad.skip());
    assert!(!bad.reason().is_empty());
}

#[test]
fn recommendation_carries_allocations_for_non_skipped() {
    let ipos = vec![
        "Wide IPO".to_owned(),
        "Mid IPO".to_owned(),
        "Narrow IPO".to_owned(),
        "Skip This".to_owned(),
    ];
    let rec = dev().evaluate(ipos, 3).unwrap();
    // Apply gives allocations only for non-skipped IPOs (first three).
    let allocations: Vec<RecommendedAllocation> = rec.apply().collect();
    assert_eq!(allocations.len(), 3);
    assert_eq!(allocations[0].typed_name(), "Wide IPO");
    assert_eq!(allocations[2].typed_name(), "Narrow IPO");
    // The skipped IPO is absent.
    assert!(allocations.iter().all(|a| a.typed_name() != "Skip This"));
}

#[test]
fn skip_flag_never_allocates() {
    let rec = dev()
        .evaluate(
            vec![
                "A".to_owned(),
                "B".to_owned(),
                "C".to_owned(),
                "Skipped".to_owned(),
            ],
            4,
        )
        .unwrap();
    let allocations: Vec<RecommendedAllocation> = rec.apply().collect();
    // Skipped IPO never appears.
    assert!(allocations.iter().all(|a| a.typed_name() != "Skipped"));
    let skipped = rec.ipos().last().unwrap();
    assert!(skipped.skip());
}

#[test]
fn empty_ipo_list_is_error() {
    assert!(matches!(
        dev().evaluate(vec![], 3),
        Err(RecommendationError::NoIpos)
    ));
}

#[test]
fn recommendation_output_is_not_advice() {
    // The dev algorithm label must make clear it is NOT investment advice.
    let rec = dev().evaluate(vec!["X".to_owned()], 1).unwrap();
    assert!(rec.label().contains("NOT INVESTMENT ADVICE"));
    assert!(rec.label().to_lowercase().contains("development"));
}
