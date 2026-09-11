using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// How a module wires its SX126x, mirroring <c>PamojaSx126xBoard</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// The SPI interface cannot see the parts around the chip, so these come from the module's
/// schematic or its maker's example code.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx126xBoard
{
    /// <summary><c>1</c> for the high power amplifier, <c>0</c> for the SX1261's.</summary>
    public byte HighPower;

    /// <summary><c>1</c> when a TCXO powered from DIO3 clocks the chip.</summary>
    public byte Tcxo;

    /// <summary>The TCXO supply voltage as SetDIO3AsTCXOCtrl takes it, 0 for 1.6 V to 7 for 3.3 V.</summary>
    public byte TcxoVoltage;

    /// <summary><c>1</c> when DIO2 drives the antenna switch.</summary>
    public byte Dio2RfSwitch;

    /// <summary><c>1</c> when the module fits the inductor the DC-DC regulator needs.</summary>
    public byte DcDc;

    /// <summary><c>1</c> for an LLCC68, which is held to the rates it supports.</summary>
    public byte Llcc68;

    /// <summary>How long the TCXO takes to settle, in microseconds.</summary>
    public uint TcxoSettleUs;
}

/// <summary>
/// What a radio of either family sends and listens with, mirroring
/// <c>PamojaLoraRadioConfig</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraRadioConfig
{
    /// <summary>The carrier frequency in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The lower edge of the band an SX126x calibrates for, or 0 for the carrier.</summary>
    public uint BandLowHz;

    /// <summary>The upper edge of that band in hertz.</summary>
    public uint BandHighHz;

    /// <summary>The spreading factor, bandwidth, coding rate, preamble, header, and CRC.</summary>
    public PamojaLoraLink Link;

    /// <summary>The output power asked of the amplifier, in dBm.</summary>
    public sbyte OutputDbm;

    /// <summary>The sync word byte: 0x34 for a public network, 0x12 for a private one.</summary>
    public byte SyncWord;

    /// <summary><c>1</c> to send frames with inverted IQ.</summary>
    public byte InvertIqTransmit;

    /// <summary><c>1</c> to expect frames with inverted IQ.</summary>
    public byte InvertIqReceive;
}

/// <summary>
/// How a reception ended, mirroring <c>PamojaLoraRadioReception</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraRadioReception
{
    /// <summary>The outcome: a frame, a timeout, a corrupt frame, or nothing.</summary>
    public byte Outcome;

    /// <summary>The payload length at the start of the buffer, for a frame.</summary>
    public nuint Len;

    /// <summary>The average RSSI over the frame, in hundredths of a dBm.</summary>
    public int RssiCentiDbm;

    /// <summary>The signal-to-noise ratio, in hundredths of a dB.</summary>
    public int SnrCentiDb;

    /// <summary>The strength of the LoRa signal itself, in hundredths of a dBm.</summary>
    public int SignalRssiCentiDbm;
}
