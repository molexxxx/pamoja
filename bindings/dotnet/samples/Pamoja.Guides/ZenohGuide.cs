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
        // A key expression names a set of keys. Chunks sit between slashes, `*` stands for
        // exactly one chunk, whatever it holds, and `**` for any number of them, including none.
        static string Selects(string pattern, string key)
        {
            string verdict = KeyExpression.Matches(pattern, key) ? "covers" : "misses";
            return $"{verdict,-10}{pattern} {verdict} {key}";
        }
        Console.WriteLine(Selects("farm/*/power", "farm/t7/power"));
        Console.WriteLine(Selects("farm/*/power", "farm/substation/power"));
        Console.WriteLine(
            $"{Selects("farm/*/power", "farm/row2/t14/power")}, since * is exactly one chunk");
        Console.WriteLine(Selects("farm/**/power", "farm/row2/t14/power"));
        Console.WriteLine(
            $"{Selects("farm/**/alarm", "farm/alarm")}, where ** is no chunk at all");

        // `$*` stands for any run of characters inside one chunk, so it selects on part of a
        // name.
        Console.WriteLine(Selects("farm/t$*/power", "farm/t7/power"));
        Console.WriteLine(Selects("farm/t$*/power", "farm/substation/power"));

        // One set of keys has one canonical spelling, and a Zenoh session accepts no other.
        foreach (string written in new[] { "farm/*/**/power", "farm/**/*/power", "farm/**/**/power" })
        {
            string? canonical = KeyExpression.Canonize(written);
            if (KeyExpression.IsCanon(written))
            {
                Console.WriteLine($"canonical {written}, as written");
            }
            else
            {
                Console.WriteLine($"rewritten {written} is spelled {canonical}");
            }
        }

        // Joining places one expression beneath another, and canonizes the seam between them.
        foreach ((string prefix, string suffix) in new[] { ("farm/t7", "power"), ("farm/**", "*/power") })
        {
            Console.WriteLine(
                $"joined    {prefix} and {suffix} make {KeyExpression.Join(prefix, suffix)}");
        }

        // A malformed expression is refused rather than repaired into something plausible.
        foreach ((string written, string why) in new[]
        {
            ("farm//power", "a chunk is empty"),
            ("farm/t7*/power", "* stands alone in its chunk, or after $"),
            ("farm/t7/power?", "? and # are reserved"),
        })
        {
            if (!KeyExpression.IsValid(written) && KeyExpression.Canonize(written) is null)
            {
                Console.WriteLine($"malformed {written}, since {why}");
            }
        }
        // ANCHOR_END: example

        Expect(KeyExpression.Matches("farm/*/power", "farm/substation/power"), "* is any chunk");
        Expect(!KeyExpression.Matches("farm/*/power", "farm/row2/t14/power"), "but only one");
        Expect(KeyExpression.Matches("farm/**/alarm", "farm/alarm"), "** can be no chunk");
        Expect(!KeyExpression.Matches("farm/t$*/power", "farm/substation/power"), "$* keeps its prefix");
        Expect(KeyExpression.Canonize("farm/**/*/power") == "farm/*/**/power", "* moves ahead of **");
        Expect(KeyExpression.Join("farm/**", "*/power") == "farm/*/**/power", "the seam is canonized");

        // ANCHOR: relations
        // Two expressions intersect when some key belongs to both. That is the question a
        // router asks before it forwards a publication on one to a subscriber on the other.
        static string Overlap(string a, string b) =>
            KeyExpression.Intersects(a, b)
                ? $"overlap   {a} and {b} share a key"
                : $"disjoint  {a} and {b} share no key";
        Console.WriteLine(Overlap("farm/*/power", "farm/t7/**"));
        Console.WriteLine(Overlap("farm/*/power", "farm/*/alarm"));

        // One includes the other when every key of the second belongs to the first, so a
        // bridge that already holds the wider subscription declares nothing new for the
        // narrower one.
        static string Covers(string a, string b) =>
            KeyExpression.Includes(a, b)
                ? $"included  {a} covers every key of {b}"
                : $"wider     {b} holds keys {a} does not";
        Console.WriteLine(Covers("farm/**", "farm/*/power"));
        Console.WriteLine(Covers("farm/*/power", "farm/**"));

        // Two spellings of one set include each other, which compares expressions nobody
        // canonized.
        (string one, string other) = ("farm/**/*/power", "farm/*/**/power");
        if (one != other && KeyExpression.Includes(one, other) && KeyExpression.Includes(other, one))
        {
            Console.WriteLine($"same      {one} and {other} select the same keys");
        }
        // ANCHOR_END: relations

        Expect(KeyExpression.Intersects("farm/*/power", "farm/t7/**"), "both select farm/t7/power");
        Expect(!KeyExpression.Intersects("farm/*/power", "farm/*/alarm"), "no key ends in both");
        Expect(KeyExpression.Includes("farm/**", "farm/*/power"), "the subtree covers the pattern");
        Expect(!KeyExpression.Includes("farm/*/power", "farm/**"), "but not the other way round");

        // ANCHOR: sealed
        // A chunk that starts with @ is verbatim: no wildcard selects it, and only the same
        // chunk matches it. A second payload version under @v2 stays out of every
        // subscription that does not name it, the way Zenoh keeps its own administration
        // space out of **.
        const string Current = "farm/@v2/t7/power";
        Console.WriteLine(
            $"{Selects("farm/**", Current)}, since no wildcard selects a chunk that starts with @");
        Console.WriteLine(Selects("farm/@v2/**", Current));
        Console.WriteLine(
            $"{Overlap("farm/@v1/**", "farm/@v2/**")}, so a reader of one version never sees the other");
        // ANCHOR_END: sealed

        Expect(!KeyExpression.Matches("farm/**", Current), "** stops at a verbatim chunk");
        Expect(KeyExpression.Matches("farm/@v2/**", Current), "naming the chunk reaches it");
        Expect(!KeyExpression.Intersects("farm/@v1/**", "farm/@v2/**"), "two versions are sealed apart");
    }
}
