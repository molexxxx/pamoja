namespace Pamoja.Core.Interop;

/// <summary>
/// The signal transition that triggers a pin interrupt, mirroring
/// <c>PamojaPinEdge</c> in <c>pamoja.h</c>.
/// </summary>
public enum PamojaPinEdge
{
    /// <summary>A low-to-high transition.</summary>
    Rising = 0,

    /// <summary>A high-to-low transition.</summary>
    Falling = 1,

    /// <summary>Either transition.</summary>
    Both = 2,
}
