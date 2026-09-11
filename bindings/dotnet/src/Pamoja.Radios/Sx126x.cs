using Pamoja.Lora;
using Pamoja.Native.Interop;

using NativeStatus = Pamoja.Native.Interop.Status;

namespace Pamoja.Radios;

/// <summary>Which power amplifier an SX126x chip carries.</summary>
public enum Sx126xAmplifier
{
    /// <summary>The SX1261 amplifier, up to 15 dBm.</summary>
    LowPower,

    /// <summary>The SX1262, SX1268, and LLCC68 amplifier, up to 22 dBm.</summary>
    HighPower,
}

/// <summary>The interrupts an SX126x raises, as GetIrqStatus reports them.</summary>
[Flags]
public enum Sx126xIrq : ushort
{
    /// <summary>No interrupt.</summary>
    None = 0,

    /// <summary>A frame has been sent.</summary>
    TxDone = NativeMethods.Sx126xIrqTxDone,

    /// <summary>A frame has been received.</summary>
    RxDone = NativeMethods.Sx126xIrqRxDone,

    /// <summary>A preamble was detected.</summary>
    PreambleDetected = NativeMethods.Sx126xIrqPreambleDetected,

    /// <summary>A valid sync word was detected.</summary>
    SyncWordValid = NativeMethods.Sx126xIrqSyncWordValid,

    /// <summary>A valid LoRa header was received.</summary>
    HeaderValid = NativeMethods.Sx126xIrqHeaderValid,

    /// <summary>A LoRa header failed its CRC.</summary>
    HeaderError = NativeMethods.Sx126xIrqHeaderError,

    /// <summary>A received payload failed its CRC.</summary>
    CrcError = NativeMethods.Sx126xIrqCrcError,

    /// <summary>Channel activity detection finished.</summary>
    CadDone = NativeMethods.Sx126xIrqCadDone,

    /// <summary>Channel activity detection heard a signal.</summary>
    CadDetected = NativeMethods.Sx126xIrqCadDetected,

    /// <summary>A transmit or receive timeout expired.</summary>
    Timeout = NativeMethods.Sx126xIrqTimeout,

    /// <summary>An LR-FHSS frequency hop is due.</summary>
    LrFhssHop = NativeMethods.Sx126xIrqLrFhssHop,

    /// <summary>Every interrupt the chip defines.</summary>
    All = NativeMethods.Sx126xIrqAll,
}

/// <summary>The faults GetDeviceErrors reports.</summary>
[Flags]
public enum Sx126xDeviceErrors : ushort
{
    /// <summary>No fault.</summary>
    None = 0,

    /// <summary>The RC64k calibration failed.</summary>
    Rc64kCalibration = NativeMethods.Sx126xErrorRc64kCalibration,

    /// <summary>The RC13M calibration failed.</summary>
    Rc13mCalibration = NativeMethods.Sx126xErrorRc13mCalibration,

    /// <summary>The PLL calibration failed.</summary>
    PllCalibration = NativeMethods.Sx126xErrorPllCalibration,

    /// <summary>The ADC calibration failed.</summary>
    AdcCalibration = NativeMethods.Sx126xErrorAdcCalibration,

    /// <summary>The image calibration failed.</summary>
    ImageCalibration = NativeMethods.Sx126xErrorImageCalibration,

    /// <summary>The crystal oscillator failed to start.</summary>
    XoscStart = NativeMethods.Sx126xErrorXoscStart,

    /// <summary>The PLL failed to lock.</summary>
    PllLock = NativeMethods.Sx126xErrorPllLock,

    /// <summary>The power-amplifier ramp failed.</summary>
    PaRamp = NativeMethods.Sx126xErrorPaRamp,
}

/// <summary>The operating mode a status byte reports.</summary>
public enum Sx126xChipMode : byte
{
    /// <summary>A value the datasheet reserves.</summary>
    Other = 0,

    /// <summary>Standby on the RC oscillator.</summary>
    StandbyRc = 2,

    /// <summary>Standby on the crystal oscillator.</summary>
    StandbyXosc = 3,

    /// <summary>Frequency synthesis.</summary>
    Fs = 4,

    /// <summary>Receiving.</summary>
    Rx = 5,

    /// <summary>Transmitting.</summary>
    Tx = 6,
}

/// <summary>The outcome of the last command, as a status byte reports it.</summary>
public enum Sx126xCommandStatus : byte
{
    /// <summary>A value the datasheet reserves.</summary>
    Other = 0,

    /// <summary>Data is available to the host.</summary>
    DataAvailable = 2,

    /// <summary>The command timed out.</summary>
    Timeout = 3,

    /// <summary>The chip could not process the command.</summary>
    ProcessingError = 4,

    /// <summary>The chip failed to execute the command.</summary>
    ExecutionFailure = 5,

    /// <summary>A transmission finished.</summary>
    TxDone = 6,
}

/// <summary>An amplifier setting: the SetPaConfig bytes and the SetTxParams power.</summary>
/// <param name="PaDutyCycle">The SetPaConfig <c>paDutyCycle</c> byte.</param>
/// <param name="HpMax">The SetPaConfig <c>hpMax</c> byte.</param>
/// <param name="DeviceSel">The SetPaConfig <c>deviceSel</c> byte.</param>
/// <param name="PaLut">The SetPaConfig <c>paLut</c> byte.</param>
/// <param name="SettingDbm">The SetTxParams power, in dBm.</param>
public readonly record struct Sx126xTxPower(
    byte PaDutyCycle,
    byte HpMax,
    byte DeviceSel,
    byte PaLut,
    sbyte SettingDbm);

/// <summary>A command that reads data back, and how much it reads.</summary>
/// <param name="Bytes">The bytes to clock out before the answer, opcode first.</param>
/// <param name="AnswerLength">How many bytes of answer follow them in the same transaction.</param>
public sealed record Sx126xQuery(byte[] Bytes, int AnswerLength);

/// <summary>A decoded status byte.</summary>
/// <param name="ChipMode">The operating mode.</param>
/// <param name="CommandStatus">The outcome of the last command.</param>
/// <param name="Error">
/// Whether that outcome is a failure: a watchdog timeout, a processing error, or an
/// execution failure.
/// </param>
public readonly record struct Sx126xStatus(
    Sx126xChipMode ChipMode,
    Sx126xCommandStatus CommandStatus,
    bool Error);

/// <summary>The signal levels a LoRa frame was received at.</summary>
/// <param name="RssiDbm">The average RSSI over the frame, in dBm.</param>
/// <param name="SnrDb">The estimated signal-to-noise ratio, in dB.</param>
/// <param name="SignalRssiDbm">The RSSI of the despread LoRa signal, in dBm.</param>
public readonly record struct Sx126xPacketStatus(double RssiDbm, double SnrDb, double SignalRssiDbm);

/// <summary>Where the last received payload sits in the chip's buffer.</summary>
/// <param name="PayloadLength">The payload length in bytes.</param>
/// <param name="Start">The buffer offset the payload starts at.</param>
public readonly record struct Sx126xRxBufferStatus(byte PayloadLength, byte Start);

/// <summary>The Semtech SX1261, SX1262, SX1268, and LLCC68 LoRa transceivers.</summary>
/// <remarks>
/// Every command returns the bytes of one SPI transaction, which the caller sends once
/// the chip's BUSY line is low, and every decoder turns the bytes the chip answered with
/// into values. Nothing here touches a bus, so the same code plans a transmission on a
/// host with no radio attached and checks a capture taken from one that has.
/// </remarks>
public static class Sx126x
{
    /// <summary>The SetRx timeout that keeps the receiver listening until told otherwise.</summary>
    public const uint RxContinuous = NativeMethods.Sx126xRxContinuous;

    /// <summary>The LoRa sync word public LoRaWAN networks use.</summary>
    public const ushort SyncWordPublic = NativeMethods.Sx126xSyncWordPublic;

    /// <summary>The LoRa sync word private networks use.</summary>
    public const ushort SyncWordPrivate = NativeMethods.Sx126xSyncWordPrivate;

    /// <summary>The register the two bytes of the LoRa sync word are written to.</summary>
    public const ushort RegisterLoraSyncWord = NativeMethods.Sx126xRegisterLoraSyncWord;

    /// <summary>Returns the RF frequency word SetRfFrequency carries.</summary>
    /// <param name="frequencyHz">The carrier frequency in hertz.</param>
    /// <returns>The frequency in steps of 32 MHz over 2^25.</returns>
    public static uint FrequencyWord(uint frequencyHz) =>
        NativeMethods.pamoja_sx126x_frequency_word(frequencyHz);

    /// <summary>Returns a timeout in the chip's steps.</summary>
    /// <param name="timeoutMicros">The timeout in microseconds.</param>
    /// <returns>The timeout in 15.625 microsecond steps, rounded up.</returns>
    public static uint TimeoutSteps(ulong timeoutMicros) =>
        NativeMethods.pamoja_sx126x_timeout_steps(timeoutMicros);

    /// <summary>Returns the CalibrateImage codes for the band a radio operates in.</summary>
    /// <param name="lowHz">The lowest frequency the radio uses, in hertz.</param>
    /// <param name="highHz">The highest frequency the radio uses, in hertz.</param>
    /// <returns>The two calibration codes.</returns>
    public static byte[] ImageCalibration(uint lowHz, uint highHz)
    {
        ushort codes = NativeMethods.pamoja_sx126x_image_calibration(lowHz, highHz);
        return [(byte)(codes >> 8), (byte)codes];
    }

    /// <summary>Returns the shortest ramp time the chip offers that lasts at least a duration.</summary>
    /// <param name="atLeastMicros">
    /// The least ramp time wanted, in microseconds; past 3400, the longest ramp there is.
    /// </param>
    /// <returns>The ramp time in microseconds.</returns>
    public static uint RampTimeMicros(uint atLeastMicros) =>
        NativeMethods.pamoja_sx126x_ramp_time_us(atLeastMicros);

    /// <summary>Returns the amplifier setting that delivers an output power.</summary>
    /// <param name="amplifier">The amplifier the chip carries.</param>
    /// <param name="outputDbm">The power wanted at the antenna port, in dBm.</param>
    /// <returns>The setting, clamped to what the amplifier can deliver.</returns>
    public static Sx126xTxPower TxPower(Sx126xAmplifier amplifier, sbyte outputDbm) =>
        FromNative(NativeMethods.pamoja_sx126x_tx_power_for_output(
            amplifier == Sx126xAmplifier.HighPower, outputDbm));

    /// <summary>Returns the strongest amplifier setting that keeps a link under an EIRP ceiling.</summary>
    /// <param name="amplifier">The amplifier the chip carries.</param>
    /// <param name="budget">The antenna gain and cable loss between the chip and the air.</param>
    /// <param name="eirpCeilingDbm">The most EIRP allowed, such as a plan's limit for the channel.</param>
    /// <returns>The setting, in whole decibels rounded down so the ceiling holds.</returns>
    public static Sx126xTxPower TxPowerUnderCeiling(
        Sx126xAmplifier amplifier,
        LoraLinkBudget budget,
        double eirpCeilingDbm)
    {
        ArgumentNullException.ThrowIfNull(budget);

        return FromNative(NativeMethods.pamoja_sx126x_tx_power_under_ceiling(
            amplifier == Sx126xAmplifier.HighPower,
            NativeLora.Budget(budget),
            NativeLora.Centi(eirpCeilingDbm)));
    }

    /// <summary>Builds SetStandby into the RC oscillator standby.</summary>
    /// <returns>The command bytes.</returns>
    public static byte[] SetStandby() => NativeMethods.pamoja_sx126x_set_standby().ToArray();

    /// <summary>Builds SetPacketType for LoRa.</summary>
    /// <returns>The command bytes.</returns>
    public static byte[] SetPacketTypeLora() =>
        NativeMethods.pamoja_sx126x_set_packet_type_lora().ToArray();

    /// <summary>Builds SetRfFrequency.</summary>
    /// <param name="frequencyHz">The carrier frequency in hertz.</param>
    /// <returns>The command bytes.</returns>
    public static byte[] SetRfFrequency(uint frequencyHz) =>
        NativeMethods.pamoja_sx126x_set_rf_frequency(frequencyHz).ToArray();

    /// <summary>Builds CalibrateImage for the band a radio operates in.</summary>
    /// <param name="lowHz">The lowest frequency the radio uses, in hertz.</param>
    /// <param name="highHz">The highest frequency the radio uses, in hertz.</param>
    /// <returns>The command bytes.</returns>
    public static byte[] CalibrateImage(uint lowHz, uint highHz) =>
        NativeMethods.pamoja_sx126x_calibrate_image(lowHz, highHz).ToArray();

    /// <summary>Builds SetPaConfig.</summary>
    /// <param name="power">The amplifier setting.</param>
    /// <returns>The command bytes.</returns>
    public static byte[] SetPaConfig(Sx126xTxPower power) =>
        NativeMethods.pamoja_sx126x_set_pa_config(ToNative(power)).ToArray();

    /// <summary>Builds SetTxParams.</summary>
    /// <param name="power">The amplifier setting.</param>
    /// <param name="rampMicros">The least ramp time wanted, in microseconds.</param>
    /// <returns>The command bytes.</returns>
    public static byte[] SetTxParams(Sx126xTxPower power, uint rampMicros) =>
        NativeMethods.pamoja_sx126x_set_tx_params(ToNative(power), rampMicros).ToArray();

    /// <summary>Builds SetModulationParams for a LoRa link.</summary>
    /// <param name="link">The link settings.</param>
    /// <returns>The command bytes, with low data-rate optimization on where the datasheet asks for it.</returns>
    /// <exception cref="PamojaException">The chip has no such bandwidth.</exception>
    public static byte[] SetLoraModulationParams(LoraLink link)
    {
        ArgumentNullException.ThrowIfNull(link);

        NativeStatus.ThrowIfError(NativeMethods.pamoja_sx126x_set_lora_modulation_params(
            NativeLora.Link(link), out PamojaSx126xCommand command));
        return command.ToArray();
    }

    /// <summary>Builds SetPacketParams for a LoRa link.</summary>
    /// <param name="link">The link settings.</param>
    /// <param name="payloadLength">The payload length in bytes.</param>
    /// <param name="invertIq">Whether to invert the IQ signals, as a LoRaWAN downlink does.</param>
    /// <returns>The command bytes.</returns>
    public static byte[] SetLoraPacketParams(LoraLink link, byte payloadLength, bool invertIq)
    {
        ArgumentNullException.ThrowIfNull(link);

        return NativeMethods.pamoja_sx126x_set_lora_packet_params(
            NativeLora.Link(link), payloadLength, invertIq).ToArray();
    }

    /// <summary>Builds SetDioIrqParams.</summary>
    /// <param name="irq">The interrupts the chip raises at all.</param>
    /// <param name="dio1">The interrupts that drive DIO1.</param>
    /// <param name="dio2">The interrupts that drive DIO2.</param>
    /// <param name="dio3">The interrupts that drive DIO3.</param>
    /// <returns>The command bytes.</returns>
    public static byte[] SetDioIrqParams(
        Sx126xIrq irq,
        Sx126xIrq dio1,
        Sx126xIrq dio2 = Sx126xIrq.None,
        Sx126xIrq dio3 = Sx126xIrq.None) =>
        NativeMethods.pamoja_sx126x_set_dio_irq_params(
            (ushort)irq, (ushort)dio1, (ushort)dio2, (ushort)dio3).ToArray();

    /// <summary>Builds ClearIrqStatus.</summary>
    /// <param name="irq">The interrupts to clear.</param>
    /// <returns>The command bytes.</returns>
    public static byte[] ClearIrqStatus(Sx126xIrq irq) =>
        NativeMethods.pamoja_sx126x_clear_irq_status((ushort)irq).ToArray();

    /// <summary>Builds SetTx.</summary>
    /// <param name="timeoutMicros">
    /// How long the chip may take to send the frame before it gives up, in microseconds;
    /// <c>0</c> waits for the frame however long it takes.
    /// </param>
    /// <returns>The command bytes.</returns>
    public static byte[] SetTx(ulong timeoutMicros) =>
        NativeMethods.pamoja_sx126x_set_tx(timeoutMicros).ToArray();

    /// <summary>Builds SetRx.</summary>
    /// <param name="timeoutMicros">
    /// How long to listen for a frame, in microseconds; <c>0</c> listens for one frame
    /// however long it takes.
    /// </param>
    /// <returns>The command bytes.</returns>
    public static byte[] SetRx(ulong timeoutMicros) =>
        NativeMethods.pamoja_sx126x_set_rx(timeoutMicros).ToArray();

    /// <summary>Builds SetRx that keeps listening, frame after frame, until told otherwise.</summary>
    /// <returns>The command bytes.</returns>
    public static byte[] SetRxContinuous() =>
        NativeMethods.pamoja_sx126x_set_rx_continuous().ToArray();

    /// <summary>Builds SetSleep.</summary>
    /// <param name="warmStart">Whether to keep the configuration through sleep.</param>
    /// <returns>The command bytes.</returns>
    public static byte[] SetSleep(bool warmStart) =>
        NativeMethods.pamoja_sx126x_set_sleep(warmStart).ToArray();

    /// <summary>Builds WriteRegister for a run of registers.</summary>
    /// <param name="address">The first register.</param>
    /// <param name="values">The values, one per register.</param>
    /// <returns>The whole transaction: the header followed by the values.</returns>
    public static byte[] WriteRegister(ushort address, ReadOnlySpan<byte> values) =>
        [.. NativeMethods.pamoja_sx126x_write_register_header(address).ToArray(), .. values];

    /// <summary>Builds WriteBuffer for a payload.</summary>
    /// <param name="offset">The buffer offset the payload starts at.</param>
    /// <param name="payload">The payload.</param>
    /// <returns>The whole transaction: the header followed by the payload.</returns>
    public static byte[] WriteBuffer(byte offset, ReadOnlySpan<byte> payload) =>
        [.. NativeMethods.pamoja_sx126x_write_buffer_header(offset).ToArray(), .. payload];

    /// <summary>Builds GetStatus.</summary>
    /// <returns>The query.</returns>
    public static Sx126xQuery GetStatus() => QueryOf(NativeMethods.pamoja_sx126x_get_status());

    /// <summary>Builds GetIrqStatus.</summary>
    /// <returns>The query, whose answer <see cref="Irq"/> decodes.</returns>
    public static Sx126xQuery GetIrqStatus() =>
        QueryOf(NativeMethods.pamoja_sx126x_get_irq_status());

    /// <summary>Builds GetRxBufferStatus.</summary>
    /// <returns>The query, whose answer <see cref="RxBufferStatus"/> decodes.</returns>
    public static Sx126xQuery GetRxBufferStatus() =>
        QueryOf(NativeMethods.pamoja_sx126x_get_rx_buffer_status());

    /// <summary>Builds GetPacketStatus.</summary>
    /// <returns>The query, whose answer <see cref="PacketStatus"/> decodes.</returns>
    public static Sx126xQuery GetPacketStatus() =>
        QueryOf(NativeMethods.pamoja_sx126x_get_packet_status());

    /// <summary>Builds GetRssiInst.</summary>
    /// <returns>The query, whose answer <see cref="RssiInstDbm"/> decodes.</returns>
    public static Sx126xQuery GetRssiInst() =>
        QueryOf(NativeMethods.pamoja_sx126x_get_rssi_inst());

    /// <summary>Builds GetDeviceErrors.</summary>
    /// <returns>The query, whose answer <see cref="DeviceErrors"/> decodes.</returns>
    public static Sx126xQuery GetDeviceErrors() =>
        QueryOf(NativeMethods.pamoja_sx126x_get_device_errors());

    /// <summary>Builds ReadRegister for a run of registers.</summary>
    /// <param name="address">The first register.</param>
    /// <param name="length">How many registers to read.</param>
    /// <returns>The query.</returns>
    public static Sx126xQuery ReadRegister(ushort address, byte length) =>
        QueryOf(NativeMethods.pamoja_sx126x_read_register(address, length));

    /// <summary>Builds ReadBuffer for a run of buffer bytes.</summary>
    /// <param name="offset">The buffer offset to read from.</param>
    /// <param name="length">How many bytes to read.</param>
    /// <returns>The query.</returns>
    public static Sx126xQuery ReadBuffer(byte offset, byte length) =>
        QueryOf(NativeMethods.pamoja_sx126x_read_buffer(offset, length));

    /// <summary>Decodes a status byte.</summary>
    /// <param name="value">The byte the chip returned.</param>
    /// <returns>The operating mode and the outcome of the last command.</returns>
    public static Sx126xStatus Status(byte value)
    {
        PamojaSx126xStatus status = NativeMethods.pamoja_sx126x_status_from_byte(value);
        return new Sx126xStatus(
            (Sx126xChipMode)status.ChipMode,
            (Sx126xCommandStatus)status.CommandStatus,
            status.Error != 0);
    }

    /// <summary>Decodes a GetIrqStatus answer.</summary>
    /// <param name="answer">The two answer bytes.</param>
    /// <returns>The interrupts raised.</returns>
    /// <exception cref="ArgumentException">The answer is not two bytes.</exception>
    public static Sx126xIrq Irq(ReadOnlySpan<byte> answer)
    {
        Exactly(answer, 2, "a GetIrqStatus answer");
        return (Sx126xIrq)NativeMethods.pamoja_sx126x_irq_from_bytes(answer[0], answer[1]);
    }

    /// <summary>Decodes a GetDeviceErrors answer.</summary>
    /// <param name="answer">The two answer bytes.</param>
    /// <returns>The faults reported.</returns>
    /// <exception cref="ArgumentException">The answer is not two bytes.</exception>
    public static Sx126xDeviceErrors DeviceErrors(ReadOnlySpan<byte> answer)
    {
        Exactly(answer, 2, "a GetDeviceErrors answer");
        return (Sx126xDeviceErrors)NativeMethods.pamoja_sx126x_device_errors_from_bytes(
            answer[0], answer[1]);
    }

    /// <summary>Decodes a GetPacketStatus answer.</summary>
    /// <param name="answer">The three answer bytes.</param>
    /// <returns>The signal levels the frame was received at.</returns>
    /// <exception cref="ArgumentException">The answer is not three bytes.</exception>
    public static Sx126xPacketStatus PacketStatus(ReadOnlySpan<byte> answer)
    {
        Exactly(answer, 3, "a GetPacketStatus answer");
        PamojaSx126xPacketStatus status = NativeMethods.pamoja_sx126x_packet_status_from_bytes(
            answer[0], answer[1], answer[2]);
        return new Sx126xPacketStatus(
            NativeLora.Db(status.RssiCentiDbm),
            NativeLora.Db(status.SnrCentiDb),
            NativeLora.Db(status.SignalRssiCentiDbm));
    }

    /// <summary>Decodes a GetRxBufferStatus answer.</summary>
    /// <param name="answer">The two answer bytes.</param>
    /// <returns>The payload length and where it starts.</returns>
    /// <exception cref="ArgumentException">The answer is not two bytes.</exception>
    public static Sx126xRxBufferStatus RxBufferStatus(ReadOnlySpan<byte> answer)
    {
        Exactly(answer, 2, "a GetRxBufferStatus answer");
        PamojaSx126xRxBufferStatus status =
            NativeMethods.pamoja_sx126x_rx_buffer_status_from_bytes(answer[0], answer[1]);
        return new Sx126xRxBufferStatus(status.PayloadLen, status.Start);
    }

    /// <summary>Decodes a GetRssiInst answer.</summary>
    /// <param name="value">The answer byte.</param>
    /// <returns>The signal level the receiver hears right now, in dBm.</returns>
    public static double RssiInstDbm(byte value) =>
        NativeLora.Db(NativeMethods.pamoja_sx126x_rssi_inst_centi_dbm(value));

    /// <summary>Reports whether an LLCC68 supports a link's spreading factor at its bandwidth.</summary>
    /// <param name="link">The link settings.</param>
    /// <returns>
    /// <c>true</c> when an LLCC68 can use the link: up to SF9 at 125 kHz, SF10 at 250 kHz, and
    /// SF11 at 500 kHz, and no bandwidth below 125 kHz.
    /// </returns>
    public static bool Llcc68Supports(LoraLink link)
    {
        ArgumentNullException.ThrowIfNull(link);

        return NativeMethods.pamoja_sx126x_llcc68_supports(NativeLora.Link(link));
    }

    /// <summary>Describes an amplifier setting from the C ABI.</summary>
    /// <param name="power">The setting as the C ABI carries it.</param>
    /// <returns>The setting.</returns>
    private static Sx126xTxPower FromNative(PamojaSx126xTxPower power) =>
        new(power.PaDutyCycle, power.HpMax, power.DeviceSel, power.PaLut, power.SettingDbm);

    /// <summary>Describes an amplifier setting the way the C ABI carries it.</summary>
    /// <param name="power">The setting.</param>
    /// <returns>The setting as the C ABI carries it.</returns>
    private static PamojaSx126xTxPower ToNative(Sx126xTxPower power) => new()
    {
        PaDutyCycle = power.PaDutyCycle,
        HpMax = power.HpMax,
        DeviceSel = power.DeviceSel,
        PaLut = power.PaLut,
        SettingDbm = power.SettingDbm,
    };

    /// <summary>Copies a query out of the C ABI.</summary>
    /// <param name="query">The query as the C ABI carries it.</param>
    /// <returns>The query.</returns>
    private static Sx126xQuery QueryOf(PamojaSx126xQuery query) =>
        new(query.Command.ToArray(), checked((int)query.AnswerLen));

    /// <summary>Refuses an answer of the wrong length.</summary>
    /// <param name="answer">The answer bytes.</param>
    /// <param name="length">The length the answer must be.</param>
    /// <param name="what">What the answer is, for the exception message.</param>
    /// <exception cref="ArgumentException">The answer is not <paramref name="length"/> bytes.</exception>
    private static void Exactly(ReadOnlySpan<byte> answer, int length, string what)
    {
        if (answer.Length != length)
        {
            throw new ArgumentException(
                $"{what} is {length} bytes, not {answer.Length}", nameof(answer));
        }
    }
}
