using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Modbus;

/// <summary>The reason a device gives for refusing a request, as it appears on the wire.</summary>
public enum ModbusExceptionCode : byte
{
    /// <summary>The function code is not allowed for this device.</summary>
    IllegalFunction = 0x01,

    /// <summary>The data address is not allowed for this device.</summary>
    IllegalDataAddress = 0x02,

    /// <summary>A value in the request is not allowed for this device.</summary>
    IllegalDataValue = 0x03,

    /// <summary>The device failed while serving the request.</summary>
    ServerDeviceFailure = 0x04,

    /// <summary>The device accepted a long-running request and is still processing it.</summary>
    Acknowledge = 0x05,

    /// <summary>The device is busy with a long-running request; retry later.</summary>
    ServerDeviceBusy = 0x06,

    /// <summary>The device detected a parity error in its memory.</summary>
    MemoryParityError = 0x08,

    /// <summary>A gateway could not route the request to the target path.</summary>
    GatewayPathUnavailable = 0x0A,

    /// <summary>A gateway got no response from the target device, usually one not on the network.</summary>
    GatewayTargetFailedToRespond = 0x0B,
}

/// <summary>Why a Modbus transaction failed.</summary>
public enum ModbusClientErrorKind : byte
{
    /// <summary>The request could not be built: a quantity outside what one request carries.</summary>
    Request = NativeMethods.ModbusClientRequest,

    /// <summary>A read was addressed to the broadcast address, which no device answers.</summary>
    BroadcastRead = NativeMethods.ModbusClientBroadcastRead,

    /// <summary>The serial port failed.</summary>
    Port = NativeMethods.ModbusClientPort,

    /// <summary>No complete reply arrived within the response timeout.</summary>
    Timeout = NativeMethods.ModbusClientTimeout,

    /// <summary>The reply failed its CRC or is not the shape its function gives.</summary>
    Frame = NativeMethods.ModbusClientFrame,

    /// <summary>A reply came back from a unit other than the one asked.</summary>
    WrongUnit = NativeMethods.ModbusClientWrongUnit,

    /// <summary>A reply answered a function other than the one asked.</summary>
    WrongFunction = NativeMethods.ModbusClientWrongFunction,

    /// <summary>A well-formed reply that does not answer the request.</summary>
    Mismatch = NativeMethods.ModbusClientMismatch,

    /// <summary>The device refused the request with an exception.</summary>
    Exception = NativeMethods.ModbusClientException,
}

/// <summary>Thrown when a Modbus transaction fails.</summary>
public sealed class ModbusClientException : PamojaException
{
    /// <summary>Creates the exception from what the native client reported.</summary>
    /// <param name="message">The reason, in words.</param>
    /// <param name="error">The reason, as the C ABI describes it.</param>
    internal ModbusClientException(string message, PamojaModbusClientError error)
        : base(message)
    {
        Kind = (ModbusClientErrorKind)error.Kind;
        Unit = error.Unit;
        FunctionCode = Kind is ModbusClientErrorKind.Exception or ModbusClientErrorKind.WrongFunction
            ? error.Function
            : null;
        Found = Kind is ModbusClientErrorKind.WrongUnit or ModbusClientErrorKind.WrongFunction
            ? error.Found
            : null;
        ExceptionCode = Kind == ModbusClientErrorKind.Exception ? (ModbusExceptionCode)error.Exception : null;
        Received = Kind == ModbusClientErrorKind.Timeout ? (int)error.Received : null;
    }

    /// <summary>Why the transaction failed.</summary>
    public ModbusClientErrorKind Kind { get; }

    /// <summary>The unit the transaction asked.</summary>
    public byte Unit { get; }

    /// <summary>
    /// For <see cref="ModbusClientErrorKind.Exception"/> and
    /// <see cref="ModbusClientErrorKind.WrongFunction"/>, the function asked.
    /// </summary>
    public byte? FunctionCode { get; }

    /// <summary>
    /// For <see cref="ModbusClientErrorKind.WrongUnit"/>, the unit that answered, and for
    /// <see cref="ModbusClientErrorKind.WrongFunction"/>, the function the reply named.
    /// </summary>
    public byte? Found { get; }

    /// <summary>For <see cref="ModbusClientErrorKind.Exception"/>, why the device refused.</summary>
    public ModbusExceptionCode? ExceptionCode { get; }

    /// <summary>For <see cref="ModbusClientErrorKind.Timeout"/>, how many bytes of a reply had arrived.</summary>
    public int? Received { get; }
}

/// <summary>
/// A Modbus RTU client on a serial line: the gateway, the master in the specification's words,
/// that sends each request and waits for its reply.
/// </summary>
/// <remarks>
/// <para>
/// Each transaction follows the Modbus over Serial Line specification. The client leaves the
/// line silent for 3.5 characters, or 1.75 ms above 19200 baud, so the request starts a new
/// frame; drops anything stale waiting in the port; writes the request; and reads the reply to
/// the length the request implies, against <see cref="ResponseTimeout"/>. The reply's CRC,
/// unit, and function are checked before a value is read out of it, and a refusal throws
/// <see cref="ModbusClientException"/> with the device's <see cref="ModbusExceptionCode"/>. A
/// write to <see cref="Broadcast"/> reaches every device and draws no reply, so the client
/// waits out <see cref="Turnaround"/> instead.
/// </para>
/// <para>
/// On the kernel's serial device every wait is real, so each call blocks until the reply is in.
/// On a simulated line nothing waits: the waits are counted in the port's
/// <see cref="SerialPort.WaitedMicros"/>.
/// </para>
/// </remarks>
/// <example>
/// <code>
/// using var client = new ModbusClient(SerialPort.Open("/dev/ttyUSB0", new SerialSettings(19_200, Parity.Even)));
/// ushort[] registers = client.ReadHoldingRegisters(17, 107, 3);
/// </code>
/// </example>
public sealed class ModbusClient : IDisposable
{
    /// <summary>The unit address every device acts on and none answers.</summary>
    public const byte Broadcast = 0;

    private readonly NativeHandle _handle;

    /// <summary>Makes a client on a port, with a one-second response timeout and a 100 ms turnaround.</summary>
    /// <param name="port">The line; the client holds its own share of it, so the port may be disposed.</param>
    public ModbusClient(SerialPort port)
    {
        ArgumentNullException.ThrowIfNull(port);
        IntPtr client = IntPtr.Zero;
        Status.ThrowIfError(port.Use(native => NativeMethods.pamoja_modbus_client_new(native, out client)));
        _handle = NativeHandle.Create(client, NativeMethods.pamoja_modbus_client_free, "Modbus client");
    }

    /// <summary>How long the client waits for a whole reply once a request has gone out.</summary>
    public TimeSpan ResponseTimeout
    {
        get => TimeSpan.FromTicks((long)_handle.Use(NativeMethods.pamoja_modbus_client_response_timeout_micros) * TimeSpan.TicksPerMicrosecond);
        set
        {
            ulong micros = Micros(value);
            _handle.Use(client => NativeMethods.pamoja_modbus_client_set_response_timeout(client, micros));
        }
    }

    /// <summary>How long the client leaves the line quiet after a broadcast.</summary>
    public TimeSpan Turnaround
    {
        get => TimeSpan.FromTicks((long)_handle.Use(NativeMethods.pamoja_modbus_client_turnaround_micros) * TimeSpan.TicksPerMicrosecond);
        set
        {
            ulong micros = Micros(value);
            _handle.Use(client => NativeMethods.pamoja_modbus_client_set_turnaround(client, micros));
        }
    }

    /// <summary>
    /// Returns the silence that separates two frames: 3.5 characters at the line's speed and
    /// format, and a fixed 1750 microseconds above 19200 baud.
    /// </summary>
    /// <param name="settings">The line's speed and character format.</param>
    /// <returns>The silence, in nanoseconds, rounded up.</returns>
    /// <exception cref="PamojaException">The settings are not ones a port has.</exception>
    public static ulong FrameGapNanos(SerialSettings settings)
    {
        Status.ThrowIfError(NativeMethods.pamoja_modbus_frame_gap_nanos(settings.ToNative(), out ulong nanos));
        return nanos;
    }

    /// <summary>Reads coils, function 0x01.</summary>
    /// <param name="unit">The device, 1 to 247.</param>
    /// <param name="start">The first coil's address.</param>
    /// <param name="quantity">How many, 1 to 2000.</param>
    /// <returns>The coils' states, in address order.</returns>
    /// <exception cref="ModbusClientException">The transaction failed; its kind says why.</exception>
    public bool[] ReadCoils(byte unit, ushort start, ushort quantity) =>
        ReadBits(quantity, (client, values) => Call(unit, (out PamojaModbusClientError error) =>
            NativeMethods.pamoja_modbus_client_read_coils(client, unit, start, quantity, values, out error)));

    /// <summary>Reads discrete inputs, function 0x02.</summary>
    /// <param name="unit">The device, 1 to 247.</param>
    /// <param name="start">The first input's address.</param>
    /// <param name="quantity">How many, 1 to 2000.</param>
    /// <returns>The inputs' states, in address order.</returns>
    /// <exception cref="ModbusClientException">The transaction failed; its kind says why.</exception>
    public bool[] ReadDiscreteInputs(byte unit, ushort start, ushort quantity) =>
        ReadBits(quantity, (client, values) => Call(unit, (out PamojaModbusClientError error) =>
            NativeMethods.pamoja_modbus_client_read_discrete_inputs(client, unit, start, quantity, values, out error)));

    /// <summary>Reads holding registers, function 0x03.</summary>
    /// <param name="unit">The device, 1 to 247.</param>
    /// <param name="start">The first register's address.</param>
    /// <param name="quantity">How many, 1 to 125.</param>
    /// <returns>The registers' values, in address order.</returns>
    /// <exception cref="ModbusClientException">The transaction failed; its kind says why.</exception>
    public ushort[] ReadHoldingRegisters(byte unit, ushort start, ushort quantity)
    {
        ushort[] values = new ushort[quantity];
        _handle.Use(client => Call(unit, (out PamojaModbusClientError error) =>
            NativeMethods.pamoja_modbus_client_read_holding_registers(client, unit, start, quantity, values, out error)));
        return values;
    }

    /// <summary>Reads input registers, function 0x04.</summary>
    /// <param name="unit">The device, 1 to 247.</param>
    /// <param name="start">The first register's address.</param>
    /// <param name="quantity">How many, 1 to 125.</param>
    /// <returns>The registers' values, in address order.</returns>
    /// <exception cref="ModbusClientException">The transaction failed; its kind says why.</exception>
    public ushort[] ReadInputRegisters(byte unit, ushort start, ushort quantity)
    {
        ushort[] values = new ushort[quantity];
        _handle.Use(client => Call(unit, (out PamojaModbusClientError error) =>
            NativeMethods.pamoja_modbus_client_read_input_registers(client, unit, start, quantity, values, out error)));
        return values;
    }

    /// <summary>Writes one coil, function 0x05.</summary>
    /// <param name="unit">The device, 1 to 247, or <see cref="Broadcast"/> for every device.</param>
    /// <param name="address">The coil's address.</param>
    /// <param name="on">The state to write.</param>
    /// <exception cref="ModbusClientException">The transaction failed; its kind says why.</exception>
    public void WriteSingleCoil(byte unit, ushort address, bool on) =>
        _handle.Use(client => Call(unit, (out PamojaModbusClientError error) =>
            NativeMethods.pamoja_modbus_client_write_single_coil(client, unit, address, on, out error)));

    /// <summary>Writes one holding register, function 0x06.</summary>
    /// <param name="unit">The device, 1 to 247, or <see cref="Broadcast"/> for every device.</param>
    /// <param name="address">The register's address.</param>
    /// <param name="value">The value to write.</param>
    /// <exception cref="ModbusClientException">The transaction failed; its kind says why.</exception>
    public void WriteSingleRegister(byte unit, ushort address, ushort value) =>
        _handle.Use(client => Call(unit, (out PamojaModbusClientError error) =>
            NativeMethods.pamoja_modbus_client_write_single_register(client, unit, address, value, out error)));

    /// <summary>Writes a run of coils, function 0x0F.</summary>
    /// <param name="unit">The device, 1 to 247, or <see cref="Broadcast"/> for every device.</param>
    /// <param name="start">The first coil's address.</param>
    /// <param name="values">The states to write, 1 to 1968 of them, in address order.</param>
    /// <exception cref="ModbusClientException">The transaction failed; its kind says why.</exception>
    public void WriteMultipleCoils(byte unit, ushort start, ReadOnlySpan<bool> values)
    {
        byte[] packed = new byte[values.Length];
        for (int index = 0; index < values.Length; index++)
        {
            packed[index] = values[index] ? (byte)1 : (byte)0;
        }

        _handle.Use(client => Call(unit, (out PamojaModbusClientError error) =>
            NativeMethods.pamoja_modbus_client_write_multiple_coils(
                client, unit, start, packed, (nuint)packed.Length, out error)));
    }

    /// <summary>Writes a run of holding registers, function 0x10.</summary>
    /// <param name="unit">The device, 1 to 247, or <see cref="Broadcast"/> for every device.</param>
    /// <param name="start">The first register's address.</param>
    /// <param name="values">The values to write, 1 to 123 of them, in address order.</param>
    /// <exception cref="ModbusClientException">The transaction failed; its kind says why.</exception>
    public void WriteMultipleRegisters(byte unit, ushort start, ReadOnlySpan<ushort> values)
    {
        ushort[] copy = values.ToArray();
        _handle.Use(client => Call(unit, (out PamojaModbusClientError error) =>
            NativeMethods.pamoja_modbus_client_write_multiple_registers(
                client, unit, start, copy, (nuint)copy.Length, out error)));
    }

    /// <summary>Releases the client. Its port stays open while another holder has it.</summary>
    public void Dispose() => _handle.Dispose();

    private delegate PamojaStatus Transaction(out PamojaModbusClientError error);

    private static void Call(byte unit, Transaction transaction)
    {
        PamojaStatus status = transaction(out PamojaModbusClientError error);
        if (status == PamojaStatus.Ok)
        {
            return;
        }

        string message = Status.LastError() ?? $"the transaction with unit {unit} failed";
        if (error.Kind != NativeMethods.ModbusClientOk)
        {
            throw new ModbusClientException(message, error);
        }

        throw new PamojaException(message);
    }

    private bool[] ReadBits(ushort quantity, Action<IntPtr, byte[]> read)
    {
        byte[] packed = new byte[quantity];
        _handle.Use(client => read(client, packed));
        bool[] bits = new bool[quantity];
        for (int index = 0; index < quantity; index++)
        {
            bits[index] = packed[index] != 0;
        }

        return bits;
    }

    private static ulong Micros(TimeSpan duration)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(duration, TimeSpan.Zero);
        return (ulong)(duration.Ticks / TimeSpan.TicksPerMicrosecond);
    }
}
