namespace Pamoja.Native.Interop;

/// <summary>
/// The result of a fallible native call, mirroring <c>PamojaStatus</c> in
/// <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// A value of <see cref="Ok"/> means success; any other value indicates a failure
/// whose message is available from
/// <see cref="NativeMethods.pamoja_last_error_message"/> on the same thread.
/// </remarks>
public enum PamojaStatus
{
    /// <summary>The call succeeded.</summary>
    Ok = 0,

    /// <summary>A transport-level failure while connecting, sending, or receiving.</summary>
    Transport = 1,

    /// <summary>A device or peripheral input/output operation failed.</summary>
    Io = 2,

    /// <summary>A payload could not be encoded or decoded.</summary>
    Codec = 3,

    /// <summary>The operation targeted a resource that is closed or disconnected.</summary>
    Closed = 4,

    /// <summary>The requested capability is not compiled into this build.</summary>
    Unsupported = 5,

    /// <summary>An argument was null or otherwise invalid, such as non-UTF-8 text.</summary>
    InvalidArgument = 6,

    /// <summary>A failure that does not map onto a more specific status.</summary>
    Other = 7,

    /// <summary>A native panic was caught at the boundary; the call had no effect.</summary>
    Panic = 8,

    /// <summary>A security check failed, such as an invalid identity or a bad signature.</summary>
    Auth = 9,
}
