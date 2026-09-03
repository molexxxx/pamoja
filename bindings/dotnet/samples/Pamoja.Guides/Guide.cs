namespace Guides;

/// <summary>The one check the guide examples use.</summary>
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
}
