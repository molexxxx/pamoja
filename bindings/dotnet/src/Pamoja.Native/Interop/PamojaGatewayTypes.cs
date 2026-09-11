using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A packet a gateway heard, without its payload, mirroring <c>PamojaGatewayRxpk</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayRxpk
{
    /// <summary>When it arrived, in microseconds since 1970-01-01 UTC, when <c>HasReceivedAt</c>.</summary>
    public ulong ReceivedAtUs;

    /// <summary>When it arrived on the GPS clock, in milliseconds, when <c>HasGpsMillis</c>.</summary>
    public ulong GpsMillis;

    /// <summary>The carrier it arrived on, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The concentrator's own timestamp of the reception, when <c>HasTimestamp</c>.</summary>
    public uint TimestampUs;

    /// <summary>The spreading factor, bandwidth, and coding rate, for a LoRa packet.</summary>
    public PamojaLoraLink Link;

    /// <summary>The bitrate in bits per second, for an FSK packet.</summary>
    public uint BitrateBps;

    /// <summary>The received signal strength, in hundredths of a dBm.</summary>
    public int RssiCentiDbm;

    /// <summary>The signal-to-noise ratio, in hundredths of a dB, when <c>HasSnr</c>.</summary>
    public int SnrCentiDb;

    /// <summary>The concentrator channel it arrived on.</summary>
    public byte Channel;

    /// <summary>The radio chain it arrived on.</summary>
    public byte RfChain;

    /// <summary>What the CRC said: <c>1</c>, <c>-1</c>, or <c>0</c>.</summary>
    public sbyte Crc;

    /// <summary>The modulation: <c>0</c> for LoRa, <c>1</c> for FSK.</summary>
    public byte Modulation;

    /// <summary><c>1</c> when the datagram carried a reception time.</summary>
    public byte HasReceivedAt;

    /// <summary><c>1</c> when it carried a GPS time.</summary>
    public byte HasGpsMillis;

    /// <summary><c>1</c> when it carried the concentrator's timestamp.</summary>
    public byte HasTimestamp;

    /// <summary><c>1</c> when it carried a signal-to-noise ratio.</summary>
    public byte HasSnr;
}

/// <summary>
/// A gateway's own status report, mirroring <c>PamojaGatewayStat</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStat
{
    /// <summary>The gateway's clock, in seconds since 1970-01-01 UTC, when <c>HasTime</c>.</summary>
    public ulong TimeS;

    /// <summary>Its latitude in degrees, north positive, when <c>HasPosition</c>.</summary>
    public double LatitudeDeg;

    /// <summary>Its longitude in degrees, east positive, when <c>HasPosition</c>.</summary>
    public double LongitudeDeg;

    /// <summary>What share of its datagrams were acknowledged, as a percentage.</summary>
    public double AcknowledgedPercent;

    /// <summary>Its altitude in meters, when <c>HasAltitude</c>.</summary>
    public int AltitudeM;

    /// <summary>How many packets its radio received.</summary>
    public uint Received;

    /// <summary>How many of those had a good CRC.</summary>
    public uint ReceivedOk;

    /// <summary>How many it forwarded.</summary>
    public uint Forwarded;

    /// <summary>How many downlink datagrams it received.</summary>
    public uint Downlinks;

    /// <summary>How many packets it transmitted.</summary>
    public uint Transmitted;

    /// <summary><c>1</c> when the report carried a clock reading.</summary>
    public byte HasTime;

    /// <summary><c>1</c> when it carried a position.</summary>
    public byte HasPosition;

    /// <summary><c>1</c> when it carried an altitude.</summary>
    public byte HasAltitude;
}

/// <summary>
/// A packet a server asks a gateway to transmit, without its payload, mirroring
/// <c>PamojaGatewayTxpk</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayTxpk
{
    /// <summary>The GPS time to transmit at, in milliseconds, when <c>HasGpsMillis</c>.</summary>
    public ulong GpsMillis;

    /// <summary>The carrier to transmit on, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The concentrator timestamp to transmit at, when <c>HasTimestamp</c>.</summary>
    public uint TimestampUs;

    /// <summary>The spreading factor, bandwidth, and coding rate, for a LoRa packet.</summary>
    public PamojaLoraLink Link;

    /// <summary>The bitrate in bits per second, for an FSK packet.</summary>
    public uint BitrateBps;

    /// <summary>The FSK frequency deviation in hertz, when <c>HasDeviation</c>.</summary>
    public uint FrequencyDeviationHz;

    /// <summary>How long a preamble to send, in symbols, when <c>HasPreamble</c>.</summary>
    public ushort PreambleSymbols;

    /// <summary>The radio chain to transmit from.</summary>
    public byte RfChain;

    /// <summary>The power to transmit at, in dBm.</summary>
    public sbyte PowerDbm;

    /// <summary>The modulation: <c>0</c> for LoRa, <c>1</c> for FSK.</summary>
    public byte Modulation;

    /// <summary><c>1</c> to transmit at once, which ignores the timestamps.</summary>
    public byte Immediate;

    /// <summary><c>1</c> to invert the LoRa polarity, as a LoRaWAN downlink is sent.</summary>
    public byte InvertPolarity;

    /// <summary><c>1</c> to leave the physical CRC off, as LoRaWAN downlinks are.</summary>
    public byte WithoutCrc;

    /// <summary><c>1</c> when a concentrator timestamp was given.</summary>
    public byte HasTimestamp;

    /// <summary><c>1</c> when a GPS time was given.</summary>
    public byte HasGpsMillis;

    /// <summary><c>1</c> when an FSK deviation was given.</summary>
    public byte HasDeviation;

    /// <summary><c>1</c> when a preamble length was given.</summary>
    public byte HasPreamble;
}
