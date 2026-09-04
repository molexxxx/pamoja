using Pamoja.Native.Interop;

namespace Pamoja.Zenoh;

/// <summary>The naming rules a Zenoh network addresses data by.</summary>
/// <remarks>
/// A key expression is a slash-separated path that may carry the <c>*</c> and
/// <c>**</c> wildcards, so one subscriber names a whole subtree of a fleet rather
/// than each node in it. Only the naming rules cross: running a Zenoh session
/// needs the std-only zenoh stack, which stays in the Rust crate.
/// </remarks>
public static class KeyExpression
{
    /// <summary>Reports whether a key expression is well formed.</summary>
    /// <param name="key">The expression to check.</param>
    /// <returns><c>true</c> when the expression is valid.</returns>
    public static bool IsValid(string key) => NativeMethods.pamoja_keyexpr_is_valid(key);

    /// <summary>Reports whether a key expression is already canonical.</summary>
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

    /// <summary>Reports whether a pattern selects a key.</summary>
    /// <param name="pattern">The expression that may carry wildcards.</param>
    /// <param name="key">The concrete key to test against it.</param>
    /// <returns><c>true</c> when the pattern selects the key.</returns>
    public static bool Matches(string pattern, string key) =>
        NativeMethods.pamoja_keyexpr_matches(pattern, key);
}
