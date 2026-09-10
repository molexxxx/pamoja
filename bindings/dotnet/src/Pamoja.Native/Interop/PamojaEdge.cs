namespace Pamoja.Native.Interop;

/// <summary>What a trigger reports when a reading changes its state, if anything.</summary>
public enum PamojaEdge
{
    /// <summary>Nothing changed.</summary>
    None = 0,

    /// <summary>The reading just crossed the line: the condition became true.</summary>
    Set = 1,

    /// <summary>The reading just came back past the release band: the condition stopped holding.</summary>
    Cleared = 2,
}
