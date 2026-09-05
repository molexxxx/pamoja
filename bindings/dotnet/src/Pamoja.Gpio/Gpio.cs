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
/// pull-up reads low when pressed, and many relay boards energise when driven low.
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
