using Pamoja;

namespace Guides;

/// <summary>The checks the guide examples use.</summary>
public static class Guide
{
    /// <summary>Throws when an expectation in a guide example does not hold.</summary>
    /// <param name="condition">The expectation.</param>
    /// <param name="message">What the example was showing.</param>
    public static void Expect(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException($"expectation failed: {message}");
        }
    }

    /// <summary>Reports whether a call the library should refuse did refuse.</summary>
    /// <param name="call">The call under test.</param>
    /// <returns><c>true</c> when it threw.</returns>
    public static bool Refused(Action call)
    {
        ArgumentNullException.ThrowIfNull(call);

        try
        {
            call();
            return false;
        }
        catch (PamojaException)
        {
            return true;
        }
    }
}
