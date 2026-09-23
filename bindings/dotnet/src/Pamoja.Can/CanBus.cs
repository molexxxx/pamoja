using Pamoja.Native.Interop;

namespace Pamoja.Can;

/// <summary>What a node's bus is.</summary>
public enum CanBusKind : byte
{
    /// <summary>A kernel CAN interface reached through SocketCAN.</summary>
    Device = NativeMethods.CanBusDevice,

    /// <summary>A bus inside the program.</summary>
    Simulated = NativeMethods.CanBusSimulated,
}

/// <summary>
/// A frame a node keeps: one whose identifier, masked, equals <see cref="Id"/>, masked, and whose
/// format is the filter's.
/// </summary>
/// <param name="Id">The identifier to match.</param>
/// <param name="Mask">The identifier bits that have to match.</param>
/// <param name="Extended">Whether the identifier is a 29-bit extended one.</param>
public readonly record struct CanFilter(uint Id, uint Mask, bool Extended = false)
{
    /// <summary>A filter that passes one identifier and nothing else.</summary>
    /// <param name="id">The identifier.</param>
    /// <param name="extended">Whether it is a 29-bit extended identifier.</param>
    /// <returns>The filter.</returns>
    public static CanFilter Exact(uint id, bool extended = false) =>
        FromNative(NativeMethods.pamoja_can_filter_exact(id, extended));

    /// <summary>
    /// A filter that passes one J1939 parameter group at any priority, from any source, and for
    /// an addressed group, to any destination.
    /// </summary>
    /// <param name="pgn">The parameter group number.</param>
    /// <returns>The filter.</returns>
    public static CanFilter Pgn(uint pgn) => FromNative(NativeMethods.pamoja_can_filter_pgn(pgn));

    /// <summary>Reports whether a frame with an identifier passes the filter.</summary>
    /// <param name="id">The frame's identifier.</param>
    /// <param name="extended">Whether it is a 29-bit extended identifier.</param>
    /// <returns><c>true</c> when the formats agree and the masked bits are equal.</returns>
    public bool Matches(uint id, bool extended = false) =>
        NativeMethods.pamoja_can_filter_matches(ToNative(), id, extended);

    /// <summary>Converts to the flat struct the C ABI takes.</summary>
    /// <returns>The interop representation.</returns>
    internal PamojaCanFilter ToNative() => new()
    {
        Id = Id,
        Mask = Mask,
        Extended = Extended ? (byte)1 : (byte)0,
    };

    private static CanFilter FromNative(PamojaCanFilter filter) =>
        new(filter.Id, filter.Mask, filter.Extended != 0);
}

/// <summary>One node's place on a CAN bus.</summary>
/// <remarks>
/// <para>
/// <see cref="Open"/> opens a kernel interface such as <c>can0</c> through SocketCAN on a Linux
/// board, once it is up with <c>ip link set can0 up type can bitrate 250000</c>.
/// <see cref="Simulated"/> makes a bus inside the program, and <see cref="Join"/> puts another
/// node on the same bus. A node hears every frame the others send and none of its own, and keeps
/// only the frames its filters pass.
/// </para>
/// <para>
/// A receive on a simulated bus with nothing waiting returns <c>null</c> at once and counts its
/// timeout in <see cref="WaitedMicros"/>. A failure throws <see cref="PamojaException"/> with
/// the reason.
/// </para>
/// </remarks>
public sealed class CanBus : IDisposable
{
    private readonly NativeHandle _handle;

    private CanBus(IntPtr bus) =>
        _handle = NativeHandle.Create(bus, NativeMethods.pamoja_can_bus_free, "CAN bus");

    /// <summary>What the bus is.</summary>
    public CanBusKind Kind => (CanBusKind)_handle.Use(NativeMethods.pamoja_can_bus_kind);

    /// <summary>How many frames the node has sent.</summary>
    public long Sent => (long)_handle.Use(NativeMethods.pamoja_can_bus_sent);

    /// <summary>How many frames the node has received.</summary>
    public long Received => (long)_handle.Use(NativeMethods.pamoja_can_bus_received);

    /// <summary>
    /// How long receives on the node have waited without a frame, in microseconds, whether or not
    /// the process slept through it.
    /// </summary>
    public ulong WaitedMicros => _handle.Use(NativeMethods.pamoja_can_bus_waited_micros);

    /// <summary>Opens a kernel CAN interface through SocketCAN, as one node on its bus.</summary>
    /// <param name="interfaceName"><c>can0</c> for the first controller, <c>vcan0</c> for a virtual one.</param>
    /// <returns>The node.</returns>
    /// <exception cref="PlatformNotSupportedException">The platform is not Linux.</exception>
    /// <exception cref="PamojaException">The interface does not exist or cannot be bound.</exception>
    public static CanBus Open(string interfaceName)
    {
        ArgumentNullException.ThrowIfNull(interfaceName);
        PamojaStatus status = NativeMethods.pamoja_can_bus_open(interfaceName, out IntPtr bus);
        if (status == PamojaStatus.Unsupported)
        {
            throw new PlatformNotSupportedException(
                Status.LastError() ?? "a CAN interface is opened only on Linux");
        }

        Status.ThrowIfError(status);
        return new CanBus(bus);
    }

    /// <summary>A new bus inside the program, with this node the first on it.</summary>
    /// <returns>The node.</returns>
    public static CanBus Simulated() => new(NativeMethods.pamoja_can_bus_simulated());

    /// <summary>Puts another node on the same bus.</summary>
    /// <returns>The new node.</returns>
    /// <exception cref="PamojaException">The kernel refuses another socket on the interface.</exception>
    public CanBus Join()
    {
        IntPtr joined = IntPtr.Zero;
        Status.ThrowIfError(_handle.Use(bus => NativeMethods.pamoja_can_bus_join(bus, out joined)));
        return new CanBus(joined);
    }

    /// <summary>Sends a frame to every other node on the bus.</summary>
    /// <param name="frame">The frame.</param>
    /// <exception cref="PamojaException">The kernel refuses the frame.</exception>
    public void Send(CanFrame frame)
    {
        ArgumentNullException.ThrowIfNull(frame);
        IntPtr native = Can.ToNative(frame);
        try
        {
            Status.ThrowIfError(_handle.Use(bus => NativeMethods.pamoja_can_bus_send(bus, native)));
        }
        finally
        {
            NativeMethods.pamoja_can_frame_free(native);
        }
    }

    /// <summary>Takes the next frame the node keeps, waiting up to a timeout for one.</summary>
    /// <param name="timeout">How long to wait.</param>
    /// <returns>The frame, or <c>null</c> when the timeout passed with nothing.</returns>
    /// <exception cref="PamojaException">The interface fails.</exception>
    public CanFrame? Receive(TimeSpan timeout)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(timeout, TimeSpan.Zero);
        ulong micros = (ulong)(timeout.Ticks / TimeSpan.TicksPerMicrosecond);
        IntPtr frame = IntPtr.Zero;
        Status.ThrowIfError(_handle.Use(bus => NativeMethods.pamoja_can_bus_receive(bus, micros, out frame)));
        return frame == IntPtr.Zero ? null : Can.Describe(frame);
    }

    /// <summary>
    /// Keeps only the frames that pass at least one of the filters, from now on; an empty list
    /// keeps nothing.
    /// </summary>
    /// <param name="filters">The filters, at most 512.</param>
    /// <exception cref="PamojaException">The kernel refuses the filters.</exception>
    public void SetFilters(params CanFilter[] filters)
    {
        ArgumentNullException.ThrowIfNull(filters);
        PamojaCanFilter[] native = [.. filters.Select(filter => filter.ToNative())];
        Status.ThrowIfError(_handle.Use(bus =>
            NativeMethods.pamoja_can_bus_set_filters(bus, native, (nuint)native.Length)));
    }

    /// <summary>Keeps every frame again, as a node does when it joins.</summary>
    /// <exception cref="PamojaException">The kernel refuses the change.</exception>
    public void ClearFilters() =>
        Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_can_bus_clear_filters));

    /// <summary>Releases the node; it leaves the bus.</summary>
    public void Dispose() => _handle.Dispose();
}
