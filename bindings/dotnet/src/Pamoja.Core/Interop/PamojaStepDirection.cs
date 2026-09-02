namespace Pamoja.Core.Interop;

/// <summary>
/// Which way to step a motor, mirroring <c>PamojaStepDirection</c> in
/// <c>pamoja.h</c>.
/// </summary>
public enum PamojaStepDirection
{
    /// <summary>Advance the sequence, turning the shaft one way.</summary>
    Forward = 0,

    /// <summary>Reverse the sequence, turning the shaft the other way.</summary>
    Backward = 1,
}
