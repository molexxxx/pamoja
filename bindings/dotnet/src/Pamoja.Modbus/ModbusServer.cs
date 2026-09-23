using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Modbus;

/// <summary>
/// A Modbus device: a unit address and the four tables it serves, coils, discrete inputs,
/// holding registers, and input registers, each holding only the addresses it was given.
/// </summary>
/// <remarks>
/// <para>
/// <see cref="Answer"/> takes one RTU frame and returns the frame the device sends back,
/// following the Modbus over Serial Line specification: a frame that fails its CRC, is for
/// another unit, or is a broadcast gets no answer, and a broadcast write is still carried out.
/// A request the device cannot serve is answered with the exception the application
/// protocol gives: an unknown function, then a quantity or value out of range, then an
/// address the device does not have.
/// </para>
/// <para>
/// Put a device on a <see cref="ModbusLine"/> to poll it with a <see cref="ModbusClient"/>.
/// The line shares the device, so this object still reads and changes its tables.
/// </para>
/// </remarks>
/// <example>
/// <code>
/// using var meter = new ModbusServer(17);
/// meter.SetHoldingRegisters(107, [2301, 418, 0]);
/// </code>
/// </example>
public sealed class ModbusServer : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Makes a device at a unit address, with every table empty.</summary>
    /// <param name="unit">Its address, 1 to 247.</param>
    /// <exception cref="PamojaException">
    /// The unit is 0, the broadcast address, or 248 to 255, which the specification reserves.
    /// </exception>
    public ModbusServer(byte unit)
    {
        Status.ThrowIfError(NativeMethods.pamoja_modbus_server_new(unit, out IntPtr server));
        _handle = NativeHandle.Create(server, NativeMethods.pamoja_modbus_server_free, "Modbus server");
    }

    /// <summary>The device's unit address.</summary>
    public byte Unit => _handle.Use(NativeMethods.pamoja_modbus_server_unit);

    /// <summary>How many requests the device has carried out, broadcasts included and refusals not.</summary>
    public int Served => (int)_handle.Use(NativeMethods.pamoja_modbus_server_served);

    /// <summary>Sets coils from an address on, adding any the device did not have.</summary>
    /// <param name="start">The first coil's address.</param>
    /// <param name="values">Their states, in address order.</param>
    public void SetCoils(ushort start, ReadOnlySpan<bool> values)
    {
        byte[] packed = Pack(values);
        Status.ThrowIfError(_handle.Use(server =>
            NativeMethods.pamoja_modbus_server_set_coils(server, start, packed, (nuint)packed.Length)));
    }

    /// <summary>Sets discrete inputs from an address on, adding any the device did not have.</summary>
    /// <param name="start">The first input's address.</param>
    /// <param name="values">Their states, in address order.</param>
    public void SetDiscreteInputs(ushort start, ReadOnlySpan<bool> values)
    {
        byte[] packed = Pack(values);
        Status.ThrowIfError(_handle.Use(server =>
            NativeMethods.pamoja_modbus_server_set_discrete_inputs(server, start, packed, (nuint)packed.Length)));
    }

    /// <summary>Sets holding registers from an address on, adding any the device did not have.</summary>
    /// <param name="start">The first register's address.</param>
    /// <param name="values">Their values, in address order.</param>
    public void SetHoldingRegisters(ushort start, ReadOnlySpan<ushort> values)
    {
        ushort[] copy = values.ToArray();
        Status.ThrowIfError(_handle.Use(server =>
            NativeMethods.pamoja_modbus_server_set_holding_registers(server, start, copy, (nuint)copy.Length)));
    }

    /// <summary>Sets input registers from an address on, adding any the device did not have.</summary>
    /// <param name="start">The first register's address.</param>
    /// <param name="values">Their values, in address order.</param>
    public void SetInputRegisters(ushort start, ReadOnlySpan<ushort> values)
    {
        ushort[] copy = values.ToArray();
        Status.ThrowIfError(_handle.Use(server =>
            NativeMethods.pamoja_modbus_server_set_input_registers(server, start, copy, (nuint)copy.Length)));
    }

    /// <summary>Reads a coil.</summary>
    /// <param name="address">The coil's address.</param>
    /// <returns>Its state, or null when the device has no coil there.</returns>
    public bool? Coil(ushort address)
    {
        bool on = false;
        bool held = _handle.Use(server => NativeMethods.pamoja_modbus_server_coil(server, address, out on));
        return held ? on : null;
    }

    /// <summary>Reads a discrete input.</summary>
    /// <param name="address">The input's address.</param>
    /// <returns>Its state, or null when the device has no input there.</returns>
    public bool? DiscreteInput(ushort address)
    {
        bool on = false;
        bool held = _handle.Use(server => NativeMethods.pamoja_modbus_server_discrete_input(server, address, out on));
        return held ? on : null;
    }

    /// <summary>Reads a holding register.</summary>
    /// <param name="address">The register's address.</param>
    /// <returns>Its value, or null when the device has no register there.</returns>
    public ushort? HoldingRegister(ushort address)
    {
        ushort value = 0;
        bool held = _handle.Use(server => NativeMethods.pamoja_modbus_server_holding_register(server, address, out value));
        return held ? value : null;
    }

    /// <summary>Reads an input register.</summary>
    /// <param name="address">The register's address.</param>
    /// <returns>Its value, or null when the device has no register there.</returns>
    public ushort? InputRegister(ushort address)
    {
        ushort value = 0;
        bool held = _handle.Use(server => NativeMethods.pamoja_modbus_server_input_register(server, address, out value));
        return held ? value : null;
    }

    /// <summary>Answers one RTU frame, as the device on the line does.</summary>
    /// <param name="frame">The frame as it came off the line, CRC included.</param>
    /// <returns>
    /// The frame to send back, or null when the device stays silent: the frame failed its CRC,
    /// is for another unit, or is a broadcast, whose write the device still carries out.
    /// </returns>
    public byte[]? Answer(ReadOnlySpan<byte> frame)
    {
        byte[] copy = frame.ToArray();
        IntPtr buffer = IntPtr.Zero;
        Status.ThrowIfError(_handle.Use(server =>
            NativeMethods.pamoja_modbus_server_answer(server, copy, (nuint)copy.Length, out buffer)));
        return buffer == IntPtr.Zero ? null : Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Runs a native call that needs this device's handle.</summary>
    /// <typeparam name="TResult">What the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    internal TResult Use<TResult>(Func<IntPtr, TResult> call) => _handle.Use(call);

    /// <summary>Releases this handle to the device. A line it is on keeps its own share.</summary>
    public void Dispose() => _handle.Dispose();

    private static byte[] Pack(ReadOnlySpan<bool> values)
    {
        byte[] packed = new byte[values.Length];
        for (int index = 0; index < values.Length; index++)
        {
            packed[index] = values[index] ? (byte)1 : (byte)0;
        }

        return packed;
    }
}

/// <summary>
/// Several devices on one simulated line, as devices share an RS485 pair: every frame reaches
/// all of them, the one it is addressed to answers, and each carries out a broadcast write.
/// </summary>
/// <remarks>
/// <see cref="Port"/> makes a serial port with the line on its far end, which a
/// <see cref="ModbusClient"/> polls exactly as it would the kernel's serial device. Nothing on
/// such a port waits: the silences and the timeouts a real line takes are counted in the
/// port's <see cref="SerialPort.WaitedMicros"/>.
/// </remarks>
/// <example>
/// <code>
/// using var line = new ModbusLine();
/// line.Attach(meter);
/// using SerialPort port = line.Port(new SerialSettings(19_200, Parity.Even));
/// </code>
/// </example>
public sealed class ModbusLine : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Makes a line with no devices on it.</summary>
    public ModbusLine()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_modbus_line_new(), NativeMethods.pamoja_modbus_line_free, "Modbus line");
    }

    /// <summary>How many devices are on the line.</summary>
    public int Count => (int)_handle.Use(NativeMethods.pamoja_modbus_line_len);

    /// <summary>Puts a device on the line. The line shares it, so the device still reads and changes.</summary>
    /// <param name="server">The device.</param>
    /// <returns>This line, to put another device on.</returns>
    public ModbusLine Attach(ModbusServer server)
    {
        ArgumentNullException.ThrowIfNull(server);
        Status.ThrowIfError(_handle.Use(line =>
            server.Use(device => NativeMethods.pamoja_modbus_line_attach(line, device))));
        return this;
    }

    /// <summary>
    /// Makes a serial port with the line on its far end: every frame written reaches each device,
    /// and whatever they answer waits to be read. Devices put on the line later are on the port too.
    /// </summary>
    /// <param name="settings">The speed and character format the line runs at.</param>
    /// <returns>The port.</returns>
    /// <exception cref="PamojaException">The settings are not ones a port has.</exception>
    public SerialPort Port(SerialSettings settings)
    {
        PamojaSerialSettings native = settings.ToNative();
        IntPtr port = IntPtr.Zero;
        Status.ThrowIfError(_handle.Use(line => NativeMethods.pamoja_modbus_line_port(line, native, out port)));
        return SerialPort.FromHandle(port);
    }

    /// <summary>Releases this handle to the line. A port made from it keeps its own share.</summary>
    public void Dispose() => _handle.Dispose();
}
