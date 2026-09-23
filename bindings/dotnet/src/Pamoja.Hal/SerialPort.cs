using Pamoja.Native.Interop;

namespace Pamoja.Hal;

/// <summary>The parity bit each character on a serial line carries.</summary>
public enum Parity : byte
{
    /// <summary>No parity bit.</summary>
    None = NativeMethods.ParityNone,

    /// <summary>A bit that makes the count of ones even, what Modbus RTU asks for by default.</summary>
    Even = NativeMethods.ParityEven,

    /// <summary>A bit that makes the count of ones odd.</summary>
    Odd = NativeMethods.ParityOdd,
}

/// <summary>What is on the other end of a serial port.</summary>
public enum SerialPortKind : byte
{
    /// <summary>The kernel's serial device, with a real line on the other end.</summary>
    Device = NativeMethods.SerialPortDevice,

    /// <summary>The port's own output, looped back to its input.</summary>
    Looped = NativeMethods.SerialPortLooped,

    /// <summary>The other end of a null-modem pair.</summary>
    Paired = NativeMethods.SerialPortPaired,

    /// <summary>A simulated device that answers each write.</summary>
    Simulated = NativeMethods.SerialPortSimulated,

    /// <summary>A script of the writes a driver is expected to make.</summary>
    Scripted = NativeMethods.SerialPortScripted,
}

/// <summary>
/// A port's speed and character format: eight data bits, with the parity and stop bits given.
/// </summary>
/// <param name="Baud">The speed, in bits a second.</param>
/// <param name="Parity">The parity bit each character carries.</param>
/// <param name="StopBits">1 or 2.</param>
/// <example>
/// <code>
/// var meter = new SerialSettings(9_600, Parity.Even);
/// // meter.ToString() is "9600 8E1", eleven bits a character
/// </code>
/// </example>
public readonly record struct SerialSettings(uint Baud, Parity Parity = Parity.None, byte StopBits = 1)
{
    /// <summary>
    /// The bits one character takes on the wire: a start bit, eight data bits, the parity bit
    /// if there is one, and the stop bits.
    /// </summary>
    /// <exception cref="PamojaException">The settings are not ones a port has.</exception>
    public uint BitsPerCharacter => Checked(NativeMethods.pamoja_serial_settings_bits_per_character(ToNative()));

    /// <summary>How long one character takes on the wire, in nanoseconds, rounded up.</summary>
    /// <exception cref="PamojaException">The settings are not ones a port has.</exception>
    public ulong CharacterNanos => Checked(NativeMethods.pamoja_serial_settings_character_nanos(ToNative()));

    /// <summary>Returns how long bytes sent back to back take on the wire.</summary>
    /// <param name="bytes">How many bytes.</param>
    /// <returns>The time in microseconds, rounded up.</returns>
    /// <exception cref="PamojaException">The settings are not ones a port has.</exception>
    public ulong TransferMicros(int bytes) => ((CharacterNanos * (ulong)bytes) + 999) / 1_000;

    /// <summary>Writes the settings the way a device's manual does, such as <c>9600 8E1</c>.</summary>
    /// <returns>The speed, the eight data bits, the parity letter, and the stop bit count.</returns>
    public override string ToString()
    {
        char letter = Parity switch
        {
            Parity.Even => 'E',
            Parity.Odd => 'O',
            _ => 'N',
        };
        return $"{Baud} 8{letter}{StopBits}";
    }

    /// <summary>Converts to the flat struct the C ABI takes.</summary>
    /// <returns>The interop representation.</returns>
    internal PamojaSerialSettings ToNative() => new()
    {
        Baud = Baud,
        Parity = (byte)Parity,
        StopBits = StopBits,
    };

    /// <summary>Converts from the flat struct the C ABI returns.</summary>
    /// <param name="settings">The interop representation.</param>
    /// <returns>The settings.</returns>
    internal static SerialSettings FromNative(PamojaSerialSettings settings) =>
        new(settings.Baud, (Parity)settings.Parity, settings.StopBits);

    private static T Checked<T>(T value)
        where T : System.Numerics.INumber<T>
    {
        if (T.IsZero(value))
        {
            throw new PamojaException(Status.LastError() ?? "settings a serial port does not have");
        }

        return value;
    }
}

/// <summary>
/// One step of a scripted port: <see cref="Write"/> for a write the program is expected to
/// make, and <see cref="Read"/> for bytes the far end sends.
/// </summary>
public sealed class SerialStep
{
    private SerialStep(bool written, byte[] bytes)
    {
        Written = written;
        Bytes = bytes;
    }

    /// <summary>Whether the program is expected to write the bytes, rather than read them.</summary>
    public bool Written { get; }

    /// <summary>The bytes of the step.</summary>
    public IReadOnlyList<byte> Bytes { get; }

    /// <summary>A write the program is expected to make, in one call.</summary>
    /// <param name="bytes">The bytes of the write.</param>
    /// <returns>The step.</returns>
    public static SerialStep Write(ReadOnlySpan<byte> bytes) => new(true, bytes.ToArray());

    /// <summary>Bytes the far end sends, readable once every step before them has happened.</summary>
    /// <param name="bytes">What arrives.</param>
    /// <returns>The step.</returns>
    public static SerialStep Read(ReadOnlySpan<byte> bytes) => new(false, bytes.ToArray());
}

/// <summary>One serial port, shared by the program and every driver built on it.</summary>
/// <remarks>
/// <para>
/// <see cref="Open"/> opens the kernel's serial device raw on a Linux board: <c>/dev/serial0</c>
/// for a Raspberry Pi's own UART, <c>/dev/ttyUSB0</c> or <c>/dev/ttyACM0</c> for a USB adapter.
/// <see cref="Looped"/> is a line with TX wired to RX, <see cref="Pair"/> the two ends of a
/// null-modem cable, and <see cref="Scripted"/> a port that checks each write against a script.
/// </para>
/// <para>
/// A write returns once the bytes have left the UART, and a read once bytes have arrived or
/// its timeout has passed. On anything but the kernel's device a read never waits: it returns
/// at once, and the time it would have waited is added to <see cref="WaitedMicros"/>. A
/// failure throws <see cref="PamojaException"/> with the reason, and a port is thread-safe, so
/// one thread may read while another writes.
/// </para>
/// </remarks>
public sealed class SerialPort : IDisposable
{
    private readonly NativeHandle _handle;

    private SerialPort(IntPtr port) =>
        _handle = NativeHandle.Create(port, NativeMethods.pamoja_serial_port_free, "serial port");

    /// <summary>What is on the other end of the port.</summary>
    public SerialPortKind Kind => (SerialPortKind)_handle.Use(NativeMethods.pamoja_serial_port_kind);

    /// <summary>The speed and character format the port runs at.</summary>
    public SerialSettings Settings =>
        SerialSettings.FromNative(_handle.Use(NativeMethods.pamoja_serial_port_settings));

    /// <summary>How many bytes have been written through the port.</summary>
    public long Written => (long)_handle.Use(NativeMethods.pamoja_serial_port_written);

    /// <summary>How many bytes have been read through the port.</summary>
    public long Received => (long)_handle.Use(NativeMethods.pamoja_serial_port_received);

    /// <summary>
    /// How long reads have waited without an answer, and waits have waited, in microseconds,
    /// whether or not the process slept through it.
    /// </summary>
    public ulong WaitedMicros => _handle.Use(NativeMethods.pamoja_serial_port_waited_micros);

    /// <summary>How many steps a script has left, or null when the port is not scripted.</summary>
    public int? Remaining
    {
        get
        {
            nuint remaining = 0;
            bool scripted = _handle.Use(port => NativeMethods.pamoja_serial_port_remaining(port, out remaining));
            return scripted ? (int)remaining : null;
        }
    }

    /// <summary>
    /// Opens the kernel's serial device raw: no echo, no line editing, no translation, just
    /// bytes. Whatever the device received before it was opened is dropped.
    /// </summary>
    /// <param name="path">The device file.</param>
    /// <param name="settings">The speed, a standard rate from 1200 to 921600, and the format.</param>
    /// <returns>The port, with the real line on the other end.</returns>
    /// <exception cref="PlatformNotSupportedException">The platform is not Linux.</exception>
    /// <exception cref="PamojaException">The device cannot be opened or set up.</exception>
    public static SerialPort Open(string path, SerialSettings settings)
    {
        ArgumentNullException.ThrowIfNull(path);
        PamojaStatus status = NativeMethods.pamoja_serial_port_open(path, settings.ToNative(), out IntPtr port);
        if (status == PamojaStatus.Unsupported)
        {
            throw new PlatformNotSupportedException(
                Status.LastError() ?? "a serial device is opened only on Linux");
        }

        Status.ThrowIfError(status);
        return new SerialPort(port);
    }

    /// <summary>A line looped back on itself: every byte written is waiting to be read.</summary>
    /// <param name="settings">The speed and format the line runs at.</param>
    /// <returns>The port.</returns>
    /// <exception cref="PamojaException">The settings are not ones a port has.</exception>
    public static SerialPort Looped(SerialSettings settings) =>
        Created(NativeMethods.pamoja_serial_port_looped(settings.ToNative()));

    /// <summary>The two ends of a null-modem pair: what one end writes, the other reads.</summary>
    /// <param name="settings">The speed and format both ends run at.</param>
    /// <returns>The two ends.</returns>
    /// <exception cref="PamojaException">The settings are not ones a port has.</exception>
    public static (SerialPort One, SerialPort Other) Pair(SerialSettings settings)
    {
        Status.ThrowIfError(NativeMethods.pamoja_serial_port_pair(
            settings.ToNative(), out IntPtr one, out IntPtr other));
        return (new SerialPort(one), new SerialPort(other));
    }

    /// <summary>
    /// A port that checks each write against the next step of a script, and makes the bytes the
    /// far end sends readable as the script reaches them.
    /// </summary>
    /// <param name="settings">The speed and format the line runs at.</param>
    /// <param name="steps">The writes and reads, in order; reads at the start are there at once.</param>
    /// <returns>The port.</returns>
    /// <exception cref="PamojaException">The settings are not ones a port has.</exception>
    public static SerialPort Scripted(SerialSettings settings, params SerialStep[] steps)
    {
        ArgumentNullException.ThrowIfNull(steps);
        IntPtr script = NativeMethods.pamoja_serial_script_new();
        try
        {
            foreach (SerialStep step in steps)
            {
                byte[] bytes = [.. step.Bytes];
                Status.ThrowIfError(step.Written
                    ? NativeMethods.pamoja_serial_script_write(script, bytes, (nuint)bytes.Length)
                    : NativeMethods.pamoja_serial_script_read(script, bytes, (nuint)bytes.Length));
            }

            return Created(NativeMethods.pamoja_serial_port_scripted(settings.ToNative(), script));
        }
        finally
        {
            NativeMethods.pamoja_serial_script_free(script);
        }
    }

    /// <summary>Writes bytes, returning once they have left the UART.</summary>
    /// <param name="bytes">The bytes, in order.</param>
    /// <exception cref="PamojaException">A script expected another write, or the device failed.</exception>
    public void Write(ReadOnlySpan<byte> bytes)
    {
        byte[] copy = bytes.ToArray();
        Status.ThrowIfError(_handle.Use(port =>
            NativeMethods.pamoja_serial_port_write(port, copy, (nuint)copy.Length)));
    }

    /// <summary>
    /// Reads up to <paramref name="max"/> bytes, waiting up to <paramref name="timeout"/> for the
    /// first one when nothing has arrived.
    /// </summary>
    /// <param name="max">The most bytes to read.</param>
    /// <param name="timeout">How long to wait for the first byte.</param>
    /// <returns>What arrived, empty when the timeout passed with nothing.</returns>
    /// <exception cref="PamojaException">The device failed.</exception>
    public byte[] Read(int max, TimeSpan timeout)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(max);
        byte[] buffer = new byte[max];
        nuint got = 0;
        ulong micros = Micros(timeout);
        Status.ThrowIfError(_handle.Use(port =>
            NativeMethods.pamoja_serial_port_read(port, buffer, (nuint)buffer.Length, micros, out got)));
        return buffer[..(int)got];
    }

    /// <summary>
    /// Drops whatever has arrived and not been read, as a client does before a request so a
    /// stale reply cannot be taken for the new one.
    /// </summary>
    /// <exception cref="PamojaException">The device failed.</exception>
    public void DiscardInput() =>
        Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_serial_port_discard_input));

    /// <summary>
    /// Waits, as a protocol does to leave the line silent between frames: really on the kernel's
    /// device, and anywhere else only counted.
    /// </summary>
    /// <param name="duration">How long.</param>
    public void Wait(TimeSpan duration)
    {
        ulong micros = Micros(duration);
        _handle.Use(port => NativeMethods.pamoja_serial_port_wait(port, micros));
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    private static SerialPort Created(IntPtr port)
    {
        if (port == IntPtr.Zero)
        {
            throw new PamojaException(Status.LastError() ?? "the port could not be made");
        }

        return new SerialPort(port);
    }

    private static ulong Micros(TimeSpan duration)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(duration, TimeSpan.Zero);
        return (ulong)(duration.Ticks / TimeSpan.TicksPerMicrosecond);
    }
}
