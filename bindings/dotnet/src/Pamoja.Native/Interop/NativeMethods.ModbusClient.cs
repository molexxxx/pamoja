using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for a Modbus client on a serial port and the devices it polls,
/// mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must be
/// updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>A client error kind: the call succeeded.</summary>
    public const byte ModbusClientOk = 0;

    /// <summary>A client error kind: the request could not be built.</summary>
    public const byte ModbusClientRequest = 1;

    /// <summary>A client error kind: a read was addressed to the broadcast address.</summary>
    public const byte ModbusClientBroadcastRead = 2;

    /// <summary>A client error kind: the serial port failed.</summary>
    public const byte ModbusClientPort = 3;

    /// <summary>A client error kind: no complete reply arrived within the response timeout.</summary>
    public const byte ModbusClientTimeout = 4;

    /// <summary>A client error kind: the reply failed its CRC or is not the shape its function gives.</summary>
    public const byte ModbusClientFrame = 5;

    /// <summary>A client error kind: a reply came back from another unit.</summary>
    public const byte ModbusClientWrongUnit = 6;

    /// <summary>A client error kind: a reply answered another function.</summary>
    public const byte ModbusClientWrongFunction = 7;

    /// <summary>A client error kind: a well-formed reply that does not answer the request.</summary>
    public const byte ModbusClientMismatch = 8;

    /// <summary>A client error kind: the device refused the request with an exception.</summary>
    public const byte ModbusClientException = 9;

    /// <summary>Makes a device at a unit address, with every table empty.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_server_new(byte unit, out IntPtr outServer);

    /// <summary>Sets coils from an address on, one byte per coil.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_server_set_coils(
        IntPtr server,
        ushort start,
        ReadOnlySpan<byte> values,
        nuint len);

    /// <summary>Sets discrete inputs from an address on, one byte per input.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_server_set_discrete_inputs(
        IntPtr server,
        ushort start,
        ReadOnlySpan<byte> values,
        nuint len);

    /// <summary>Sets holding registers from an address on.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_server_set_holding_registers(
        IntPtr server,
        ushort start,
        ReadOnlySpan<ushort> values,
        nuint len);

    /// <summary>Sets input registers from an address on.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_server_set_input_registers(
        IntPtr server,
        ushort start,
        ReadOnlySpan<ushort> values,
        nuint len);

    /// <summary>Reads a coil, reporting whether the device has it.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_modbus_server_coil(
        IntPtr server,
        ushort address,
        [MarshalAs(UnmanagedType.U1)] out bool outOn);

    /// <summary>Reads a discrete input, reporting whether the device has it.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_modbus_server_discrete_input(
        IntPtr server,
        ushort address,
        [MarshalAs(UnmanagedType.U1)] out bool outOn);

    /// <summary>Reads a holding register, reporting whether the device has it.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_modbus_server_holding_register(
        IntPtr server,
        ushort address,
        out ushort outValue);

    /// <summary>Reads an input register, reporting whether the device has it.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_modbus_server_input_register(
        IntPtr server,
        ushort address,
        out ushort outValue);

    /// <summary>Returns a device's unit address.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_modbus_server_unit(IntPtr server);

    /// <summary>Returns how many requests a device has carried out.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_modbus_server_served(IntPtr server);

    /// <summary>Answers one RTU frame, leaving the buffer null when the device stays silent.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_server_answer(
        IntPtr server,
        ReadOnlySpan<byte> frame,
        nuint len,
        out IntPtr outBuffer);

    /// <summary>Releases the caller's handle to a device. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_modbus_server_free(IntPtr server);

    /// <summary>Makes a line with no devices on it.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_modbus_line_new();

    /// <summary>Puts a device on a line, which shares it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_line_attach(IntPtr line, IntPtr server);

    /// <summary>Returns how many devices are on a line.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_modbus_line_len(IntPtr line);

    /// <summary>Makes a serial port with a line on its far end.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_line_port(
        IntPtr line,
        PamojaSerialSettings settings,
        out IntPtr outPort);

    /// <summary>Releases the caller's handle to a line. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_modbus_line_free(IntPtr line);

    /// <summary>Returns the silence that separates two frames, in nanoseconds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_frame_gap_nanos(
        PamojaSerialSettings settings,
        out ulong outNanos);

    /// <summary>Makes a client on a port.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_new(IntPtr port, out IntPtr outClient);

    /// <summary>Sets how long a client waits for a whole reply, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_modbus_client_set_response_timeout(IntPtr client, ulong micros);

    /// <summary>Sets how long a client leaves the line quiet after a broadcast, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_modbus_client_set_turnaround(IntPtr client, ulong micros);

    /// <summary>Returns a client's response timeout, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_modbus_client_response_timeout_micros(IntPtr client);

    /// <summary>Returns a client's turnaround delay, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_modbus_client_turnaround_micros(IntPtr client);

    /// <summary>Reads coils, one byte per coil.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_read_coils(
        IntPtr client,
        byte unit,
        ushort start,
        ushort quantity,
        Span<byte> outValues,
        out PamojaModbusClientError outError);

    /// <summary>Reads discrete inputs, one byte per input.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_read_discrete_inputs(
        IntPtr client,
        byte unit,
        ushort start,
        ushort quantity,
        Span<byte> outValues,
        out PamojaModbusClientError outError);

    /// <summary>Reads holding registers.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_read_holding_registers(
        IntPtr client,
        byte unit,
        ushort start,
        ushort quantity,
        Span<ushort> outValues,
        out PamojaModbusClientError outError);

    /// <summary>Reads input registers.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_read_input_registers(
        IntPtr client,
        byte unit,
        ushort start,
        ushort quantity,
        Span<ushort> outValues,
        out PamojaModbusClientError outError);

    /// <summary>Writes one coil.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_write_single_coil(
        IntPtr client,
        byte unit,
        ushort address,
        [MarshalAs(UnmanagedType.U1)] bool on,
        out PamojaModbusClientError outError);

    /// <summary>Writes one holding register.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_write_single_register(
        IntPtr client,
        byte unit,
        ushort address,
        ushort value,
        out PamojaModbusClientError outError);

    /// <summary>Writes a run of coils, one byte per coil.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_write_multiple_coils(
        IntPtr client,
        byte unit,
        ushort start,
        ReadOnlySpan<byte> values,
        nuint len,
        out PamojaModbusClientError outError);

    /// <summary>Writes a run of holding registers.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_client_write_multiple_registers(
        IntPtr client,
        byte unit,
        ushort start,
        ReadOnlySpan<ushort> values,
        nuint len,
        out PamojaModbusClientError outError);

    /// <summary>Releases a client. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_modbus_client_free(IntPtr client);
}
