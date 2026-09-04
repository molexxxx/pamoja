namespace Pamoja;

/// <summary>The exception thrown when a pamoja operation fails.</summary>
public sealed class PamojaException : Exception
{
    /// <summary>Creates an exception with the given message.</summary>
    /// <param name="message">A human-readable description of the failure.</param>
    public PamojaException(string message)
        : base(message)
    {
    }

    /// <summary>Creates an exception with the given message and underlying cause.</summary>
    /// <param name="message">A human-readable description of the failure.</param>
    /// <param name="innerException">The underlying cause.</param>
    public PamojaException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
