namespace Pamoja.Native.Interop;

/// <summary>
/// The functions a host supplies to stand as a transport, mirroring
/// <c>PamojaTransportCallbacks</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// <c>Connect</c>, <c>Send</c>, and <c>Subscribe</c> are required; <c>Recv</c> is null
/// for a link that only sends, and <c>Release</c> is null when the user data needs
/// no cleanup. Every pointer targets a method marked
/// <see cref="System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute"/> with the
/// C calling convention.
/// </remarks>
[System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]
public unsafe struct PamojaTransportCallbacks
{
    /// <summary>Establishes the link.</summary>
    public delegate* unmanaged[Cdecl]<IntPtr, PamojaStatus> Connect;

    /// <summary>Publishes a payload to a null-terminated UTF-8 topic.</summary>
    public delegate* unmanaged[Cdecl]<IntPtr, IntPtr, byte*, nuint, PamojaStatus> Send;

    /// <summary>Subscribes to a null-terminated UTF-8 topic filter.</summary>
    public delegate* unmanaged[Cdecl]<IntPtr, IntPtr, PamojaStatus> Subscribe;

    /// <summary>Waits for the next message, storing a handle or null once the link ended.</summary>
    public delegate* unmanaged[Cdecl]<IntPtr, IntPtr*, PamojaStatus> Recv;

    /// <summary>Frees the user data once nothing will call the host again.</summary>
    public delegate* unmanaged[Cdecl]<IntPtr, void> Release;
}
