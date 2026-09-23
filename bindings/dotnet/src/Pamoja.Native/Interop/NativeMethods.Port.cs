using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for one serial port of the pamoja C ABI - the kernel's serial
/// device, a looped line, a null-modem pair, or a script - mirroring <c>pamoja.h</c>
/// one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must be
/// updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>No parity bit.</summary>
    public const byte ParityNone = 0;

    /// <summary>A parity bit that makes the count of ones even.</summary>
    public const byte ParityEven = 1;

    /// <summary>A parity bit that makes the count of ones odd.</summary>
    public const byte ParityOdd = 2;

    /// <summary>A port kind: the kernel's serial device.</summary>
    public const byte SerialPortDevice = 0;

    /// <summary>A port kind: the port's own output, looped back to its input.</summary>
    public const byte SerialPortLooped = 1;

    /// <summary>A port kind: the other end of a null-modem pair.</summary>
    public const byte SerialPortPaired = 2;

    /// <summary>A port kind: a simulated device that answers each write.</summary>
    public const byte SerialPortSimulated = 3;

    /// <summary>A port kind: a script of the writes a driver is expected to make.</summary>
    public const byte SerialPortScripted = 4;

    /// <summary>Returns eight data bits, no parity, and one stop bit at a speed.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSerialSettings pamoja_serial_settings(uint baud);

    /// <summary>Returns the bits one character takes on the wire.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_serial_settings_bits_per_character(PamojaSerialSettings settings);

    /// <summary>Returns how long one character takes on the wire, in nanoseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_serial_settings_character_nanos(PamojaSerialSettings settings);

    /// <summary>Opens the kernel's serial device raw.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_serial_port_open(
        string path,
        PamojaSerialSettings settings,
        out IntPtr outPort);

    /// <summary>Creates a line looped back on itself.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_serial_port_looped(PamojaSerialSettings settings);

    /// <summary>Creates the two ends of a null-modem pair.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_port_pair(
        PamojaSerialSettings settings,
        out IntPtr outOne,
        out IntPtr outOther);

    /// <summary>Creates an empty script.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_serial_script_new();

    /// <summary>Adds a write the program is expected to make.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_script_write(
        IntPtr script,
        ReadOnlySpan<byte> bytes,
        nuint len);

    /// <summary>Adds bytes the far end sends.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_script_read(
        IntPtr script,
        ReadOnlySpan<byte> bytes,
        nuint len);

    /// <summary>Returns how many steps a script holds.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_serial_script_len(IntPtr script);

    /// <summary>Releases a script. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_serial_script_free(IntPtr script);

    /// <summary>Creates a port that plays a copy of a script.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_serial_port_scripted(PamojaSerialSettings settings, IntPtr script);

    /// <summary>Gives another holder the same port.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_serial_port_clone(IntPtr port);

    /// <summary>Returns what is on the other end of a port.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_serial_port_kind(IntPtr port);

    /// <summary>Returns the speed and format a port runs at.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSerialSettings pamoja_serial_port_settings(IntPtr port);

    /// <summary>Writes bytes, waiting until they have left the UART on a real device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_port_write(
        IntPtr port,
        ReadOnlySpan<byte> bytes,
        nuint len);

    /// <summary>Reads what has arrived, waiting up to a timeout for the first byte.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_port_read(
        IntPtr port,
        Span<byte> outBytes,
        nuint capacity,
        ulong timeoutMicros,
        out nuint outLen);

    /// <summary>Drops whatever has arrived and not been read.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_port_discard_input(IntPtr port);

    /// <summary>Waits, really on a device and anywhere else only counted.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_serial_port_wait(IntPtr port, ulong micros);

    /// <summary>Returns how many bytes have been written through a port.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_serial_port_written(IntPtr port);

    /// <summary>Returns how many bytes have been read through a port.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_serial_port_received(IntPtr port);

    /// <summary>Returns how long a port's reads and waits have waited, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_serial_port_waited_micros(IntPtr port);

    /// <summary>Reports how many steps a scripted port has left.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_serial_port_remaining(IntPtr port, out nuint outRemaining);

    /// <summary>Releases a holder's share of a port. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_serial_port_free(IntPtr port);
}
