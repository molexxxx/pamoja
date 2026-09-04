//! The Zenoh keys guide example; see docs/guides/zenoh.md.

/// The key-expression rules a fleet subscription depends on: what a pattern selects, where
/// the two wildcards differ, and why comparing expressions means canonizing them first.
#[test]
fn a_pattern_selects_a_subtree_and_canonizes_before_it_is_compared() {
    // ANCHOR: example
    use pamoja_zenoh::keyexpr::{canonize, is_canon, is_valid, matches};

    // A key expression names a set of keys. `*` stands for exactly one chunk, so this
    // selects the battery of any node, and not a battery nested deeper.
    assert!(is_valid("fleet/*/battery"));
    assert!(matches("fleet/*/battery", "fleet/n7/battery"));
    assert!(!matches("fleet/*/battery", "fleet/n7/rack/battery"));

    // `**` stands for any number of chunks, including none, which is what a subscription
    // covering a whole subtree wants.
    assert!(matches("fleet/**", "fleet/n7/rack/battery"));
    assert!(matches("fleet/**/battery", "fleet/battery"));

    // Two expressions that select the same keys have one canonical form. Comparing or
    // routing on the written form would treat these as different subscriptions.
    assert!(!is_canon("fleet/**/**/battery"));
    assert_eq!(
        canonize("fleet/**/**/battery").as_deref(),
        Some("fleet/**/battery")
    );
    assert!(is_canon("fleet/**/battery"));

    // A malformed expression is rejected rather than canonized into something plausible.
    assert!(!is_valid("fleet//battery"));
    assert_eq!(canonize("fleet//battery"), None);
    // ANCHOR_END: example
}
