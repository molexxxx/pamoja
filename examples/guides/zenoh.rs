//! The Zenoh key expression guide example; see docs/guides/zenoh.md.
//!
//! Run: `cargo run -p pamoja-examples --example zenoh`

use std::error::Error;

/// What a key expression selects across a wind farm's keys, the one spelling a Zenoh session
/// accepts, how two expressions relate, and the chunks no wildcard reaches: the questions a
/// subscriber, a router, and a bridge each ask before a sample moves.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_zenoh::keyexpr::{canonize, is_canon, is_valid, join, matches};

    // A key expression names a set of keys. Chunks sit between slashes, `*` stands for
    // exactly one chunk, whatever it holds, and `**` for any number of them, including none.
    let selects = |pattern: &str, key: &str| {
        let verdict = if matches(pattern, key) {
            "covers"
        } else {
            "misses"
        };
        format!("{verdict:<10}{pattern} {verdict} {key}")
    };
    println!("{}", selects("farm/*/power", "farm/t7/power"));
    println!("{}", selects("farm/*/power", "farm/substation/power"));
    println!(
        "{}, since * is exactly one chunk",
        selects("farm/*/power", "farm/row2/t14/power")
    );
    println!("{}", selects("farm/**/power", "farm/row2/t14/power"));
    println!(
        "{}, where ** is no chunk at all",
        selects("farm/**/alarm", "farm/alarm")
    );

    // `$*` stands for any run of characters inside one chunk, so it selects on part of a name.
    println!("{}", selects("farm/t$*/power", "farm/t7/power"));
    println!("{}", selects("farm/t$*/power", "farm/substation/power"));

    // One set of keys has one canonical spelling, and a Zenoh session accepts no other.
    for written in ["farm/*/**/power", "farm/**/*/power", "farm/**/**/power"] {
        let canonical = canonize(written).expect("a valid expression");
        if is_canon(written) {
            println!("canonical {written}, as written");
        } else {
            println!("rewritten {written} is spelled {canonical}");
        }
    }

    // Joining places one expression beneath another, and canonizes the seam between them.
    for (prefix, suffix) in [("farm/t7", "power"), ("farm/**", "*/power")] {
        let joined = join(prefix, suffix).expect("two valid halves");
        println!("joined    {prefix} and {suffix} make {joined}");
    }

    // A malformed expression is refused rather than repaired into something plausible.
    for (written, why) in [
        ("farm//power", "a chunk is empty"),
        ("farm/t7*/power", "* stands alone in its chunk, or after $"),
        ("farm/t7/power?", "? and # are reserved"),
    ] {
        if !is_valid(written) && canonize(written).is_none() {
            println!("malformed {written}, since {why}");
        }
    }
    // ANCHOR_END: example

    assert!(matches("farm/*/power", "farm/substation/power"));
    assert!(!matches("farm/*/power", "farm/row2/t14/power"));
    assert!(matches("farm/**/alarm", "farm/alarm"));
    assert!(!matches("farm/t$*/power", "farm/substation/power"));
    assert_eq!(
        canonize("farm/**/*/power").as_deref(),
        Some("farm/*/**/power")
    );
    assert_eq!(
        join("farm/**", "*/power").as_deref(),
        Some("farm/*/**/power")
    );

    // ANCHOR: relations
    use pamoja_zenoh::keyexpr::{includes, intersects};

    // Two expressions intersect when some key belongs to both. That is the question a router
    // asks before it forwards a publication on one to a subscriber on the other.
    let overlap = |a: &str, b: &str| {
        if intersects(a, b) {
            format!("overlap   {a} and {b} share a key")
        } else {
            format!("disjoint  {a} and {b} share no key")
        }
    };
    println!("{}", overlap("farm/*/power", "farm/t7/**"));
    println!("{}", overlap("farm/*/power", "farm/*/alarm"));

    // One includes the other when every key of the second belongs to the first, so a bridge
    // that already holds the wider subscription declares nothing new for the narrower one.
    let covers = |a: &str, b: &str| {
        if includes(a, b) {
            format!("included  {a} covers every key of {b}")
        } else {
            format!("wider     {b} holds keys {a} does not")
        }
    };
    println!("{}", covers("farm/**", "farm/*/power"));
    println!("{}", covers("farm/*/power", "farm/**"));

    // Two spellings of one set include each other, which compares expressions nobody
    // canonized.
    let (one, other) = ("farm/**/*/power", "farm/*/**/power");
    if one != other && includes(one, other) && includes(other, one) {
        println!("same      {one} and {other} select the same keys");
    }
    // ANCHOR_END: relations

    assert!(intersects("farm/*/power", "farm/t7/**"));
    assert!(!intersects("farm/*/power", "farm/*/alarm"));
    assert!(includes("farm/**", "farm/*/power"));
    assert!(!includes("farm/*/power", "farm/**"));

    // ANCHOR: sealed
    // A chunk that starts with @ is verbatim: no wildcard selects it, and only the same chunk
    // matches it. A second payload version under @v2 stays out of every subscription that
    // does not name it, the way Zenoh keeps its own administration space out of **.
    let current = "farm/@v2/t7/power";
    println!(
        "{}, since no wildcard selects a chunk that starts with @",
        selects("farm/**", current)
    );
    println!("{}", selects("farm/@v2/**", current));
    println!(
        "{}, so a reader of one version never sees the other",
        overlap("farm/@v1/**", "farm/@v2/**")
    );
    // ANCHOR_END: sealed

    assert!(!matches("farm/**", current));
    assert!(matches("farm/@v2/**", current));
    assert!(!intersects("farm/@v1/**", "farm/@v2/**"));
    Ok(())
}
