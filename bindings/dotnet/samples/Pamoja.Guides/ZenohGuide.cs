using Pamoja.Zenoh;

using static Guides.Guide;

namespace Guides;

/// <summary>The Zenoh keys guide example; see docs/guides/zenoh.md.</summary>
public static class ZenohGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A key expression names a set of keys. `*` stands for exactly one chunk, so this
        // selects the battery of any node, and not a battery nested deeper.
        Expect(KeyExpression.IsValid("fleet/*/battery"), "the pattern is well formed");
        Expect(
            KeyExpression.Matches("fleet/*/battery", "fleet/n7/battery"),
            "one chunk stands in for the node");
        Expect(
            !KeyExpression.Matches("fleet/*/battery", "fleet/n7/rack/battery"),
            "but not for two");

        // `**` stands for any number of chunks, including none, which is what a
        // subscription covering a whole subtree wants.
        Expect(
            KeyExpression.Matches("fleet/**", "fleet/n7/rack/battery"),
            "the subtree wildcard reaches any depth");
        Expect(
            KeyExpression.Matches("fleet/**/battery", "fleet/battery"),
            "including no chunks at all");

        // Two expressions that select the same keys have one canonical form. Comparing or
        // routing on the written form would treat these as different subscriptions.
        Expect(!KeyExpression.IsCanon("fleet/**/**/battery"), "a repeated wildcard is not canonical");
        Expect(
            KeyExpression.Canonize("fleet/**/**/battery") == "fleet/**/battery",
            "and collapses to the form that selects the same keys");
        Expect(KeyExpression.IsCanon("fleet/**/battery"), "which is canonical");

        // A malformed expression is rejected rather than canonized into something plausible.
        Expect(!KeyExpression.IsValid("fleet//battery"), "an empty chunk is malformed");
        Expect(KeyExpression.Canonize("fleet//battery") is null, "and has no canonical form");
        // ANCHOR_END: example
    }
}
