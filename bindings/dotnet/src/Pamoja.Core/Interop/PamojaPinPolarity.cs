namespace Pamoja.Core.Interop;

/// <summary>
/// Whether a signal is asserted by a high or a low physical level, mirroring
/// <c>PamojaPinPolarity</c> in <c>pamoja.h</c>.
/// </summary>
public enum PamojaPinPolarity
{
    /// <summary>A high level means asserted.</summary>
    ActiveHigh = 0,

    /// <summary>A low level means asserted.</summary>
    ActiveLow = 1,
}
