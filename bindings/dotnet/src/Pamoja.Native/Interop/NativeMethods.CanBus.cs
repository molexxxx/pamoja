using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for a node on a CAN bus, simulated or a kernel interface through
/// SocketCAN, mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must be
/// updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>A bus kind: a kernel CAN interface reached through SocketCAN.</summary>
    public const byte CanBusDevice = 0;

    /// <summary>A bus kind: a bus inside the program.</summary>
    public const byte CanBusSimulated = 1;

    /// <summary>Makes a new bus inside the program, with the returned node the first on it.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_can_bus_simulated();

    /// <summary>Opens a kernel CAN interface through SocketCAN.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_can_bus_open(string interfaceName, out IntPtr outBus);

    /// <summary>Puts another node on the same bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_can_bus_join(IntPtr bus, out IntPtr outBus);

    /// <summary>Returns what a node's bus is.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_can_bus_kind(IntPtr bus);

    /// <summary>Sends a frame to every other node on the bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_can_bus_send(IntPtr bus, IntPtr frame);

    /// <summary>Takes the next frame the node keeps, leaving the handle null on a timeout.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_can_bus_receive(IntPtr bus, ulong timeoutMicros, out IntPtr outFrame);

    /// <summary>Keeps only the frames that pass at least one of the filters.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_can_bus_set_filters(
        IntPtr bus,
        ReadOnlySpan<PamojaCanFilter> filters,
        nuint len);

    /// <summary>Keeps every frame again.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_can_bus_clear_filters(IntPtr bus);

    /// <summary>Returns how many frames a node has sent.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_can_bus_sent(IntPtr bus);

    /// <summary>Returns how many frames a node has received.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_can_bus_received(IntPtr bus);

    /// <summary>Returns how long receives on a node have waited without a frame, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_can_bus_waited_micros(IntPtr bus);

    /// <summary>Releases a node; it leaves the bus. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_can_bus_free(IntPtr bus);

    /// <summary>A filter that passes one identifier and nothing else.</summary>
    [LibraryImport(Library)]
    public static partial PamojaCanFilter pamoja_can_filter_exact(uint id, [MarshalAs(UnmanagedType.U1)] bool extended);

    /// <summary>A filter that passes one J1939 parameter group.</summary>
    [LibraryImport(Library)]
    public static partial PamojaCanFilter pamoja_can_filter_pgn(uint pgn);

    /// <summary>Reports whether a frame with an identifier passes a filter.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_can_filter_matches(
        PamojaCanFilter filter,
        uint id,
        [MarshalAs(UnmanagedType.U1)] bool extended);
}
