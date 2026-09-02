namespace Pamoja.Core.Interop;

/// <summary>
/// A stepper drive pattern, mirroring <c>PamojaStepDrive</c> in <c>pamoja.h</c>.
/// </summary>
public enum PamojaStepDrive
{
    /// <summary>One coil at a time: four steps, least torque and least power.</summary>
    Wave = 0,

    /// <summary>Two adjacent coils at a time: four steps, most torque.</summary>
    FullStep = 1,

    /// <summary>Alternating one and two coils: eight steps, double resolution.</summary>
    HalfStep = 2,
}
