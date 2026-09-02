namespace Pamoja.Core.Interop;

/// <summary>
/// The physical voltage level on a pin, mirroring <c>PamojaPinLevel</c> in
/// <c>pamoja.h</c>.
/// </summary>
public enum PamojaPinLevel
{
    /// <summary>A low level, near ground.</summary>
    Low = 0,

    /// <summary>A high level, near the supply voltage.</summary>
    High = 1,
}
