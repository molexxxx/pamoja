//! The Zenoh key expression guide example; see docs/guides/zenoh.md.

/// What a key expression selects, and why two that select the same keys have to be
/// compared in one canonical form rather than as written.
#[test]
fn a_pattern_selects_a_subtree_and_canonizes_before_it_is_compared() {
    // ANCHOR: example
    use pamoja_zenoh::keyexpr::{canonize, is_canon, is_valid, matches};

    // A key expression names a set of keys. `*` stands for exactly one chunk, so this
    // selects the battery of any node, and not a battery nested deeper.
    let any_node = "fleet/*/battery";
    for key in ["fleet/n7/battery", "fleet/n7/rack/battery"] {
        println!("{any_node} covers {key}: {}", matches(any_node, key));
    }

    // `**` stands for any number of chunks, including none, which is what a subscription
    // covering a whole subtree wants.
    println!(
        "fleet/** covers a nested key: {}",
        matches("fleet/**", "fleet/n7/rack/battery")
    );
    println!(
        "fleet/**/battery covers fleet/battery: {}",
        matches("fleet/**/battery", "fleet/battery")
    );

    // Two expressions that select the same keys have one canonical form. Comparing or
    // routing on the written form would treat these as different subscriptions.
    let written = "fleet/**/**/battery";
    let canonical = canonize(written).expect("a canonical form");
    println!(
        "{written} is canonical: {}, and canonizes to {canonical}",
        is_canon(written)
    );

    // A malformed expression is rejected rather than canonized into something plausible.
    let malformed = "fleet//battery";
    let refused = canonize(malformed);
    println!(
        "{malformed} is valid: {}, canonizes to {}",
        is_valid(malformed),
        refused.as_deref().unwrap_or("nothing")
    );
    // ANCHOR_END: example

    assert!(is_valid(any_node));
    assert!(matches(any_node, "fleet/n7/battery"));
    assert!(!matches(any_node, "fleet/n7/rack/battery"));
    assert!(matches("fleet/**", "fleet/n7/rack/battery"));
    assert!(matches("fleet/**/battery", "fleet/battery"));
    assert!(!is_canon(written));
    assert_eq!(canonical, "fleet/**/battery");
    assert!(is_canon("fleet/**/battery"));
    assert!(!is_valid(malformed));
    assert_eq!(refused, None);
}
