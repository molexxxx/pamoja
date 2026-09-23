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

/// <summary>A part that is not there, of any of the three kinds a simulated bus holds.</summary>
/// <remarks>
/// <see cref="I2cPart"/> answers from registers a byte wide, as Bosch's parts do;
/// <see cref="WordPart"/> from registers sixteen bits wide, as Texas Instruments' parts do; and
/// <see cref="CommandPart"/> from commands and the replies they leave, as Sensirion's parts do.
/// A simulated bus takes any of them, and <see cref="I2cBus.Part(byte)"/> gives each back as
/// its own kind.
/// </remarks>
public abstract class SimulatedPart : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a part's native handle.</summary>
    /// <param name="handle">The part's handle, released with <c>pamoja_i2c_part_free</c>.</param>
    private protected SimulatedPart(NativeHandle handle)
    {
        ArgumentNullException.ThrowIfNull(handle);
        _handle = handle;
    }

    /// <summary>The address the part answers to.</summary>
    public byte Address => _handle.Use(NativeMethods.pamoja_i2c_part_address);

    /// <summary>How many transfers the part has served.</summary>
    public int Transfers => (int)_handle.Use(NativeMethods.pamoja_i2c_part_transfers);

    /// <summary>Wraps a part a native call made as the kind it is.</summary>
    /// <param name="handle">The part's handle, released with <c>pamoja_i2c_part_free</c>.</param>
    /// <returns>An <see cref="I2cPart"/>, a <see cref="WordPart"/>, or a <see cref="CommandPart"/>.</returns>
    public static SimulatedPart FromHandle(NativeHandle handle)
    {
        ArgumentNullException.ThrowIfNull(handle);
        return handle.Use(NativeMethods.pamoja_i2c_part_kind) switch
        {
            NativeMethods.I2cPartWords => new WordPart(handle),
            NativeMethods.I2cPartCommands => new CommandPart(handle),
            _ => new I2cPart(handle),
        };
    }

    /// <summary>Runs a native call that needs this part's handle.</summary>
    /// <typeparam name="TResult">What the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    public TResult Use<TResult>(Func<IntPtr, TResult> call) => _handle.Use(call);

    /// <inheritdoc/>
    public void Dispose()
    {
        _handle.Dispose();
        GC.SuppressFinalize(this);
    }

    /// <summary>Wraps a new native part, or throws when the native side made none.</summary>
    /// <param name="part">The handle a native constructor returned.</param>
    /// <param name="what">What the part is, for the error.</param>
    /// <returns>The owned handle.</returns>
    private protected static NativeHandle Own(IntPtr part, string what) =>
        NativeHandle.Create(part, NativeMethods.pamoja_i2c_part_free, what);
}

/// <summary>A part that is not there, answering from 256 registers a byte wide.</summary>
/// <remarks>
/// A write names a register and fills it and the ones after it; a read takes them back from
/// wherever the last write left off. What a driver writes stays written, so
/// <see cref="Register"/> reads a part's configuration back once a driver is done with it.
/// </remarks>
public sealed class I2cPart : SimulatedPart
{
    /// <summary>Creates a part answering at one address, with every register reading zero.</summary>
    /// <param name="address">The 7-bit address it answers to.</param>
    public I2cPart(byte address)
        : base(Own(NativeMethods.pamoja_i2c_part_new(address), "I2C part"))
    {
    }

    /// <summary>Wraps a part a native call made, such as a simulated sensor from another package.</summary>
    /// <param name="handle">The part's handle, released with <c>pamoja_i2c_part_free</c>.</param>
    public I2cPart(NativeHandle handle)
        : base(handle)
    {
    }

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
        Status.ThrowIfError(Use(part =>
            NativeMethods.pamoja_i2c_part_load(part, first, copy, (nuint)copy.Length)));
    }

    /// <summary>Reads what one register holds now.</summary>
    /// <param name="register">Which register.</param>
    /// <returns>Its value, which is what a driver wrote if it wrote one.</returns>
    public byte Register(byte register) =>
        Use(part => NativeMethods.pamoja_i2c_part_register(part, register));

    /// <summary>Reads consecutive registers from one register on.</summary>
    /// <param name="first">The first register.</param>
    /// <param name="length">How many registers.</param>
    /// <returns>One byte per register.</returns>
    public byte[] Read(byte first, int length)
    {
        byte[] bytes = new byte[length];
        Status.ThrowIfError(Use(part =>
            NativeMethods.pamoja_i2c_part_read(part, first, bytes, (nuint)bytes.Length)));
        return bytes;
    }
}

/// <summary>A part that is not there, answering from 256 registers sixteen bits wide.</summary>
/// <remarks>
/// A pointer byte names a register and a register travels most significant byte first. A
/// write of the pointer alone aims the next read; a write of the pointer and a word stores the
/// word; a read takes words from the pointer on. Bits the part sets for itself, such as a
/// conversion-ready flag, are marked with <see cref="ReadOnly"/> and keep the part's value
/// whatever a driver writes.
/// </remarks>
public sealed class WordPart : SimulatedPart
{
    /// <summary>Creates a part answering at one address, with every register reading zero.</summary>
    /// <param name="address">The 7-bit address it answers to.</param>
    public WordPart(byte address)
        : base(Own(NativeMethods.pamoja_i2c_word_part_new(address), "I2C word part"))
    {
    }

    /// <summary>Wraps a part a native call made, such as a simulated sensor from another package.</summary>
    /// <param name="handle">The part's handle, released with <c>pamoja_i2c_part_free</c>.</param>
    public WordPart(NativeHandle handle)
        : base(handle)
    {
    }

    /// <summary>Puts a value in one register and returns the part.</summary>
    /// <param name="register">The register.</param>
    /// <param name="value">What it holds, read-only bits included.</param>
    /// <returns>This part, so calls chain.</returns>
    public WordPart Holding(byte register, ushort value)
    {
        Set(register, value);
        return this;
    }

    /// <summary>Marks bits of one register as the part's to set, and returns the part.</summary>
    /// <param name="register">The register.</param>
    /// <param name="mask">The bits a driver's write leaves as the part holds them.</param>
    /// <returns>This part, so calls chain.</returns>
    public WordPart ReadOnly(byte register, ushort mask)
    {
        Status.ThrowIfError(Use(part => NativeMethods.pamoja_i2c_part_read_only(part, register, mask)));
        return this;
    }

    /// <summary>Puts a value in one register, read-only bits included, the way the part itself would.</summary>
    /// <param name="register">The register.</param>
    /// <param name="value">What it holds.</param>
    public void Set(byte register, ushort value) =>
        Status.ThrowIfError(Use(part => NativeMethods.pamoja_i2c_part_set_word(part, register, value)));

    /// <summary>Reads what one register holds now.</summary>
    /// <param name="register">Which register.</param>
    /// <returns>Its value, which is what a driver wrote there apart from the read-only bits.</returns>
    public ushort Word(byte register) =>
        Use(part => NativeMethods.pamoja_i2c_part_word(part, register));
}

/// <summary>A part that is not there, answering commands with the replies it was given.</summary>
/// <remarks>
/// A write sends a command and any arguments after it; a read then takes the reply that
/// command left, once, padded with <c>0xFF</c> the way an idle bus reads. A command given no
/// reply leaves none, and a read then is not acknowledged, which is what a real part does when
/// asked for data it does not have.
/// </remarks>
public sealed class CommandPart : SimulatedPart
{
    /// <summary>Creates a part answering at one address that has been given no replies yet.</summary>
    /// <param name="address">The 7-bit address it answers to.</param>
    /// <param name="width">How many bytes a command takes: two for Sensirion's 16-bit commands.</param>
    public CommandPart(byte address, int width = 2)
        : base(Own(NativeMethods.pamoja_i2c_command_part_new(address, (nuint)width), "I2C command part"))
    {
    }

    /// <summary>Wraps a part a native call made, such as a simulated sensor from another package.</summary>
    /// <param name="handle">The part's handle, released with <c>pamoja_i2c_part_free</c>.</param>
    public CommandPart(NativeHandle handle)
        : base(handle)
    {
    }

    /// <summary>Every write the part has received, oldest first: a command and any arguments after it.</summary>
    public IReadOnlyList<byte[]> Received
    {
        get
        {
            int count = (int)Use(NativeMethods.pamoja_i2c_part_received_count);
            var writes = new List<byte[]>(count);
            for (int index = 0; index < count; index++)
            {
                IntPtr buffer = Use(part => NativeMethods.pamoja_i2c_part_received(part, (nuint)index));
                writes.Add(OwnedBuffer.Take(buffer));
            }

            return writes;
        }
    }

    /// <summary>Answers one command with a reply from now on, and returns the part.</summary>
    /// <param name="command">The command's bytes.</param>
    /// <param name="reply">What a read after it returns, in place of any reply given before.</param>
    /// <returns>This part, so calls chain.</returns>
    public CommandPart Answering(ReadOnlySpan<byte> command, ReadOnlySpan<byte> reply)
    {
        Answer(command, reply);
        return this;
    }

    /// <summary>Answers one command with a reply from now on.</summary>
    /// <param name="command">The command's bytes.</param>
    /// <param name="reply">What a read after it returns, in place of any reply given before.</param>
    public void Answer(ReadOnlySpan<byte> command, ReadOnlySpan<byte> reply)
    {
        byte[] commandCopy = command.ToArray();
        byte[] replyCopy = reply.ToArray();
        Status.ThrowIfError(Use(part => NativeMethods.pamoja_i2c_part_answer(
            part, commandCopy, (nuint)commandCopy.Length, replyCopy, (nuint)replyCopy.Length)));
    }
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
/// <see cref="SimulatedPart"/>s on a bus, each answering at its own address;
/// <see cref="Scripted"/> plays <see cref="I2cStep"/>s in order and refuses any other transfer.
/// A driver runs the same way over all three.
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
    /// The parts, of any kind, copied onto the bus. A later part at an address an earlier one
    /// holds takes its place.
    /// </param>
    /// <returns>The bus. A transfer to an address no part holds throws, as nothing acknowledges it.</returns>
    public static I2cBus Simulated(params SimulatedPart[] parts)
    {
        ArgumentNullException.ThrowIfNull(parts);
        var bus = new I2cBus(NativeMethods.pamoja_i2c_bus_simulated());
        foreach (SimulatedPart part in parts)
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
    /// <param name="part">The part, of any kind.</param>
    /// <exception cref="PamojaException">The bus is not simulated.</exception>
    public void Attach(SimulatedPart part)
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
    /// <returns>
    /// The copy, as the kind of part it is, or null when the bus is not simulated or no part
    /// holds the address.
    /// </returns>
    public SimulatedPart? Part(byte address)
    {
        IntPtr part = _handle.Use(bus => NativeMethods.pamoja_i2c_bus_part(bus, address));
        return part == IntPtr.Zero
            ? null
            : SimulatedPart.FromHandle(new NativeHandle(part, NativeMethods.pamoja_i2c_part_free));
    }

    /// <summary>Copies what a simulated part of one kind holds now, with whatever drivers wrote to it.</summary>
    /// <typeparam name="TPart"><see cref="I2cPart"/>, <see cref="WordPart"/>, or <see cref="CommandPart"/>.</typeparam>
    /// <param name="address">The part's address.</param>
    /// <returns>
    /// The copy, or null when the bus is not simulated, no part holds the address, or the part
    /// there is of another kind.
    /// </returns>
    public TPart? Part<TPart>(byte address)
        where TPart : SimulatedPart
    {
        SimulatedPart? part = Part(address);
        if (part is TPart wanted)
        {
            return wanted;
        }

        part?.Dispose();
        return null;
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

/// <summary>
/// What paces a driver that has to wait between pin changes, such as a stepper between
/// steps. <see cref="SleepDelay"/> really waits; <see cref="DelayLog"/> counts every wait and
/// waits for none, for a program run with nothing plugged in.
/// </summary>
public interface IDelay
{
    /// <summary>Waits, or counts the wait.</summary>
    /// <param name="micros">How long, in microseconds.</param>
    void DelayMicros(uint micros);
}

/// <summary>
/// A delay that records every wait it is asked for and sleeps through none of them, as
/// <c>pamoja_hal::script::DelayLog</c> does in Rust.
/// </summary>
/// <example>
/// <code>
/// var delay = new DelayLog();
/// delay.DelayMicros(480);
/// delay.DelayMicros(10_000);
/// // delay.TotalMicros is 10480 and delay.TotalMillis is 10
/// </code>
/// </example>
public sealed class DelayLog : IDelay
{
    private readonly List<uint> _waits = new();

    /// <summary>Every wait asked for, in microseconds, oldest first.</summary>
    public IReadOnlyList<uint> WaitsMicros => _waits;

    /// <summary>The waits added up, in microseconds.</summary>
    public ulong TotalMicros { get; private set; }

    /// <summary>The waits added up, in whole milliseconds, rounded down.</summary>
    public ulong TotalMillis => TotalMicros / 1_000;

    /// <summary>Records a wait.</summary>
    /// <param name="micros">How long, in microseconds.</param>
    public void DelayMicros(uint micros)
    {
        _waits.Add(micros);
        TotalMicros += micros;
    }

    /// <summary>Forgets every recorded wait.</summary>
    public void Clear()
    {
        _waits.Clear();
        TotalMicros = 0;
    }
}

/// <summary>
/// A delay that really waits: <see cref="Thread.Sleep(int)"/>, rounded up to whole
/// milliseconds, for a millisecond or more, and a spin on
/// <see cref="System.Diagnostics.Stopwatch"/> for a shorter wait, which the scheduler cannot
/// keep. A sleep can run over by the scheduler's own latency.
/// </summary>
public sealed class SleepDelay : IDelay
{
    /// <summary>Waits.</summary>
    /// <param name="micros">How long, in microseconds.</param>
    public void DelayMicros(uint micros)
    {
        if (micros >= 1_000)
        {
            Thread.Sleep(checked((int)((micros + 999) / 1_000)));
            return;
        }

        long until = System.Diagnostics.Stopwatch.GetTimestamp()
            + (long)micros * System.Diagnostics.Stopwatch.Frequency / 1_000_000;
        while (System.Diagnostics.Stopwatch.GetTimestamp() < until)
        {
            Thread.SpinWait(1);
        }
    }
}
