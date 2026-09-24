using Pamoja.Native.Interop;

namespace Pamoja.Zenoh;

/// <summary>The naming rules a Zenoh network addresses data by.</summary>
/// <remarks>
/// A key expression is a slash-separated path that may carry the <c>*</c> and
/// <c>**</c> wildcards, so one subscriber names a whole subtree of a fleet rather
/// than each node in it. A chunk that starts with <c>@</c> is verbatim, and no
/// wildcard selects it. Only the naming rules cross: running a Zenoh session
/// needs the std-only zenoh stack, which stays in the Rust crate.
/// </remarks>
public static class KeyExpression
{
    /// <summary>Reports whether a key expression is well formed.</summary>
    /// <param name="key">The expression to check.</param>
    /// <returns><c>true</c> when the expression is valid.</returns>
    public static bool IsValid(string key) => NativeMethods.pamoja_keyexpr_is_valid(key);

    /// <summary>Reports whether a key expression is already canonical.</summary>
    /// <remarks>The canonical form is the only one a Zenoh session accepts.</remarks>
    /// <param name="key">The expression to check.</param>
    /// <returns><c>true</c> when the expression is canonical.</returns>
    public static bool IsCanon(string key) => NativeMethods.pamoja_keyexpr_is_canon(key);

    /// <summary>Rewrites a key expression into its canonical form.</summary>
    /// <remarks>
    /// Two expressions that select the same data have one canonical form, so
    /// canonizing before comparing or routing keeps equivalent expressions equal.
    /// </remarks>
    /// <param name="key">The expression to canonize.</param>
    /// <returns>The canonical form, or <c>null</c> if the expression is malformed.</returns>
    public static string? Canonize(string key) =>
        OwnedString.ReadOrNull(NativeMethods.pamoja_keyexpr_canonize(key));

    /// <summary>Joins two key expressions with a slash and canonizes the result.</summary>
    /// <param name="prefix">The leading expression.</param>
    /// <param name="suffix">The expression to place beneath it.</param>
    /// <returns>
    /// The joined expression in canonical form, or <c>null</c> if either side is
    /// empty or the joined expression is malformed.
    /// </returns>
    public static string? Join(string prefix, string suffix) =>
        OwnedString.ReadOrNull(NativeMethods.pamoja_keyexpr_join(prefix, suffix));

    /// <summary>Reports whether a pattern selects a concrete key.</summary>
    /// <param name="pattern">The expression that may carry wildcards.</param>
    /// <param name="key">The concrete key to test against it.</param>
    /// <returns><c>true</c> when the pattern selects the key.</returns>
    public static bool Matches(string pattern, string key) =>
        NativeMethods.pamoja_keyexpr_matches(pattern, key);

    /// <summary>Reports whether two key expressions share at least one key.</summary>
    /// <remarks>
    /// This is the relation Zenoh routes by: a publication on one reaches a
    /// subscriber on the other exactly when the two intersect.
    /// </remarks>
    /// <param name="a">One expression.</param>
    /// <param name="b">The other expression.</param>
    /// <returns>
    /// <c>true</c> when some key is selected by both, or <c>false</c> if none is
    /// or either expression is malformed.
    /// </returns>
    public static bool Intersects(string a, string b) =>
        NativeMethods.pamoja_keyexpr_intersects(a, b);

    /// <summary>Reports whether one key expression selects every key another selects.</summary>
    /// <param name="a">The expression that may be the wider one.</param>
    /// <param name="b">The expression tested for being covered by <paramref name="a"/>.</param>
    /// <returns>
    /// <c>true</c> when every key <paramref name="b"/> selects is also selected by
    /// <paramref name="a"/>, or <c>false</c> if not or either expression is malformed.
    /// </returns>
    public static bool Includes(string a, string b) =>
        NativeMethods.pamoja_keyexpr_includes(a, b);
}
