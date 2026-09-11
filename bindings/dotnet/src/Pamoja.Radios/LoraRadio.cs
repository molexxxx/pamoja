using Pamoja.Lora;
using Pamoja.Native.Interop;

using NativeStatus = Pamoja.Native.Interop.Status;

namespace Pamoja.Radios;

/// <summary>Which family a radio's chip belongs to.</summary>
public enum LoraRadioFamily
{
    /// <summary>The SX1261, SX1262, SX1268, and LLCC68, which take commands.</summary>
    Sx126x,

    /// <summary>The SX1276, SX1277, SX1278, and SX1279, which take registers.</summary>
    Sx127x,
}

/// <summary>How a reception ended.</summary>
public enum LoraReceptionOutcome
{
    /// <summary>A frame arrived and checked.</summary>
    Frame,

    /// <summary>No frame arrived before the timeout.</summary>
    Timeout,

    /// <summary>A frame arrived whose header or CRC failed its check, and was dropped.</summary>
    Corrupt,

    /// <summary>Nothing has arrived since the last frame was taken.</summary>
    Nothing,
}

/// <summary>Where a radio module is wired on a Linux board.</summary>
/// <param name="Spi">The SPI device file, one per chip select, such as <c>/dev/spidev0.0</c>.</param>
/// <param name="GpioChip">The GPIO chip the lines are on, <c>/dev/gpiochip0</c> for a Raspberry Pi's header.</param>
/// <param name="ResetLine">The line the reset pin is on, the BCM GPIO number on a Raspberry Pi.</param>
public sealed record LoraRadioWiring(string Spi, string GpioChip, uint ResetLine)
{
    /// <summary>The line an SX126x's BUSY pin is on. The SX127x has no BUSY pin.</summary>
    public uint? BusyLine { get; init; }

    /// <summary>The SPI clock in hertz, 2 MHz when left null.</summary>
    public uint? SpiHz { get; init; }
}

/// <summary>How a module wires its SX126x.</summary>
/// <param name="Amplifier">The chip's power amplifier.</param>
public sealed record Sx126xBoard(Sx126xAmplifier Amplifier)
{
    /// <summary>The voltage DIO3 supplies a TCXO with, such as 1.7, when a TCXO clocks the chip.</summary>
    public double? TcxoVolts { get; init; }

    /// <summary>How long the TCXO takes to settle, in microseconds.</summary>
    public uint TcxoSettleMicros { get; init; } = 5_000;

    /// <summary>Whether DIO2 drives the antenna switch.</summary>
    public bool Dio2RfSwitch { get; init; }

    /// <summary>Whether the module fits the inductor the DC-DC regulator needs.</summary>
    public bool DcDc { get; init; }

    /// <summary>Whether the chip is an LLCC68, which is held to the rates it supports.</summary>
    public bool Llcc68 { get; init; }
}

/// <summary>How a module wires its SX127x.</summary>
/// <param name="Output">The amplifier output the antenna is on, PA_BOOST on an RFM95W.</param>
public sealed record Sx127xBoard(Sx127xPaOutput Output)
{
    /// <summary>Whether a TCXO drives the XTA pin instead of a crystal.</summary>
    public bool Tcxo { get; init; }
}

/// <summary>What a radio sends and listens with.</summary>
/// <param name="FrequencyHz">The carrier frequency in hertz.</param>
/// <param name="Link">The spreading factor, bandwidth, coding rate, preamble, header, and CRC.</param>
/// <param name="OutputDbm">The output power asked of the amplifier, in dBm, clamped to its range.</param>
public sealed record LoraRadioConfig(uint FrequencyHz, LoraLink Link, sbyte OutputDbm)
{
    /// <summary>
    /// The sync word byte: 0x34 for a public network such as LoRaWAN, and 0x12, the default,
    /// for a private one.
    /// </summary>
    public byte SyncWord { get; init; } = NativeMethods.LoraRadioSyncWordPrivate;

    /// <summary>
    /// The lower edge of the band an SX126x calibrates its receiver for, with
    /// <see cref="BandHighHz"/>; the carrier alone when either is null.
    /// </summary>
    public uint? BandLowHz { get; init; }

    /// <summary>The upper edge of that band in hertz.</summary>
    public uint? BandHighHz { get; init; }

    /// <summary>Whether frames go out with inverted IQ, as a LoRaWAN gateway sends downlinks.</summary>
    public bool InvertIqTransmit { get; init; }

    /// <summary>Whether frames are expected with inverted IQ, as a LoRaWAN device hears downlinks.</summary>
    public bool InvertIqReceive { get; init; }
}

/// <summary>How a reception ended, with the frame and its signal levels when one arrived.</summary>
/// <param name="Outcome">How the reception ended.</param>
/// <param name="Payload">The frame's payload, or <c>null</c> without a frame.</param>
/// <param name="RssiDbm">The received signal strength averaged over the frame, in dBm.</param>
/// <param name="SnrDb">The estimated signal-to-noise ratio, in dB.</param>
/// <param name="SignalRssiDbm">The estimated strength of the LoRa signal itself, in dBm.</param>
public sealed record LoraReception(
    LoraReceptionOutcome Outcome,
    byte[]? Payload,
    double? RssiDbm,
    double? SnrDb,
    double? SignalRssiDbm);

/// <summary>A LoRa radio opened on a Linux board.</summary>
/// <remarks>
/// <para>
/// The radio is reached through the kernel's spidev and GPIO character devices, so it opens
/// only on Linux; every other platform throws <see cref="PlatformNotSupportedException"/>.
/// </para>
/// <para>
/// Every call waits on the chip, a transmission for its airtime and a reception for its
/// timeout, so a program with a user interface makes these calls on a worker thread. One
/// thread at a time uses a radio.
/// </para>
/// </remarks>
public sealed class LoraRadio : IDisposable
{
    /// <summary>The most bytes one LoRa frame carries.</summary>
    private const int FrameMax = 255;

    private readonly NativeHandle _handle;

    private LoraRadio(IntPtr radio)
    {
        _handle = new NativeHandle(radio, NativeMethods.pamoja_lora_radio_free);
        Family = NativeMethods.pamoja_lora_radio_family(radio) == NativeMethods.LoraRadioSx126x
            ? LoraRadioFamily.Sx126x
            : LoraRadioFamily.Sx127x;
    }

    /// <summary>The family of the radio's chip.</summary>
    public LoraRadioFamily Family { get; }

    /// <summary>Opens an SX1261, SX1262, SX1268, or LLCC68 module and resets it.</summary>
    /// <param name="wiring">The SPI device and the lines, which must name the BUSY line.</param>
    /// <param name="board">How the module wires the chip.</param>
    /// <returns>The radio, reset and in standby, ready for <see cref="Configure"/>.</returns>
    /// <exception cref="PlatformNotSupportedException">The platform is not Linux.</exception>
    /// <exception cref="ArgumentOutOfRangeException">DIO3 cannot supply the TCXO voltage.</exception>
    /// <exception cref="PamojaException">A device could not be opened, or no chip answered.</exception>
    public static LoraRadio OpenSx126x(LoraRadioWiring wiring, Sx126xBoard board)
    {
        ArgumentNullException.ThrowIfNull(wiring);
        ArgumentNullException.ThrowIfNull(board);

        var native = new PamojaSx126xBoard
        {
            HighPower = Flag(board.Amplifier == Sx126xAmplifier.HighPower),
            Tcxo = Flag(board.TcxoVolts is not null),
            TcxoVoltage = board.TcxoVolts is double volts ? TcxoVoltageCode(volts) : (byte)0,
            Dio2RfSwitch = Flag(board.Dio2RfSwitch),
            DcDc = Flag(board.DcDc),
            Llcc68 = Flag(board.Llcc68),
            TcxoSettleUs = board.TcxoSettleMicros,
        };
        PamojaStatus status = NativeMethods.pamoja_lora_radio_open_sx126x(
            wiring.Spi,
            wiring.SpiHz ?? 0,
            wiring.GpioChip,
            wiring.BusyLine ?? uint.MaxValue,
            wiring.ResetLine,
            native,
            out IntPtr radio);
        return Opened(status, radio);
    }

    /// <summary>
    /// Opens an SX1276, SX1277, SX1278, or SX1279 module, such as an RFM95W, and resets it
    /// into LoRa mode.
    /// </summary>
    /// <param name="wiring">The SPI device and the reset line.</param>
    /// <param name="board">How the module wires the chip.</param>
    /// <returns>The radio, reset and in standby, ready for <see cref="Configure"/>.</returns>
    /// <exception cref="PlatformNotSupportedException">The platform is not Linux.</exception>
    /// <exception cref="PamojaException">A device could not be opened, or no chip answered.</exception>
    public static LoraRadio OpenSx127x(LoraRadioWiring wiring, Sx127xBoard board)
    {
        ArgumentNullException.ThrowIfNull(wiring);
        ArgumentNullException.ThrowIfNull(board);

        PamojaStatus status = NativeMethods.pamoja_lora_radio_open_sx127x(
            wiring.Spi,
            wiring.SpiHz ?? 0,
            wiring.GpioChip,
            wiring.ResetLine,
            board.Output == Sx127xPaOutput.PaBoost,
            board.Tcxo,
            out IntPtr radio);
        return Opened(status, radio);
    }

    /// <summary>Tunes the radio to a configuration.</summary>
    /// <param name="config">The carrier, link, power, sync word, and IQ polarity.</param>
    /// <exception cref="PamojaException">The chip cannot use the settings, or did not answer.</exception>
    public void Configure(LoraRadioConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);

        var native = new PamojaLoraRadioConfig
        {
            FrequencyHz = config.FrequencyHz,
            BandLowHz = config.BandLowHz ?? 0,
            BandHighHz = config.BandHighHz ?? 0,
            Link = NativeLora.Link(config.Link),
            OutputDbm = config.OutputDbm,
            SyncWord = config.SyncWord,
            InvertIqTransmit = Flag(config.InvertIqTransmit),
            InvertIqReceive = Flag(config.InvertIqReceive),
        };
        NativeStatus.ThrowIfError(
            _handle.Use(handle => NativeMethods.pamoja_lora_radio_configure(handle, native)));
    }

    /// <summary>Sends one frame and waits for it to leave.</summary>
    /// <param name="payload">The payload, 1 to 255 bytes.</param>
    /// <returns>The frame's time on air, in microseconds.</returns>
    /// <exception cref="PamojaException">The radio is unconfigured, or the chip did not answer.</exception>
    public ulong Transmit(ReadOnlySpan<byte> payload)
    {
        bool added = false;
        try
        {
            _handle.DangerousAddRef(ref added);
            PamojaStatus status = NativeMethods.pamoja_lora_radio_transmit(
                _handle.DangerousGetHandle(), payload, (nuint)payload.Length, out ulong airtimeUs);
            NativeStatus.ThrowIfError(status);
            return airtimeUs;
        }
        finally
        {
            if (added)
            {
                _handle.DangerousRelease();
            }
        }
    }

    /// <summary>Listens for one frame.</summary>
    /// <param name="timeout">How long to listen for a frame to start.</param>
    /// <returns>The frame and its levels, or that the timeout passed or the frame was corrupt.</returns>
    /// <exception cref="PamojaException">The radio is unconfigured, or the chip did not answer.</exception>
    public LoraReception Receive(TimeSpan timeout)
    {
        byte[] buffer = new byte[FrameMax];
        PamojaLoraRadioReception reception = default;
        NativeStatus.ThrowIfError(Listening(
            buffer, (ulong)Math.Max(timeout.TotalMicroseconds, 0), ref reception));
        return Heard(buffer, reception);
    }

    /// <summary>Starts listening, frame after frame, until another call changes the mode.</summary>
    /// <exception cref="PamojaException">The radio is unconfigured, or the chip did not answer.</exception>
    public void Listen() =>
        NativeStatus.ThrowIfError(_handle.Use(NativeMethods.pamoja_lora_radio_listen));

    /// <summary>Takes the frame a listening radio has received, if one has arrived.</summary>
    /// <returns>
    /// The frame, a corrupt frame, or <see cref="LoraReceptionOutcome.Nothing"/> when nothing
    /// has arrived.
    /// </returns>
    /// <exception cref="PamojaException">The chip did not answer.</exception>
    public LoraReception TakeFrame()
    {
        byte[] buffer = new byte[FrameMax];
        PamojaLoraRadioReception reception = default;
        bool added = false;
        try
        {
            _handle.DangerousAddRef(ref added);
            NativeStatus.ThrowIfError(NativeMethods.pamoja_lora_radio_take_frame(
                _handle.DangerousGetHandle(), buffer, (nuint)buffer.Length, out reception));
        }
        finally
        {
            if (added)
            {
                _handle.DangerousRelease();
            }
        }

        return Heard(buffer, reception);
    }

    /// <summary>Puts the radio in standby, which stops a transmission or a reception.</summary>
    /// <exception cref="PamojaException">The chip did not answer.</exception>
    public void Standby() =>
        NativeStatus.ThrowIfError(_handle.Use(NativeMethods.pamoja_lora_radio_standby));

    /// <summary>
    /// Puts the radio to sleep until the next call wakes it. An SX126x is configured again
    /// before its next frame; an SX127x keeps its registers.
    /// </summary>
    /// <exception cref="PamojaException">The chip did not answer.</exception>
    public void Sleep() =>
        NativeStatus.ThrowIfError(_handle.Use(NativeMethods.pamoja_lora_radio_sleep));

    /// <summary>Reads one register of the radio's chip.</summary>
    /// <param name="address">A 16-bit address on the SX126x, 0x00 to 0x7F on the SX127x.</param>
    /// <returns>The register's value.</returns>
    /// <exception cref="PamojaException">The address is past the chip's map, or it did not answer.</exception>
    public byte ReadRegister(ushort address)
    {
        byte value = 0;
        NativeStatus.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_lora_radio_read_register(handle, address, out value)));
        return value;
    }

    /// <summary>Writes one register of the radio's chip.</summary>
    /// <param name="address">A 16-bit address on the SX126x, 0x00 to 0x7F on the SX127x.</param>
    /// <param name="value">The value to write.</param>
    /// <exception cref="PamojaException">The address is past the chip's map, or it did not answer.</exception>
    public void WriteRegister(ushort address, byte value) =>
        NativeStatus.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_lora_radio_write_register(handle, address, value)));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Wraps an opened radio, or turns why it did not open into an exception.</summary>
    /// <param name="status">What the native open call returned.</param>
    /// <param name="radio">The radio it opened, when it did.</param>
    /// <returns>The radio.</returns>
    private static LoraRadio Opened(PamojaStatus status, IntPtr radio)
    {
        if (status == PamojaStatus.Unsupported)
        {
            throw new PlatformNotSupportedException(
                NativeStatus.LastError() ?? "a LoRa radio is opened only on Linux");
        }

        NativeStatus.ThrowIfError(status);
        return new LoraRadio(radio);
    }

    /// <summary>Runs the native receive call over a buffer.</summary>
    /// <param name="buffer">Where the payload goes.</param>
    /// <param name="timeoutUs">How long to listen, in microseconds.</param>
    /// <param name="reception">Receives how the reception ended.</param>
    /// <returns>What the native call returned.</returns>
    private PamojaStatus Listening(byte[] buffer, ulong timeoutUs, ref PamojaLoraRadioReception reception)
    {
        bool added = false;
        try
        {
            _handle.DangerousAddRef(ref added);
            PamojaStatus status = NativeMethods.pamoja_lora_radio_receive(
                _handle.DangerousGetHandle(),
                buffer,
                (nuint)buffer.Length,
                timeoutUs,
                out PamojaLoraRadioReception heard);
            reception = heard;
            return status;
        }
        finally
        {
            if (added)
            {
                _handle.DangerousRelease();
            }
        }
    }

    /// <summary>Reads how a reception ended, copying out the payload of a frame.</summary>
    /// <param name="buffer">The buffer the payload went into.</param>
    /// <param name="reception">What the native call reported.</param>
    /// <returns>The reception.</returns>
    private static LoraReception Heard(byte[] buffer, PamojaLoraRadioReception reception)
    {
        LoraReceptionOutcome outcome = reception.Outcome switch
        {
            NativeMethods.LoraRadioFrame => LoraReceptionOutcome.Frame,
            NativeMethods.LoraRadioTimeout => LoraReceptionOutcome.Timeout,
            NativeMethods.LoraRadioCorrupt => LoraReceptionOutcome.Corrupt,
            _ => LoraReceptionOutcome.Nothing,
        };
        if (outcome != LoraReceptionOutcome.Frame)
        {
            return new LoraReception(outcome, null, null, null, null);
        }

        return new LoraReception(
            outcome,
            buffer[..(int)reception.Len],
            NativeLora.Db(reception.RssiCentiDbm),
            NativeLora.Db(reception.SnrCentiDb),
            NativeLora.Db(reception.SignalRssiCentiDbm));
    }

    /// <summary>Describes a flag the way the C ABI carries it.</summary>
    /// <param name="set">Whether the flag is set.</param>
    /// <returns><c>1</c> when set, <c>0</c> otherwise.</returns>
    private static byte Flag(bool set) => set ? (byte)1 : (byte)0;

    /// <summary>Names the TCXO voltage code a number of volts selects.</summary>
    /// <param name="volts">The supply voltage, such as 1.7.</param>
    /// <returns>The code SetDIO3AsTCXOCtrl takes.</returns>
    /// <exception cref="ArgumentOutOfRangeException">DIO3 cannot supply that voltage.</exception>
    private static byte TcxoVoltageCode(double volts) => (int)Math.Round(volts * 1000) switch
    {
        1600 => 0,
        1700 => 1,
        1800 => 2,
        2200 => 3,
        2400 => 4,
        2700 => 5,
        3000 => 6,
        3300 => 7,
        _ => throw new ArgumentOutOfRangeException(
            nameof(volts),
            volts,
            "DIO3 supplies a TCXO with 1.6, 1.7, 1.8, 2.2, 2.4, 2.7, 3.0, or 3.3 V"),
    };
}
