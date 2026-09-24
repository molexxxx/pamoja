namespace Pamoja.Profile;

/// <summary>
/// The name a misspelled one was probably meant to be, worked out the way the library's own
/// parser does, so a refusal reads the same in every language.
/// </summary>
internal static class Nearest
{
    /// <summary>The control kinds the library ships, as a manifest names them.</summary>
    internal static readonly string[] BuiltInKinds = ["setpoint", "level", "surge", "monitor"];

    /// <summary>Picks the allowed name a given one is most likely a misspelling of.</summary>
    /// <param name="given">The name the file or the program used.</param>
    /// <param name="allowed">The names it may use there, in the order to prefer them.</param>
    /// <returns>
    /// The closest allowed name, or <c>null</c> when none is within two edits and closer
    /// than half the given name's length.
    /// </returns>
    internal static string? Of(string given, IEnumerable<string> allowed)
    {
        int limit = Math.Clamp(given.Length / 2, 1, 2);
        string? best = null;
        int fewest = int.MaxValue;
        foreach (string name in allowed)
        {
            int edits = Distance(given, name);
            if (edits <= limit && edits < fewest)
            {
                best = name;
                fewest = edits;
            }
        }

        return best;
    }

    /// <summary>The reason a control kind with no policy behind it is refused.</summary>
    /// <param name="kind">The custom kind the profile names.</param>
    /// <param name="registered">The kinds a registry knows, in name order.</param>
    /// <returns>The reason, worded as the library words it.</returns>
    internal static string Unresolved(string kind, IReadOnlyList<string> registered)
    {
        string? near = Of(kind, BuiltInKinds.Concat(registered));
        string hint = near is not null
            ? $"; did you mean `{near}`?"
            : registered.Count == 0
                ? "; resolve the profile through a PolicyRegistry that registers it"
                : $"; the registry knows `{string.Join("`, `", registered)}`";
        return $"codec error: no policy decides the control kind `{kind}`, which is not built in{hint}";
    }

    private static int Distance(string a, string b)
    {
        int[] previous = Enumerable.Range(0, b.Length + 1).ToArray();
        for (int i = 0; i < a.Length; i++)
        {
            int[] current = new int[b.Length + 1];
            current[0] = i + 1;
            for (int j = 0; j < b.Length; j++)
            {
                int substitute = previous[j] + (a[i] == b[j] ? 0 : 1);
                current[j + 1] = Math.Min(substitute, Math.Min(previous[j + 1] + 1, current[j] + 1));
            }

            previous = current;
        }

        return previous[b.Length];
    }
}
