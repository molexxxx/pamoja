using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The amplifier settings of an SX127x, mirroring <c>PamojaSx127xTxPower</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx127xTxPower
{
    /// <summary>RegPaConfig: PaSelect, MaxPower, and OutputPower.</summary>
    public byte PaConfig;

    /// <summary>RegPaDac: the +20 dBm setting above +17 dBm on PA_BOOST, else its reset value.</summary>
    public byte PaDac;

    /// <summary>RegOcp: the current limit.</summary>
    public byte Ocp;

    /// <summary>The output power the settings produce, in dBm.</summary>
    public sbyte OutputDbm;
}

/// <summary>
/// The LoRa modem registers of an SX127x for a link, mirroring <c>PamojaSx127xModem</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx127xModem
{
    /// <summary>RegModemConfig1: bandwidth, coding rate, and header mode.</summary>
    public byte ModemConfig1;

    /// <summary>RegModemConfig2: spreading factor, CRC, and the top bits of the symbol timeout.</summary>
    public byte ModemConfig2;

    /// <summary>RegModemConfig3: low data rate optimization and the AGC.</summary>
    public byte ModemConfig3;

    /// <summary>The DetectionOptimize bits for the low three bits of RegDetectOptimize.</summary>
    public byte DetectionOptimize;

    /// <summary>RegDetectionThreshold.</summary>
    public byte DetectionThreshold;
}

/// <summary>
/// The signal levels of a packet an SX127x received, mirroring
/// <c>PamojaSx127xPacketStatus</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx127xPacketStatus
{
    /// <summary>The RSSI averaged over the packet, in hundredths of a dBm.</summary>
    public int RssiCentiDbm;

    /// <summary>The estimated signal-to-noise ratio, in hundredths of a dB.</summary>
    public int SnrCentiDb;

    /// <summary>The strength of the packet itself, in hundredths of a dBm.</summary>
    public int SignalRssiCentiDbm;
}

/// <summary>
/// The live state of an SX127x LoRa modem, mirroring <c>PamojaSx127xModemStatus</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx127xModemStatus
{
    /// <summary>The coding rate denominator the last header announced, or 0 for a reserved value.</summary>
    public byte CodingRateDenominator;

    /// <summary><c>1</c> when the modem is clear.</summary>
    public byte Clear;

    /// <summary><c>1</c> when the header of the packet under way is valid.</summary>
    public byte HeaderValid;

    /// <summary><c>1</c> while a reception is under way.</summary>
    public byte RxOngoing;

    /// <summary><c>1</c> once the modem has synchronized on the end of the preamble.</summary>
    public byte SignalSynchronized;

    /// <summary><c>1</c> once a LoRa preamble has been detected.</summary>
    public byte SignalDetected;
}

/// <summary>
/// The writes of the SX127x 500 kHz sensitivity erratum, mirroring
/// <c>PamojaSx127xHighBwOptimize</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx127xHighBwOptimize
{
    /// <summary>The RegHighBwOptimize1 value.</summary>
    public byte Optimize1;

    /// <summary>The RegHighBwOptimize2 value, when <see cref="HasOptimize2"/> is set.</summary>
    public byte Optimize2;

    /// <summary><c>1</c> when RegHighBwOptimize2 is written.</summary>
    public byte HasOptimize2;
}

/// <summary>
/// The receive settings of the SX127x spurious reception erratum, mirroring
/// <c>PamojaSx127xSpuriousReception</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx127xSpuriousReception
{
    /// <summary><c>1</c> when AutomaticIFOn stays on.</summary>
    public byte AutomaticIf;

    /// <summary><c>1</c> when RegIfFreq2 is set by hand, with RegIfFreq1 cleared.</summary>
    public byte HasIfFreq2;

    /// <summary>The RegIfFreq2 value, when <see cref="HasIfFreq2"/> is set.</summary>
    public byte IfFreq2;

    /// <summary>How far above the carrier to receive, in hertz.</summary>
    public uint OffsetHz;
}
