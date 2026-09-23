using Pamoja.Native.Interop;

namespace Pamoja.Gpio;

/// <summary>The physical voltage level on a pin.</summary>
public enum PinLevel
{
    /// <summary>A low level, near ground.</summary>
    Low = 0,

    /// <summary>A high level, near the supply voltage.</summary>
    High = 1,
}

/// <summary>The signal transition that triggers a pin interrupt.</summary>
public enum PinEdge
{
    /// <summary>A low-to-high transition.</summary>
    Rising = 0,

    /// <summary>A high-to-low transition.</summary>
    Falling = 1,

    /// <summary>Either transition.</summary>
    Both = 2,
}

/// <summary>Whether a signal is asserted by a high or a low physical level.</summary>
public enum PinPolarity
{
    /// <summary>A high level means asserted.</summary>
    ActiveHigh = 0,

    /// <summary>A low level means asserted, the wiring of most buttons and relay boards.</summary>
    ActiveLow = 1,
}

/// <summary>The clock polarity and phase pair an SPI mode number names.</summary>
public sealed class SpiClock
{
    /// <summary>Creates a pair.</summary>
    /// <param name="cpol">Whether the clock idles high.</param>
    /// <param name="cpha">Whether data is sampled on the trailing edge.</param>
    internal SpiClock(bool cpol, bool cpha)
    {
        Cpol = cpol;
        Cpha = cpha;
    }

    /// <summary>Whether the clock idles high (CPOL = 1), which is modes 2 and 3.</summary>
    public bool Cpol { get; }

    /// <summary>
    /// Whether data is sampled on the trailing edge (CPHA = 1), which is modes 1 and 3.
    /// </summary>
    public bool Cpha { get; }
}

/// <summary>I2C addressing per the NXP I2C-bus specification (UM10204).</summary>
/// <remarks>
/// The original 7-bit address shares its byte with the read/write bit, so it lands
/// on the wire as <c>(address &lt;&lt; 1) | r/w</c>. The 10-bit extension spends the
/// reserved <c>11110xx</c> prefix and takes two bytes.
/// </remarks>
public static class I2c
{
    /// <summary>The lowest 7-bit address the specification keeps for itself.</summary>
    public const byte ReservedFrom = NativeMethods.I2cReservedFrom;

    /// <summary>The first 7-bit address above the reserved block at the bottom.</summary>
    public const byte ReservedBelow = NativeMethods.I2cReservedBelow;

    /// <summary>Returns the address bytes a controller puts on the bus for a transfer.</summary>
    /// <param name="address">The device address.</param>
    /// <param name="read">Whether the transfer reads rather than writes.</param>
    /// <param name="tenBit">Whether this is a 10-bit address.</param>
    /// <returns>One byte for a 7-bit address, two for a 10-bit one.</returns>
    /// <exception cref="PamojaException">The address is outside its width's range.</exception>
    public static byte[] AddressFrame(ushort address, bool read = false, bool tenBit = false)
    {
        PamojaI2cAddress validated = Validate(address, tenBit);
        PamojaI2cDirection direction =
            read ? PamojaI2cDirection.Read : PamojaI2cDirection.Write;
        byte[] frame = new byte[2];
        Status.ThrowIfError(NativeMethods.pamoja_i2c_address_frame(
            validated, direction, frame, (nuint)frame.Length, out nuint written));
        return frame[..checked((int)written)];
    }

    /// <summary>Returns how many bytes an address frame occupies.</summary>
    /// <param name="address">The device address.</param>
    /// <param name="tenBit">Whether this is a 10-bit address.</param>
    /// <returns><c>1</c> for a 7-bit address, <c>2</c> for a 10-bit one.</returns>
    /// <exception cref="PamojaException">The address is outside its width's range.</exception>
    public static int FrameLen(ushort address, bool tenBit = false) =>
        checked((int)NativeMethods.pamoja_i2c_address_frame_len(Validate(address, tenBit)));

    /// <summary>Reports whether an address falls in a range the specification reserves.</summary>
    /// <param name="address">The device address.</param>
    /// <param name="tenBit">
    /// Whether this is a 10-bit address, which is never reserved in this sense.
    /// </param>
    /// <returns>Whether the address is reserved.</returns>
    /// <exception cref="PamojaException">The address is outside its width's range.</exception>
    /// <remarks>
    /// UM10204 reserves <c>0x00..=0x07</c> and <c>0x78..=0x7F</c>, leaving
    /// <c>0x08..=0x77</c> for ordinary devices.
    /// </remarks>
    public static bool IsReserved(ushort address, bool tenBit = false) =>
        NativeMethods.pamoja_i2c_address_is_reserved(Validate(address, tenBit));

    /// <summary>Reports whether an address is the general call address 0x00.</summary>
    /// <param name="address">The device address.</param>
    /// <param name="tenBit">Whether this is a 10-bit address.</param>
    /// <returns>Whether this is the broadcast every device on the bus listens to.</returns>
    /// <exception cref="PamojaException">The address is outside its width's range.</exception>
    public static bool IsGeneralCall(ushort address, bool tenBit = false) =>
        NativeMethods.pamoja_i2c_address_is_general_call(Validate(address, tenBit));

    /// <summary>Validates an address of the given width, rejecting one out of range.</summary>
    /// <param name="address">The device address.</param>
    /// <param name="tenBit">Whether this is a 10-bit address.</param>
    /// <returns>The validated address.</returns>
    /// <exception cref="PamojaException">The address is outside its width's range.</exception>
    private static PamojaI2cAddress Validate(ushort address, bool tenBit)
    {
        if (tenBit)
        {
            Status.ThrowIfError(
                NativeMethods.pamoja_i2c_address_ten_bit(address, out PamojaI2cAddress wide));
            return wide;
        }

        if (address > byte.MaxValue)
        {
            throw new PamojaException("I2C address is out of range");
        }

        Status.ThrowIfError(NativeMethods.pamoja_i2c_address_seven_bit(
            (byte)address, out PamojaI2cAddress narrow));
        return narrow;
    }
}

/// <summary>The four SPI clock modes, as the (CPOL, CPHA) pair datasheets quote.</summary>
public static class Spi
{
    /// <summary>Returns the clock polarity and phase a mode number names.</summary>
    /// <param name="mode">The mode number, 0 to 3.</param>
    /// <returns>The pair.</returns>
    /// <exception cref="PamojaException">The mode number is above 3.</exception>
    public static SpiClock ClockFor(byte mode)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_spi_mode_cpol_cpha(mode, out bool cpol, out bool cpha));
        return new SpiClock(cpol, cpha);
    }

    /// <summary>Returns the mode number a clock polarity and phase name.</summary>
    /// <param name="cpol">Whether the clock idles high.</param>
    /// <param name="cpha">Whether data is sampled on the trailing edge.</param>
    /// <returns>The mode number, 0 to 3. Every pair names a mode, so this never fails.</returns>
    public static byte ModeFor(bool cpol, bool cpha) =>
        NativeMethods.pamoja_spi_mode_from_cpol_cpha(cpol, cpha);
}

/// <summary>The GPIO pin model: levels, interrupt edges, and active polarity.</summary>
/// <remarks>
/// Active-low wiring is everywhere in cheap hardware: a button to ground with a
/// pull-up reads low when pressed, and many relay boards energize when driven low.
/// <see cref="PinPolarity"/> maps between "asserted" and the physical level so that
/// mapping lives in one place instead of in scattered inversions.
/// </remarks>
public static class Pin
{
    /// <summary>Returns the level a boolean names.</summary>
    /// <param name="high"><c>true</c> for high, <c>false</c> for low.</param>
    /// <returns>The level.</returns>
    public static PinLevel LevelFrom(bool high) =>
        (PinLevel)NativeMethods.pamoja_pin_level_from_bool(high);

    /// <summary>Returns the opposite level.</summary>
    /// <param name="level">The level to invert.</param>
    /// <returns>The other level.</returns>
    public static PinLevel Invert(PinLevel level) =>
        (PinLevel)NativeMethods.pamoja_pin_level_inverted((PamojaPinLevel)level);

    /// <summary>Reports whether a transition fires an interrupt trigger.</summary>
    /// <param name="edge">The trigger configured on the pin.</param>
    /// <param name="from">The level before the change.</param>
    /// <param name="to">The level after it.</param>
    /// <returns>Whether the trigger fires.</returns>
    public static bool Triggers(PinEdge edge, PinLevel from, PinLevel to) =>
        NativeMethods.pamoja_pin_edge_triggered_by(
            (PamojaPinEdge)edge, (PamojaPinLevel)from, (PamojaPinLevel)to);

    /// <summary>Returns the physical level that represents a logical state.</summary>
    /// <param name="polarity">How the signal is wired.</param>
    /// <param name="asserted">Whether the signal should be asserted.</param>
    /// <returns>The level to drive, inverted for active-low wiring.</returns>
    public static PinLevel LevelFor(PinPolarity polarity, bool asserted) =>
        (PinLevel)NativeMethods.pamoja_pin_polarity_level((PamojaPinPolarity)polarity, asserted);

    /// <summary>Reports whether a physical level means the signal is asserted.</summary>
    /// <param name="polarity">How the signal is wired.</param>
    /// <param name="level">The level read on the pin.</param>
    /// <returns>Whether the signal is asserted.</returns>
    public static bool IsAsserted(PinPolarity polarity, PinLevel level) =>
        NativeMethods.pamoja_pin_polarity_is_asserted(
            (PamojaPinPolarity)polarity, (PamojaPinLevel)level);
}

/// <summary>
/// A line a pin library drives: <c>System.Device.Gpio</c> on a Raspberry Pi, a vendor
/// SDK on a microcontroller, or a <see cref="PinScript"/> in a test. Anything with this
/// one method can sit under a <see cref="Switch{TLine}"/>.
/// </summary>
public interface IOutputLine
{
    /// <summary>Drives the line to a level.</summary>
    /// <param name="level">The level to drive.</param>
    void Drive(PinLevel level);
}

/// <summary>A line a pin library reads, which a <see cref="Contact{TLine}"/> sits over.</summary>
public interface IInputLine
{
    /// <summary>Reads the line's level now.</summary>
    /// <returns>The level on the line.</returns>
    PinLevel Read();
}

/// <summary>
/// A two-state output over any line, with its polarity said once: a relay, an LED, a
/// solenoid valve, a buzzer. <c>Set(true)</c> asserts it, which drives the line low for
/// an active-low part, so no call site inverts a level by hand.
/// </summary>
/// <typeparam name="TLine">The line the part is wired to.</typeparam>
public sealed class Switch<TLine>
    where TLine : IOutputLine
{
    private readonly TLine _line;

    /// <summary>Wraps a line, starting deasserted. Nothing is driven until <see cref="Set"/>.</summary>
    /// <param name="line">The line the part is wired to.</param>
    /// <param name="polarity">How the part is wired.</param>
    public Switch(TLine line, PinPolarity polarity)
    {
        _line = line;
        Polarity = polarity;
    }

    /// <summary>How the part is wired.</summary>
    public PinPolarity Polarity { get; }

    /// <summary>Whether the part was last set on.</summary>
    public bool IsAsserted { get; private set; }

    /// <summary>Turns the part on or off, driving whichever level that means for its wiring.</summary>
    /// <param name="asserted"><c>true</c> to turn it on.</param>
    /// <exception cref="Exception">Whatever the line throws when it cannot be driven.</exception>
    public void Set(bool asserted)
    {
        _line.Drive(Pin.LevelFor(Polarity, asserted));
        IsAsserted = asserted;
    }

    /// <summary>Hands the line back, for a test to read what was driven or a program to reuse it.</summary>
    /// <returns>The line.</returns>
    public TLine Release() => _line;
}

/// <summary>Builds switches, inferring the line's type.</summary>
public static class Switch
{
    /// <summary>A switch whose part is asserted by a high level.</summary>
    /// <typeparam name="TLine">The line the part is wired to.</typeparam>
    /// <param name="line">The line.</param>
    /// <returns>The switch.</returns>
    public static Switch<TLine> ActiveHigh<TLine>(TLine line)
        where TLine : IOutputLine =>
        new(line, PinPolarity.ActiveHigh);

    /// <summary>
    /// A switch whose part is asserted by a low level, the wiring of most relay boards.
    /// </summary>
    /// <typeparam name="TLine">The line the part is wired to.</typeparam>
    /// <param name="line">The line.</param>
    /// <returns>The switch.</returns>
    public static Switch<TLine> ActiveLow<TLine>(TLine line)
        where TLine : IOutputLine =>
        new(line, PinPolarity.ActiveLow);
}

/// <summary>
/// A two-state input over any line, with its polarity said once: a button, a float
/// switch, a reed switch, a limit switch. <see cref="IsAsserted"/> answers whether it is
/// closed, pressed, or tripped, whatever level that takes on the wire.
/// </summary>
/// <typeparam name="TLine">The line the part is wired to.</typeparam>
public sealed class Contact<TLine>
    where TLine : IInputLine
{
    private readonly TLine _line;

    /// <summary>Wraps a line.</summary>
    /// <param name="line">The line the part is wired to.</param>
    /// <param name="polarity">How the part is wired.</param>
    public Contact(TLine line, PinPolarity polarity)
    {
        _line = line;
        Polarity = polarity;
    }

    /// <summary>How the part is wired.</summary>
    public PinPolarity Polarity { get; }

    /// <summary>Reads the raw level on the line.</summary>
    /// <returns>The level.</returns>
    /// <exception cref="Exception">Whatever the line throws when it cannot be read.</exception>
    public PinLevel Level() => _line.Read();

    /// <summary>Reads the line and reports whether the part is asserted.</summary>
    /// <returns>Whether it is closed, pressed, or tripped.</returns>
    /// <exception cref="Exception">Whatever the line throws when it cannot be read.</exception>
    public bool IsAsserted() => Pin.IsAsserted(Polarity, _line.Read());

    /// <summary>Hands the line back.</summary>
    /// <returns>The line.</returns>
    public TLine Release() => _line;
}

/// <summary>Builds contacts, inferring the line's type.</summary>
public static class Contact
{
    /// <summary>A contact that reads high when asserted.</summary>
    /// <typeparam name="TLine">The line the part is wired to.</typeparam>
    /// <param name="line">The line.</param>
    /// <returns>The contact.</returns>
    public static Contact<TLine> ActiveHigh<TLine>(TLine line)
        where TLine : IInputLine =>
        new(line, PinPolarity.ActiveHigh);

    /// <summary>
    /// A contact that reads low when asserted, the wiring of a switch to ground with a
    /// pull-up.
    /// </summary>
    /// <typeparam name="TLine">The line the part is wired to.</typeparam>
    /// <param name="line">The line.</param>
    /// <returns>The contact.</returns>
    public static Contact<TLine> ActiveLow<TLine>(TLine line)
        where TLine : IInputLine =>
        new(line, PinPolarity.ActiveLow);
}

/// <summary>
/// A GPIO line opened on a Linux board, through the kernel's GPIO character device: the
/// line a <see cref="Switch{TLine}"/> or a <see cref="Contact{TLine}"/> sits over on a
/// Raspberry Pi or any Linux board.
/// </summary>
/// <remarks>
/// <para>
/// A line is held by one process at a time, and the kernel names the holder, which
/// <c>gpioinfo</c> prints; a line another program or a kernel driver holds cannot be opened
/// until it is let go. On Raspberry Pi OS a user in the <c>gpio</c> group opens lines
/// without root. Dispose the line to hand it back.
/// </para>
/// <para>
/// Only Linux has the GPIO character device; every other platform throws
/// <see cref="PlatformNotSupportedException"/>.
/// </para>
/// </remarks>
public sealed class GpioLine : IOutputLine, IInputLine, IDisposable
{
    private readonly NativeHandle _handle;

    private GpioLine(IntPtr line, string chip, uint offset)
    {
        _handle = new NativeHandle(line, NativeMethods.pamoja_gpio_line_free);
        Chip = chip;
        Offset = offset;
    }

    /// <summary>The GPIO chip's device file.</summary>
    public string Chip { get; }

    /// <summary>The line's number on its chip.</summary>
    public uint Offset { get; }

    /// <summary>Opens a line as an output, driving <paramref name="initial"/> from the moment it is taken.</summary>
    /// <param name="chip">The GPIO chip's device file, <c>/dev/gpiochip0</c> on most boards.</param>
    /// <param name="line">The line's number on that chip, the GPIO or BCM number on a Raspberry Pi.</param>
    /// <param name="initial">
    /// The level to drive as soon as the line is taken; an active-low relay is opened high so
    /// it stays off.
    /// </param>
    /// <returns>The line.</returns>
    /// <exception cref="PlatformNotSupportedException">The platform is not Linux.</exception>
    /// <exception cref="PamojaException">The chip or the line could not be opened.</exception>
    public static GpioLine OpenOutput(string chip, uint line, PinLevel initial)
    {
        ArgumentNullException.ThrowIfNull(chip);
        PamojaStatus status = NativeMethods.pamoja_gpio_line_open_output(
            chip, line, (PamojaPinLevel)initial, out IntPtr opened);
        return Opened(status, opened, chip, line);
    }

    /// <summary>Opens a line as an input.</summary>
    /// <param name="chip">The GPIO chip's device file.</param>
    /// <param name="line">The line's number on that chip.</param>
    /// <returns>The line.</returns>
    /// <exception cref="PlatformNotSupportedException">The platform is not Linux.</exception>
    /// <exception cref="PamojaException">The chip or the line could not be opened.</exception>
    public static GpioLine OpenInput(string chip, uint line)
    {
        ArgumentNullException.ThrowIfNull(chip);
        PamojaStatus status = NativeMethods.pamoja_gpio_line_open_input(chip, line, out IntPtr opened);
        return Opened(status, opened, chip, line);
    }

    /// <summary>Drives the line to a level. The line must have been opened as an output.</summary>
    /// <param name="level">The level to drive.</param>
    /// <exception cref="PamojaException">The kernel refused the write, which an input line does.</exception>
    public void Drive(PinLevel level) =>
        Status.ThrowIfError(
            _handle.Use(line => NativeMethods.pamoja_gpio_line_drive(line, (PamojaPinLevel)level)));

    /// <summary>Reads the level on the line now.</summary>
    /// <returns>The level.</returns>
    /// <exception cref="PamojaException">The kernel refused the read.</exception>
    public PinLevel Read()
    {
        PamojaPinLevel level = PamojaPinLevel.Low;
        Status.ThrowIfError(_handle.Use(line => NativeMethods.pamoja_gpio_line_read(line, out level)));
        return (PinLevel)level;
    }

    /// <summary>Hands the line back to the kernel.</summary>
    public void Dispose() => _handle.Dispose();

    private static GpioLine Opened(PamojaStatus status, IntPtr line, string chip, uint offset)
    {
        if (status == PamojaStatus.Unsupported)
        {
            throw new PlatformNotSupportedException(
                Status.LastError() ?? "a GPIO line is opened only on Linux");
        }

        Status.ThrowIfError(status);
        return new GpioLine(line, chip, offset);
    }
}

/// <summary>
/// A line for running with nothing plugged in: it answers the reads it was given, in
/// order, and records every level it is driven to. It is what the examples and tests put
/// under a switch or a contact, and the one thing a real node replaces with its board's
/// pin library.
/// </summary>
/// <remarks>
/// It behaves as <c>pamoja_hal::script::PinScript</c> does in Rust: it starts released,
/// high, and once its reads run out a read answers the level it was last driven to.
/// </remarks>
public sealed class PinScript : IOutputLine, IInputLine
{
    private readonly Queue<PinLevel> _inputs;
    private readonly List<PinLevel> _driven = new();

    /// <summary>Creates a released line.</summary>
    /// <param name="inputs">The levels to answer reads with, in order.</param>
    public PinScript(params PinLevel[] inputs) => _inputs = new Queue<PinLevel>(inputs);

    /// <summary>Every level the line was driven to, oldest first.</summary>
    public IReadOnlyList<PinLevel> Driven => _driven;

    /// <summary>The level the line was last driven to, high while it has never been driven.</summary>
    public PinLevel Level { get; private set; } = PinLevel.High;

    /// <summary>How many scripted reads are left.</summary>
    public int Remaining => _inputs.Count;

    /// <summary>Records a driven level.</summary>
    /// <param name="level">The level driven.</param>
    public void Drive(PinLevel level)
    {
        Level = level;
        _driven.Add(level);
    }

    /// <summary>Answers the next scripted level, or the driven level once the script runs out.</summary>
    /// <returns>The level.</returns>
    public PinLevel Read() => _inputs.Count > 0 ? _inputs.Dequeue() : Level;
}
