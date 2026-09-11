using Pamoja.Lora;
using Pamoja.Native.Interop;

using NativeStatus = Pamoja.Native.Interop.Status;

namespace Pamoja.Radios;

/// <summary>Which amplifier output an SX127x module wires to its antenna.</summary>
public enum Sx127xPaOutput
{
    /// <summary>The high efficiency amplifier on RFO_LF or RFO_HF, -4 to +15 dBm.</summary>
    Rfo,

    /// <summary>The regulated amplifier on PA_BOOST, +2 to +20 dBm, as on the RFM95W.</summary>
    PaBoost,
}

/// <summary>An SX127x operating mode, the Mode bits of RegOpMode.</summary>
public enum Sx127xMode : byte
{
    /// <summary>Only the SPI interface and the registers are powered; the only mode that may switch modems.</summary>
    Sleep = NativeMethods.Sx127xModeSleep,

    /// <summary>The oscillator and the baseband are on.</summary>
    Standby = NativeMethods.Sx127xModeStandby,

    /// <summary>The PLL is locked for transmit.</summary>
    FsTx = NativeMethods.Sx127xModeFsTx,

    /// <summary>One packet goes out, then the chip returns to standby.</summary>
    Tx = NativeMethods.Sx127xModeTx,

    /// <summary>The PLL is locked for receive.</summary>
    FsRx = NativeMethods.Sx127xModeFsRx,

    /// <summary>The receiver takes packet after packet.</summary>
    RxContinuous = NativeMethods.Sx127xModeRxContinuous,

    /// <summary>The receiver waits for one packet or the symbol timeout.</summary>
    RxSingle = NativeMethods.Sx127xModeRxSingle,

    /// <summary>Channel activity detection looks for a LoRa preamble.</summary>
    Cad = NativeMethods.Sx127xModeCad,
}

/// <summary>The LoRa interrupt flags of RegIrqFlags.</summary>
[Flags]
public enum Sx127xIrq : byte
{
    /// <summary>No interrupt.</summary>
    None = 0,

    /// <summary>Channel activity detection heard a LoRa signal.</summary>
    CadDetected = NativeMethods.Sx127xIrqCadDetected,

    /// <summary>Frequency hopping moved to the next channel.</summary>
    FhssChangeChannel = NativeMethods.Sx127xIrqFhssChangeChannel,

    /// <summary>Channel activity detection finished.</summary>
    CadDone = NativeMethods.Sx127xIrqCadDone,

    /// <summary>The payload has been transmitted.</summary>
    TxDone = NativeMethods.Sx127xIrqTxDone,

    /// <summary>A valid header was received.</summary>
    ValidHeader = NativeMethods.Sx127xIrqValidHeader,

    /// <summary>The payload failed its CRC.</summary>
    PayloadCrcError = NativeMethods.Sx127xIrqPayloadCrcError,

    /// <summary>A packet has been received.</summary>
    RxDone = NativeMethods.Sx127xIrqRxDone,

    /// <summary>A single reception timed out before a preamble arrived.</summary>
    RxTimeout = NativeMethods.Sx127xIrqRxTimeout,

    /// <summary>Every interrupt, which writing back clears.</summary>
    All = NativeMethods.Sx127xIrqAll,
}

/// <summary>The SX127x register addresses, from Table 41 of the datasheet with the LoRa page selected.</summary>
public static class Sx127xRegister
{
    /// <summary>RegFifo, the LoRa data buffer, read or written at RegFifoAddrPtr.</summary>
    public const byte Fifo = NativeMethods.Sx127xRegFifo;

    /// <summary>RegOpMode: LoRa or FSK, the register page, and the operating mode.</summary>
    public const byte OpMode = NativeMethods.Sx127xRegOpMode;

    /// <summary>RegFrfMsb, the top byte of the carrier word.</summary>
    public const byte FrfMsb = NativeMethods.Sx127xRegFrfMsb;

    /// <summary>RegFrfMid, the middle byte of the carrier word.</summary>
    public const byte FrfMid = NativeMethods.Sx127xRegFrfMid;

    /// <summary>RegFrfLsb, the low byte of the carrier word.</summary>
    public const byte FrfLsb = NativeMethods.Sx127xRegFrfLsb;

    /// <summary>RegPaConfig: the amplifier output, its maximum, and the power.</summary>
    public const byte PaConfig = NativeMethods.Sx127xRegPaConfig;

    /// <summary>RegPaRamp, the amplifier ramp time.</summary>
    public const byte PaRamp = NativeMethods.Sx127xRegPaRamp;

    /// <summary>RegOcp, the amplifier current limit.</summary>
    public const byte Ocp = NativeMethods.Sx127xRegOcp;

    /// <summary>RegLna, the LNA gain and current.</summary>
    public const byte Lna = NativeMethods.Sx127xRegLna;

    /// <summary>RegFifoAddrPtr, where the next RegFifo access lands.</summary>
    public const byte FifoAddrPtr = NativeMethods.Sx127xRegFifoAddrPtr;

    /// <summary>RegFifoTxBaseAddr, where a transmitted payload starts.</summary>
    public const byte FifoTxBaseAddr = NativeMethods.Sx127xRegFifoTxBaseAddr;

    /// <summary>RegFifoRxBaseAddr, where received payloads start.</summary>
    public const byte FifoRxBaseAddr = NativeMethods.Sx127xRegFifoRxBaseAddr;

    /// <summary>RegFifoRxCurrentAddr, where the last received packet starts.</summary>
    public const byte FifoRxCurrentAddr = NativeMethods.Sx127xRegFifoRxCurrentAddr;

    /// <summary>RegIrqFlagsMask, the interrupts masked off.</summary>
    public const byte IrqFlagsMask = NativeMethods.Sx127xRegIrqFlagsMask;

    /// <summary>RegIrqFlags, the interrupts raised, each cleared by writing it back as a 1.</summary>
    public const byte IrqFlags = NativeMethods.Sx127xRegIrqFlags;

    /// <summary>RegRxNbBytes, the payload length of the last packet received.</summary>
    public const byte RxNbBytes = NativeMethods.Sx127xRegRxNbBytes;

    /// <summary>RegModemStat, the live state of the modem.</summary>
    public const byte ModemStat = NativeMethods.Sx127xRegModemStat;

    /// <summary>RegPktSnrValue, the SNR of the last packet in quarters of a decibel.</summary>
    public const byte PktSnrValue = NativeMethods.Sx127xRegPktSnrValue;

    /// <summary>RegPktRssiValue, the RSSI of the last packet.</summary>
    public const byte PktRssiValue = NativeMethods.Sx127xRegPktRssiValue;

    /// <summary>RegRssiValue, the RSSI the receiver hears right now.</summary>
    public const byte RssiValue = NativeMethods.Sx127xRegRssiValue;

    /// <summary>RegHopChannel, the PLL lock and the CRC the last header announced.</summary>
    public const byte HopChannel = NativeMethods.Sx127xRegHopChannel;

    /// <summary>RegModemConfig1: bandwidth, coding rate, and header mode.</summary>
    public const byte ModemConfig1 = NativeMethods.Sx127xRegModemConfig1;

    /// <summary>RegModemConfig2: spreading factor, CRC, and the top bits of the symbol timeout.</summary>
    public const byte ModemConfig2 = NativeMethods.Sx127xRegModemConfig2;

    /// <summary>RegSymbTimeoutLsb, the low byte of the symbol timeout.</summary>
    public const byte SymbTimeoutLsb = NativeMethods.Sx127xRegSymbTimeoutLsb;

    /// <summary>RegPreambleMsb, the high byte of the preamble length.</summary>
    public const byte PreambleMsb = NativeMethods.Sx127xRegPreambleMsb;

    /// <summary>RegPreambleLsb, the low byte of the preamble length.</summary>
    public const byte PreambleLsb = NativeMethods.Sx127xRegPreambleLsb;

    /// <summary>RegPayloadLength, the payload length to send.</summary>
    public const byte PayloadLength = NativeMethods.Sx127xRegPayloadLength;

    /// <summary>RegMaxPayloadLength, the longest payload a received header may announce.</summary>
    public const byte MaxPayloadLength = NativeMethods.Sx127xRegMaxPayloadLength;

    /// <summary>RegModemConfig3: low data rate optimization and the automatic gain control.</summary>
    public const byte ModemConfig3 = NativeMethods.Sx127xRegModemConfig3;

    /// <summary>RegRssiWideband, a wideband RSSI sample.</summary>
    public const byte RssiWideband = NativeMethods.Sx127xRegRssiWideband;

    /// <summary>RegIfFreq2, which the spurious reception erratum sets.</summary>
    public const byte IfFreq2 = NativeMethods.Sx127xRegIfFreq2;

    /// <summary>RegIfFreq1, which the spurious reception erratum clears.</summary>
    public const byte IfFreq1 = NativeMethods.Sx127xRegIfFreq1;

    /// <summary>RegDetectOptimize: the automatic IF and the detection optimization.</summary>
    public const byte DetectOptimize = NativeMethods.Sx127xRegDetectOptimize;

    /// <summary>RegInvertIQ, the IQ polarity of each path.</summary>
    public const byte InvertIq = NativeMethods.Sx127xRegInvertIq;

    /// <summary>RegHighBwOptimize1, which the 500 kHz erratum sets.</summary>
    public const byte HighBwOptimize1 = NativeMethods.Sx127xRegHighBwOptimize1;

    /// <summary>RegDetectionThreshold, the LoRa detection threshold.</summary>
    public const byte DetectionThreshold = NativeMethods.Sx127xRegDetectionThreshold;

    /// <summary>RegSyncWord, the LoRa sync word.</summary>
    public const byte SyncWord = NativeMethods.Sx127xRegSyncWord;

    /// <summary>RegHighBwOptimize2, which the 500 kHz erratum sets.</summary>
    public const byte HighBwOptimize2 = NativeMethods.Sx127xRegHighBwOptimize2;

    /// <summary>RegInvertIQ2, which completes an IQ inversion.</summary>
    public const byte InvertIq2 = NativeMethods.Sx127xRegInvertIq2;

    /// <summary>RegImageCal, at the address of RegInvertIQ2 on the FSK page.</summary>
    public const byte ImageCal = NativeMethods.Sx127xRegImageCal;

    /// <summary>RegDioMapping1, the events DIO0 to DIO3 signal.</summary>
    public const byte DioMapping1 = NativeMethods.Sx127xRegDioMapping1;

    /// <summary>RegDioMapping2, the events DIO4 and DIO5 signal.</summary>
    public const byte DioMapping2 = NativeMethods.Sx127xRegDioMapping2;

    /// <summary>RegVersion, the silicon revision.</summary>
    public const byte Version = NativeMethods.Sx127xRegVersion;

    /// <summary>RegTcxo, a crystal or a TCXO on XTA.</summary>
    public const byte Tcxo = NativeMethods.Sx127xRegTcxo;

    /// <summary>RegPaDac, the +20 dBm setting of PA_BOOST.</summary>
    public const byte PaDac = NativeMethods.Sx127xRegPaDac;
}

/// <summary>The amplifier settings of an SX127x: RegPaConfig, RegPaDac, and RegOcp.</summary>
/// <param name="PaConfig">RegPaConfig: PaSelect, MaxPower, and OutputPower.</param>
/// <param name="PaDac">RegPaDac: the +20 dBm setting above +17 dBm on PA_BOOST, else its reset value.</param>
/// <param name="Ocp">RegOcp: the current limit.</param>
/// <param name="OutputDbm">The output power the settings produce, in dBm.</param>
public readonly record struct Sx127xTxPower(byte PaConfig, byte PaDac, byte Ocp, sbyte OutputDbm);

/// <summary>The LoRa modem registers of an SX127x for a link.</summary>
/// <param name="ModemConfig1">RegModemConfig1: bandwidth, coding rate, and header mode.</param>
/// <param name="ModemConfig2">RegModemConfig2: spreading factor, CRC, and the top bits of the symbol timeout.</param>
/// <param name="ModemConfig3">RegModemConfig3: low data rate optimization and the AGC.</param>
/// <param name="DetectionOptimize">The DetectionOptimize bits for the low three bits of RegDetectOptimize.</param>
/// <param name="DetectionThreshold">RegDetectionThreshold.</param>
public readonly record struct Sx127xModem(
    byte ModemConfig1,
    byte ModemConfig2,
    byte ModemConfig3,
    byte DetectionOptimize,
    byte DetectionThreshold);

/// <summary>The signal levels a LoRa packet was received with.</summary>
/// <param name="RssiDbm">The RSSI averaged over the packet, in dBm.</param>
/// <param name="SnrDb">The estimated signal-to-noise ratio, in dB.</param>
/// <param name="SignalRssiDbm">The strength of the packet itself, in dBm.</param>
public readonly record struct Sx127xPacketStatus(double RssiDbm, double SnrDb, double SignalRssiDbm);

/// <summary>The live state of an SX127x LoRa modem, from RegModemStat.</summary>
/// <param name="CodingRateDenominator">The coding rate denominator the last header announced, or <c>null</c> for a reserved value.</param>
/// <param name="Clear">Whether the modem is clear.</param>
/// <param name="HeaderValid">Whether the header of the packet under way is valid.</param>
/// <param name="RxOngoing">Whether a reception is under way.</param>
/// <param name="SignalSynchronized">Whether the modem has synchronized on the end of the preamble.</param>
/// <param name="SignalDetected">Whether a LoRa preamble has been detected.</param>
public readonly record struct Sx127xModemStatus(
    byte? CodingRateDenominator,
    bool Clear,
    bool HeaderValid,
    bool RxOngoing,
    bool SignalSynchronized,
    bool SignalDetected);

/// <summary>The writes of the SX127x 500 kHz sensitivity erratum.</summary>
/// <param name="Optimize1">The RegHighBwOptimize1 value.</param>
/// <param name="Optimize2">The RegHighBwOptimize2 value, or <c>null</c> when it is not written.</param>
public readonly record struct Sx127xHighBwOptimize(byte Optimize1, byte? Optimize2);

/// <summary>The receive settings of the SX127x spurious reception erratum.</summary>
/// <param name="AutomaticIf">Whether AutomaticIFOn stays on.</param>
/// <param name="IfFreq2">The RegIfFreq2 value, with RegIfFreq1 cleared, or <c>null</c> when the IF stays automatic.</param>
/// <param name="OffsetHz">How far above the carrier to receive, in hertz.</param>
public readonly record struct Sx127xSpuriousReception(bool AutomaticIf, byte? IfFreq2, uint OffsetHz);

/// <summary>The Semtech SX1276, SX1277, SX1278, and SX1279, and modules such as the RFM95W.</summary>
/// <remarks>
/// These radios are driven through registers. Each SPI transaction starts with an address byte
/// whose top bit is set for a write, followed by the data, and the address advances with each
/// byte except at the FIFO. These methods give the addresses, the values a LoRa link and an
/// output power put in them, and the readings decoded, from the SX1276/77/78/79 datasheet
/// (Rev 7). Nothing here touches a bus.
/// </remarks>
public static class Sx127x
{
    /// <summary>The RegVersion value of an SX1276, SX1277, SX1278, or SX1279.</summary>
    public const byte Version = NativeMethods.Sx127xVersion;

    /// <summary>The bit of an address byte that makes an access a write.</summary>
    public const byte Write = NativeMethods.Sx127xWrite;

    /// <summary>The sync word the datasheet reserves for LoRaWAN networks.</summary>
    public const byte SyncWordPublic = NativeMethods.Sx127xSyncWordPublic;

    /// <summary>The private sync word, and the chip's reset value.</summary>
    public const byte SyncWordPrivate = NativeMethods.Sx127xSyncWordPrivate;

    /// <summary>RegPaDac at its reset value.</summary>
    public const byte PaDacDefault = NativeMethods.Sx127xPaDacDefault;

    /// <summary>RegPaDac with the +20 dBm setting on PA_BOOST.</summary>
    public const byte PaDacHighPower = NativeMethods.Sx127xPaDacHighPower;

    /// <summary>The RegImageCal bit that starts a calibration.</summary>
    public const byte ImageCalStartBit = NativeMethods.Sx127xImageCalStart;

    /// <summary>The RegImageCal bit set while a calibration runs.</summary>
    public const byte ImageCalRunningBit = NativeMethods.Sx127xImageCalRunning;

    /// <summary>RegLna with maximum gain and the high frequency LNA boost.</summary>
    public const byte LnaBoosted = NativeMethods.Sx127xLnaBoosted;

    /// <summary>RegTcxo for a module clocked by a TCXO.</summary>
    public const byte TcxoInputOn = NativeMethods.Sx127xTcxoInputOn;

    /// <summary>RegDioMapping1 with DIO0 signaling RxDone.</summary>
    public const byte Dio0RxDone = NativeMethods.Sx127xDio0RxDone;

    /// <summary>RegDioMapping1 with DIO0 signaling TxDone.</summary>
    public const byte Dio0TxDone = NativeMethods.Sx127xDio0TxDone;

    /// <summary>RegDioMapping1 with DIO0 signaling CadDone.</summary>
    public const byte Dio0CadDone = NativeMethods.Sx127xDio0CadDone;

    /// <summary>Returns the 24-bit RegFrf word for a frequency.</summary>
    /// <param name="frequencyHz">The carrier frequency in hertz.</param>
    /// <returns>The frequency in steps of 32 MHz over 2^19, rounded to the nearest step.</returns>
    public static uint FrequencyWord(uint frequencyHz) =>
        NativeMethods.pamoja_sx127x_frequency_word(frequencyHz);

    /// <summary>Returns the frequency a RegFrf word selects.</summary>
    /// <param name="word">The 24-bit frequency word.</param>
    /// <returns>The carrier frequency in hertz.</returns>
    public static uint FrequencyFromWord(uint word) =>
        NativeMethods.pamoja_sx127x_frequency_from_word(word);

    /// <summary>Returns the address byte that reads a register.</summary>
    /// <param name="address">The register address.</param>
    /// <returns>The address with the write bit clear.</returns>
    public static byte ReadAddress(byte address) => NativeMethods.pamoja_sx127x_read_address(address);

    /// <summary>Returns the address byte that writes a register.</summary>
    /// <param name="address">The register address.</param>
    /// <returns>The address with the write bit set.</returns>
    public static byte WriteAddress(byte address) => NativeMethods.pamoja_sx127x_write_address(address);

    /// <summary>Returns the RegOpMode value for a LoRa operating mode.</summary>
    /// <param name="mode">The operating mode.</param>
    /// <returns>The register value, with the LoRa register page selected.</returns>
    public static byte LoraOpMode(Sx127xMode mode) =>
        NativeMethods.pamoja_sx127x_lora_op_mode((byte)mode);

    /// <summary>Returns the RegOpMode value for an FSK operating mode, which image calibration needs.</summary>
    /// <param name="mode">The operating mode.</param>
    /// <returns>The register value.</returns>
    public static byte FskOpMode(Sx127xMode mode) =>
        NativeMethods.pamoja_sx127x_fsk_op_mode((byte)mode);

    /// <summary>Returns the operating mode a RegOpMode value holds.</summary>
    /// <param name="opMode">The register value.</param>
    /// <returns>The mode.</returns>
    public static Sx127xMode ModeFromOpMode(byte opMode) =>
        (Sx127xMode)NativeMethods.pamoja_sx127x_mode_from_op_mode(opMode);

    /// <summary>Returns the LoRa modem registers for a link at a carrier.</summary>
    /// <param name="link">The link settings.</param>
    /// <param name="frequencyHz">The carrier frequency, which rules out 250 and 500 kHz below 175 MHz.</param>
    /// <param name="symbolTimeout">A single reception timeout in symbols, whose top bits go in RegModemConfig2.</param>
    /// <returns>RegModemConfig1 to 3 and the SF6 detection settings.</returns>
    /// <exception cref="PamojaException">The SX127x cannot use the link at the carrier.</exception>
    public static Sx127xModem Modem(LoraLink link, uint frequencyHz, ushort symbolTimeout = 0)
    {
        ArgumentNullException.ThrowIfNull(link);

        NativeStatus.ThrowIfError(NativeMethods.pamoja_sx127x_modem(
            NativeLora.Link(link), frequencyHz, symbolTimeout, out PamojaSx127xModem modem));
        return new Sx127xModem(
            modem.ModemConfig1,
            modem.ModemConfig2,
            modem.ModemConfig3,
            modem.DetectionOptimize,
            modem.DetectionThreshold);
    }

    /// <summary>Returns a single reception timeout in the link's symbols.</summary>
    /// <param name="link">The link settings, whose symbol time counts the timeout.</param>
    /// <param name="timeoutMicros">How long to listen for a preamble, in microseconds.</param>
    /// <returns>The timeout rounded up to whole symbols, from 4 to 1023.</returns>
    public static ushort SymbolTimeout(LoraLink link, ulong timeoutMicros)
    {
        ArgumentNullException.ThrowIfNull(link);

        return NativeMethods.pamoja_sx127x_symbol_timeout(NativeLora.Link(link), timeoutMicros);
    }

    /// <summary>Chooses the amplifier settings for an output power.</summary>
    /// <param name="output">The amplifier output the module uses.</param>
    /// <param name="outputDbm">The output power wanted, in dBm.</param>
    /// <returns>The settings, clamped to what the output delivers.</returns>
    public static Sx127xTxPower TxPower(Sx127xPaOutput output, sbyte outputDbm) =>
        FromNative(NativeMethods.pamoja_sx127x_tx_power_for_output(
            output == Sx127xPaOutput.PaBoost, outputDbm));

    /// <summary>Chooses the amplifier settings that keep a link's EIRP at or under a ceiling.</summary>
    /// <param name="output">The amplifier output the module uses.</param>
    /// <param name="budget">The link budget, whose transmitting antenna and cable apply.</param>
    /// <param name="eirpCeilingDbm">The EIRP limit in dBm.</param>
    /// <returns>The settings, rounded down to whole decibels.</returns>
    public static Sx127xTxPower TxPowerUnderCeiling(
        Sx127xPaOutput output,
        LoraLinkBudget budget,
        double eirpCeilingDbm)
    {
        ArgumentNullException.ThrowIfNull(budget);

        return FromNative(NativeMethods.pamoja_sx127x_tx_power_under_ceiling(
            output == Sx127xPaOutput.PaBoost,
            NativeLora.Budget(budget),
            NativeLora.Centi(eirpCeilingDbm)));
    }

    /// <summary>Returns RegOcp for a current limit.</summary>
    /// <param name="milliamps">The most current the amplifier may draw.</param>
    /// <returns>The register value with the protection on.</returns>
    public static byte OcpRegister(ushort milliamps) => NativeMethods.pamoja_sx127x_ocp_register(milliamps);

    /// <summary>Returns RegInvertIQ for the IQ polarity of each path.</summary>
    /// <param name="receive">Whether to invert the receive path, as a LoRaWAN device does for downlinks.</param>
    /// <param name="transmit">Whether to invert the transmit path, as a gateway does.</param>
    /// <returns>The register value, with the transmit bit set for normal IQ as the reference drivers have it.</returns>
    public static byte InvertIq(bool receive, bool transmit) =>
        NativeMethods.pamoja_sx127x_invert_iq(receive, transmit);

    /// <summary>Returns RegInvertIQ2 for the path in use.</summary>
    /// <param name="inverted">Whether that path is inverted.</param>
    /// <returns>0x19 when inverted, else 0x1D.</returns>
    public static byte InvertIq2(bool inverted) => NativeMethods.pamoja_sx127x_invert_iq_2(inverted);

    /// <summary>Returns the writes of the 500 kHz sensitivity erratum.</summary>
    /// <param name="link">The link settings, whose bandwidth decides.</param>
    /// <param name="frequencyHz">The carrier frequency in hertz.</param>
    /// <returns>RegHighBwOptimize1 and, where it is written, RegHighBwOptimize2.</returns>
    /// <exception cref="PamojaException">The SX127x has no such bandwidth.</exception>
    public static Sx127xHighBwOptimize HighBwOptimize(LoraLink link, uint frequencyHz)
    {
        ArgumentNullException.ThrowIfNull(link);

        NativeStatus.ThrowIfError(NativeMethods.pamoja_sx127x_high_bw_optimize(
            NativeLora.Link(link), frequencyHz, out PamojaSx127xHighBwOptimize optimize));
        return new Sx127xHighBwOptimize(
            optimize.Optimize1,
            optimize.HasOptimize2 != 0 ? optimize.Optimize2 : null);
    }

    /// <summary>Returns the receive settings of the spurious reception erratum.</summary>
    /// <param name="link">The link settings, whose bandwidth decides.</param>
    /// <returns>The automatic IF, the hand-set IF, and the carrier offset.</returns>
    /// <exception cref="PamojaException">The SX127x has no such bandwidth.</exception>
    public static Sx127xSpuriousReception SpuriousReception(LoraLink link)
    {
        ArgumentNullException.ThrowIfNull(link);

        NativeStatus.ThrowIfError(NativeMethods.pamoja_sx127x_spurious_reception(
            NativeLora.Link(link), out PamojaSx127xSpuriousReception erratum));
        return new Sx127xSpuriousReception(
            erratum.AutomaticIf != 0,
            erratum.HasIfFreq2 != 0 ? erratum.IfFreq2 : null,
            erratum.OffsetHz);
    }

    /// <summary>Returns RegImageCal to start a calibration.</summary>
    /// <param name="current">The register's current value.</param>
    /// <returns>The value with ImageCalStart set and AutoImageCalOn clear.</returns>
    public static byte ImageCalStart(byte current) => NativeMethods.pamoja_sx127x_image_cal_start(current);

    /// <summary>Returns RegDetectOptimize with AutomaticIFOn set or clear.</summary>
    /// <param name="current">The register's current value.</param>
    /// <param name="on">Whether the automatic IF stays on.</param>
    /// <returns>The register value.</returns>
    public static byte AutomaticIf(byte current, bool on) =>
        NativeMethods.pamoja_sx127x_automatic_if(current, on);

    /// <summary>Decodes RegPktSnrValue and RegPktRssiValue, read together.</summary>
    /// <param name="answer">The two register values, SNR first.</param>
    /// <param name="frequencyHz">The carrier the packet was heard at, which picks the RF port's offset.</param>
    /// <returns>The three signal levels.</returns>
    /// <exception cref="ArgumentException">The answer is not two bytes.</exception>
    public static Sx127xPacketStatus PacketStatus(ReadOnlySpan<byte> answer, uint frequencyHz)
    {
        if (answer.Length != 2)
        {
            throw new ArgumentException(
                $"RegPktSnrValue and RegPktRssiValue are 2 bytes, not {answer.Length}",
                nameof(answer));
        }

        PamojaSx127xPacketStatus status = NativeMethods.pamoja_sx127x_packet_status_from_bytes(
            answer[0], answer[1], frequencyHz);
        return new Sx127xPacketStatus(
            NativeLora.Db(status.RssiCentiDbm),
            NativeLora.Db(status.SnrCentiDb),
            NativeLora.Db(status.SignalRssiCentiDbm));
    }

    /// <summary>Decodes RegRssiValue.</summary>
    /// <param name="value">The register value.</param>
    /// <param name="frequencyHz">The carrier the receiver is tuned to.</param>
    /// <returns>The signal power the receiver hears right now, in dBm.</returns>
    public static double RssiDbm(byte value, uint frequencyHz) =>
        NativeLora.Db(NativeMethods.pamoja_sx127x_rssi_centi_dbm(value, frequencyHz));

    /// <summary>Decodes RegModemStat.</summary>
    /// <param name="value">The register value.</param>
    /// <returns>The modem's live state.</returns>
    public static Sx127xModemStatus ModemStatus(byte value)
    {
        PamojaSx127xModemStatus status = NativeMethods.pamoja_sx127x_modem_status_from_byte(value);
        return new Sx127xModemStatus(
            status.CodingRateDenominator == 0 ? null : status.CodingRateDenominator,
            status.Clear != 0,
            status.HeaderValid != 0,
            status.RxOngoing != 0,
            status.SignalSynchronized != 0,
            status.SignalDetected != 0);
    }

    /// <summary>Describes amplifier settings from the C ABI.</summary>
    /// <param name="power">The settings as the C ABI carries them.</param>
    /// <returns>The settings.</returns>
    private static Sx127xTxPower FromNative(PamojaSx127xTxPower power) =>
        new(power.PaConfig, power.PaDac, power.Ocp, power.OutputDbm);
}
