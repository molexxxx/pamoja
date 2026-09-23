using Pamoja.Native.Interop;

namespace Pamoja.Hal;

/// <summary>What answers on an <see cref="I2cBus"/>.</summary>
public enum I2cBusKind : byte
{
    /// <summary>The kernel's adapter, with real parts on real wires.</summary>
    Adapter = NativeMethods.I2cBusAdapter,

    /// <summary>Simulated parts, answering from their registers.</summary>
    Simulated = NativeMethods.I2cBusSimulated,

    /// <summary>A script of the transfers a driver is expected to make.</summary>
    Scripted = NativeMethods.I2cBusScripted,
}

/// <summary>How a scripted step fails the transfer that reaches it.</summary>
public enum I2cFault : byte
{
    /// <summary>Nothing acknowledged the address.</summary>
    NoAcknowledgeAddress = 0,

    /// <summary>The part did not acknowledge a data byte.</summary>
    NoAcknowledgeData = 1,

    /// <summary>A missing acknowledge, with no telling whether of the address or the data.</summary>
    NoAcknowledge = 2,

    /// <summary>A bus error, such as a misplaced start or stop condition.</summary>
    Bus = 3,

    /// <summary>Another controller won the bus.</summary>
    ArbitrationLoss = 4,

    /// <summary>Data arrived faster than it was taken.</summary>
    Overrun = 5,

    /// <summary>A failure of no more particular kind.</summary>
    Other = 6,
}

/// <summary>A part that is not there, answering from 256 registers.</summary>
/// <remarks>
/// A write names a register and fills it and the ones after it; a read takes them back from
/// wherever the last write left off. What a driver writes stays written, so
/// <see cref="Register"/> reads a part's configuration back once a driver is done with it.
/// </remarks>
public sealed class I2cPart : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a part answering at one address, with every register reading zero.</summary>
    /// <param name="address">The 7-bit address it answers to.</param>
    public I2cPart(byte address)
        : this(NativeHandle.Create(
            NativeMethods.pamoja_i2c_part_new(address),
            NativeMethods.pamoja_i2c_part_free,
            "I2C part"))
    {
    }

    /// <summary>Wraps a part a native call made, such as a simulated sensor from another package.</summary>
    /// <param name="handle">The part's handle, released with <c>pamoja_i2c_part_free</c>.</param>
    public I2cPart(NativeHandle handle)
    {
        ArgumentNullException.ThrowIfNull(handle);
        _handle = handle;
    }

    /// <summary>The address the part answers to.</summary>
    public byte Address => _handle.Use(NativeMethods.pamoja_i2c_part_address);

    /// <summary>How many transfers the part has served.</summary>
    public int Transfers => (int)_handle.Use(NativeMethods.pamoja_i2c_part_transfers);

    /// <summary>Puts bytes in the part from a register on, and returns the part.</summary>
    /// <param name="first">The register the bytes start at.</param>
    /// <param name="bytes">What to put there. Past the last register it wraps to the first.</param>
    /// <returns>This part, so calls chain.</returns>
    public I2cPart Holding(byte first, ReadOnlySpan<byte> bytes)
    {
        Load(first, bytes);
        return this;
    }

    /// <summary>Puts bytes in the part from a register on.</summary>
    /// <param name="first">The register the bytes start at.</param>
    /// <param name="bytes">What to put there. Past the last register it wraps to the first.</param>
    public void Load(byte first, ReadOnlySpan<byte> bytes)
    {
        byte[] copy = bytes.ToArray();
        Status.ThrowIfError(_handle.Use(part =>
            NativeMethods.pamoja_i2c_part_load(part, first, copy, (nuint)copy.Length)));
    }

    /// <summary>Reads what one register holds now.</summary>
    /// <param name="register">Which register.</param>
    /// <returns>Its value, which is what a driver wrote if it wrote one.</returns>
    public byte Register(byte register) =>
        _handle.Use(part => NativeMethods.pamoja_i2c_part_register(part, register));

    /// <summary>Reads consecutive registers from one register on.</summary>
    /// <param name="first">The first register.</param>
    /// <param name="length">How many registers.</param>
    /// <returns>One byte per register.</returns>
    public byte[] Read(byte first, int length)
    {
        byte[] bytes = new byte[length];
        Status.ThrowIfError(_handle.Use(part =>
            NativeMethods.pamoja_i2c_part_read(part, first, bytes, (nuint)bytes.Length)));
        return bytes;
    }

    /// <summary>Runs a native call that needs this part's handle.</summary>
    /// <typeparam name="TResult">What the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    public TResult Use<TResult>(Func<IntPtr, TResult> call) => _handle.Use(call);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>One transfer a script expects, and what the part answers.</summary>
public sealed class I2cStep
{
    private enum Shape
    {
        Write,
        Read,
        WriteRead,
        Fault,
    }

    private readonly Shape _shape;
    private readonly byte _address;
    private readonly byte[] _bytes;
    private readonly byte[] _reply;
    private readonly I2cFault _fault;

    private I2cStep(Shape shape, byte address, byte[] bytes, byte[] reply, I2cFault fault)
    {
        _shape = shape;
        _address = address;
        _bytes = bytes;
        _reply = reply;
        _fault = fault;
    }

    /// <summary>The driver writes exactly <paramref name="bytes"/> to the address.</summary>
    /// <param name="address">The 7-bit address the write must go to.</param>
    /// <param name="bytes">The bytes the driver must send.</param>
    /// <returns>The step.</returns>
    public static I2cStep Write(byte address, ReadOnlySpan<byte> bytes) =>
        new(Shape.Write, address, bytes.ToArray(), [], default);

    /// <summary>The driver reads from the address and receives <paramref name="reply"/>.</summary>
    /// <param name="address">The 7-bit address the read must come from.</param>
    /// <param name="reply">The bytes the part answers with; the driver must ask for exactly this many.</param>
    /// <returns>The step.</returns>
    public static I2cStep Read(byte address, ReadOnlySpan<byte> reply) =>
        new(Shape.Read, address, [], reply.ToArray(), default);

    /// <summary>
    /// The driver writes <paramref name="bytes"/> and then reads <paramref name="reply"/> in one
    /// transaction, the shape of a register read.
    /// </summary>
    /// <param name="address">The 7-bit address of the part.</param>
    /// <param name="bytes">The bytes the driver must send first, usually a register address.</param>
    /// <param name="reply">The bytes the part answers with.</param>
    /// <returns>The step.</returns>
    public static I2cStep WriteRead(byte address, ReadOnlySpan<byte> bytes, ReadOnlySpan<byte> reply) =>
        new(Shape.WriteRead, address, bytes.ToArray(), reply.ToArray(), default);

    /// <summary>The next transfer to the address fails, the way a missing or busy part does.</summary>
    /// <param name="address">The 7-bit address the failing transfer must go to.</param>
    /// <param name="fault">The failure the driver sees.</param>
    /// <returns>The step.</returns>
    public static I2cStep Fault(byte address, I2cFault fault) =>
        new(Shape.Fault, address, [], [], fault);

    /// <summary>Adds this step to a native script.</summary>
    /// <param name="script">The script's handle.</param>
    /// <returns>The status of the native call.</returns>
    internal PamojaStatus AddTo(IntPtr script) => _shape switch
    {
        Shape.Write => NativeMethods.pamoja_i2c_script_write(
            script, _address, _bytes, (nuint)_bytes.Length),
        Shape.Read => NativeMethods.pamoja_i2c_script_read(
            script, _address, _reply, (nuint)_reply.Length),
        Shape.WriteRead => NativeMethods.pamoja_i2c_script_write_read(
            script, _address, _bytes, (nuint)_bytes.Length, _reply, (nuint)_reply.Length),
        _ => NativeMethods.pamoja_i2c_script_fault(script, _address, (byte)_fault),
    };
}

/// <summary>One I2C bus, shared by the program and every driver built on it.</summary>
/// <remarks>
/// <para>
/// <see cref="Open"/> opens the kernel's adapter on a Linux board; <see cref="Simulated"/> puts
/// <see cref="I2cPart"/>s on a bus, each answering at its own address; <see cref="Scripted"/>
/// plays <see cref="I2cStep"/>s in order and refuses any other transfer. A driver runs the same
/// way over all three.
/// </para>
/// <para>
/// A failed transfer throws <see cref="PamojaException"/> with the reason: nothing answered at
/// the address, the script expected something else, or the kernel's own words. Only Linux has
/// the adapter; opening one anywhere else throws <see cref="PlatformNotSupportedException"/>.
/// </para>
/// </remarks>
public sealed class I2cBus : IDisposable
{
    private readonly NativeHandle _handle;

    private I2cBus(IntPtr bus) =>
        _handle = NativeHandle.Create(bus, NativeMethods.pamoja_i2c_bus_free, "I2C bus");

    /// <summary>What answers on the bus.</summary>
    public I2cBusKind Kind => (I2cBusKind)_handle.Use(NativeMethods.pamoja_i2c_bus_kind);

    /// <summary>
    /// How many transfers have been made on the bus, by the program and every driver on it,
    /// including any that failed.
    /// </summary>
    public int Transfers => (int)_handle.Use(NativeMethods.pamoja_i2c_bus_transfers);

    /// <summary>How many steps a script has left, or null when the bus is not scripted.</summary>
    public int? Remaining
    {
        get
        {
            nuint remaining = 0;
            bool scripted = _handle.Use(bus => NativeMethods.pamoja_i2c_bus_remaining(bus, out remaining));
            return scripted ? (int)remaining : null;
        }
    }

    /// <summary>
    /// How long the drivers on the bus have asked to wait, in microseconds, whether or not the
    /// process slept through it.
    /// </summary>
    public ulong WaitedMicros => _handle.Use(NativeMethods.pamoja_i2c_bus_waited_micros);

    /// <summary>Opens the kernel's I2C adapter, such as <c>/dev/i2c-1</c> on a Raspberry Pi.</summary>
    /// <param name="path">The adapter's device file.</param>
    /// <returns>The bus, with the real parts wired to it on the other end.</returns>
    /// <exception cref="PlatformNotSupportedException">The platform is not Linux.</exception>
    /// <exception cref="PamojaException">
    /// The file cannot be opened as an adapter: the interface is not turned on, or the process
    /// may not use it.
    /// </exception>
    public static I2cBus Open(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        PamojaStatus status = NativeMethods.pamoja_i2c_bus_open(path, out IntPtr bus);
        if (status == PamojaStatus.Unsupported)
        {
            throw new PlatformNotSupportedException(
                Status.LastError() ?? "an I2C adapter is opened only on Linux");
        }

        Status.ThrowIfError(status);
        return new I2cBus(bus);
    }

    /// <summary>Makes a bus of simulated parts, each answering at its own address.</summary>
    /// <param name="parts">
    /// The parts, copied onto the bus. A later part at an address an earlier one holds takes
    /// its place.
    /// </param>
    /// <returns>The bus. A transfer to an address no part holds throws, as nothing acknowledges it.</returns>
    public static I2cBus Simulated(params I2cPart[] parts)
    {
        ArgumentNullException.ThrowIfNull(parts);
        var bus = new I2cBus(NativeMethods.pamoja_i2c_bus_simulated());
        foreach (I2cPart part in parts)
        {
            bus.Attach(part);
        }

        return bus;
    }

    /// <summary>Makes a bus that plays the steps in order and refuses any other transfer.</summary>
    /// <param name="steps">The transfers a driver is expected to make, and the replies.</param>
    /// <returns>The bus.</returns>
    /// <exception cref="PamojaException">A step could not be added to the script.</exception>
    public static I2cBus Scripted(params I2cStep[] steps)
    {
        ArgumentNullException.ThrowIfNull(steps);
        IntPtr script = NativeMethods.pamoja_i2c_script_new();
        try
        {
            foreach (I2cStep step in steps)
            {
                Status.ThrowIfError(step.AddTo(script));
            }

            return new I2cBus(NativeMethods.pamoja_i2c_bus_scripted(script));
        }
        finally
        {
            NativeMethods.pamoja_i2c_script_free(script);
        }
    }

    /// <summary>
    /// Puts a copy of a part on a simulated bus, in place of any part at its address. A driver
    /// keeps working across the change, which is how a test moves a reading on.
    /// </summary>
    /// <param name="part">The part.</param>
    /// <exception cref="PamojaException">The bus is not simulated.</exception>
    public void Attach(I2cPart part)
    {
        ArgumentNullException.ThrowIfNull(part);
        Status.ThrowIfError(_handle.Use(bus =>
            part.Use(held => NativeMethods.pamoja_i2c_bus_attach(bus, held))));
    }

    /// <summary>Writes bytes to a part in one transaction: usually a register and its value.</summary>
    /// <param name="address">The part's 7-bit address.</param>
    /// <param name="bytes">The bytes to write.</param>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void Write(byte address, ReadOnlySpan<byte> bytes)
    {
        byte[] copy = bytes.ToArray();
        Status.ThrowIfError(_handle.Use(bus =>
            NativeMethods.pamoja_i2c_bus_write(bus, address, copy, (nuint)copy.Length)));
    }

    /// <summary>Reads bytes from a part in one transaction.</summary>
    /// <param name="address">The part's 7-bit address.</param>
    /// <param name="length">How many bytes to read.</param>
    /// <returns>The bytes.</returns>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public byte[] Read(byte address, int length)
    {
        byte[] bytes = new byte[length];
        Status.ThrowIfError(_handle.Use(bus =>
            NativeMethods.pamoja_i2c_bus_read(bus, address, bytes, (nuint)bytes.Length)));
        return bytes;
    }

    /// <summary>
    /// Writes bytes and then reads the reply in one transaction, with a repeated start between
    /// them, which is how a register is read.
    /// </summary>
    /// <param name="address">The part's 7-bit address.</param>
    /// <param name="bytes">What to write first, usually the register address.</param>
    /// <param name="length">How many bytes to read.</param>
    /// <returns>The reply.</returns>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public byte[] WriteRead(byte address, ReadOnlySpan<byte> bytes, int length)
    {
        byte[] copy = bytes.ToArray();
        byte[] reply = new byte[length];
        Status.ThrowIfError(_handle.Use(bus => NativeMethods.pamoja_i2c_bus_write_read(
            bus, address, copy, (nuint)copy.Length, reply, (nuint)reply.Length)));
        return reply;
    }

    /// <summary>Copies what a simulated part holds now, with whatever drivers wrote to it.</summary>
    /// <param name="address">The part's address.</param>
    /// <returns>The copy, or null when the bus is not simulated or no part holds the address.</returns>
    public I2cPart? Part(byte address)
    {
        IntPtr part = _handle.Use(bus => NativeMethods.pamoja_i2c_bus_part(bus, address));
        return part == IntPtr.Zero
            ? null
            : new I2cPart(new NativeHandle(part, NativeMethods.pamoja_i2c_part_free));
    }

    /// <summary>Runs a native call that needs this bus's handle.</summary>
    /// <remarks>
    /// A driver takes the bus's handle when it is built and holds its own share of the bus, so
    /// this bus may be disposed straight afterward.
    /// </remarks>
    /// <typeparam name="TResult">What the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    public TResult Use<TResult>(Func<IntPtr, TResult> call) => _handle.Use(call);

    /// <summary>Releases this share of the bus. The bus closes when no driver holds it either.</summary>
    public void Dispose() => _handle.Dispose();
}
