using Pamoja.Zenoh;

using static Guides.Guide;

namespace Guides;

/// <summary>The Zenoh key expression guide example; see docs/guides/zenoh.md.</summary>
public static class ZenohGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A key expression names a set of keys. `*` stands for exactly one chunk, so this
        // selects the battery of any node, and not a battery nested deeper.
        const string AnyNode = "fleet/*/battery";
        foreach (string key in new[] { "fleet/n7/battery", "fleet/n7/rack/battery" })
        {
            Console.WriteLine($"{AnyNode} covers {key}: {KeyExpression.Matches(AnyNode, key)}");
        }

        // `**` stands for any number of chunks, including none, which is what a
        // subscription covering a whole subtree wants.
        Console.WriteLine(
            "fleet/** covers a nested key: "
            + KeyExpression.Matches("fleet/**", "fleet/n7/rack/battery"));
        Console.WriteLine(
            "fleet/**/battery covers fleet/battery: "
            + KeyExpression.Matches("fleet/**/battery", "fleet/battery"));

        // Two expressions that select the same keys have one canonical form. Comparing or
        // routing on the written form would treat these as different subscriptions.
        const string Written = "fleet/**/**/battery";
        string? canonical = KeyExpression.Canonize(Written);
        Console.WriteLine(
            $"{Written} is canonical: {KeyExpression.IsCanon(Written)},"
            + $" and canonizes to {canonical}");

        // A malformed expression is rejected rather than canonized into something
        // plausible.
        const string Malformed = "fleet//battery";
        Console.WriteLine(
            $"{Malformed} is valid: {KeyExpression.IsValid(Malformed)},"
            + $" canonizes to {KeyExpression.Canonize(Malformed) ?? "nothing"}");
        // ANCHOR_END: example

        Expect(KeyExpression.IsValid(AnyNode), "the pattern is well formed");
        Expect(KeyExpression.Matches(AnyNode, "fleet/n7/battery"), "one chunk matches");
        Expect(!KeyExpression.Matches(AnyNode, "fleet/n7/rack/battery"), "but not two");
        Expect(KeyExpression.Matches("fleet/**", "fleet/n7/rack/battery"), "any depth matches");
        Expect(KeyExpression.Matches("fleet/**/battery", "fleet/battery"), "including none");
        Expect(!KeyExpression.IsCanon(Written), "a repeated wildcard is not canonical");
        Expect(canonical == "fleet/**/battery", "and canonizes to the single one");
        Expect(!KeyExpression.IsValid(Malformed), "an empty chunk is malformed");
        Expect(KeyExpression.Canonize(Malformed) is null, "and canonizes to nothing");
    }
}
