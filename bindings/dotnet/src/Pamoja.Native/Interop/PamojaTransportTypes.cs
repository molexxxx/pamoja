using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>Whether a CoAP request is acknowledged and retried.</summary>
public enum PamojaCoapReliability
{
    /// <summary>Fire and forget: the request is sent once and not acknowledged.</summary>
    NonConfirmable = 0,

    /// <summary>The request is acknowledged, and retransmitted until an ACK arrives.</summary>
    Confirmable = 1,
}

/// <summary>The settings a CoAP endpoint is built from.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaCoapConfig
{
    /// <summary>The peer hostname or IP address, as null-terminated UTF-8.</summary>
    public IntPtr Host;

    /// <summary>The peer UDP port, conventionally 5683 for plaintext CoAP.</summary>
    public ushort Port;

    /// <summary>The local address to bind, or null for the default.</summary>
    public IntPtr Bind;

    /// <summary>Whether requests are acknowledged and retried.</summary>
    public PamojaCoapReliability Reliability;

    /// <summary>How long to wait for an acknowledgement, in milliseconds.</summary>
    public uint AckTimeoutMs;

    /// <summary>How many times to retransmit an unacknowledged request.</summary>
    public uint MaxRetransmits;
}

/// <summary>What became of a message handed to a ladder.</summary>
public enum PamojaDelivery
{
    /// <summary>A rung took the message and it is on its way.</summary>
    Sent = 0,

    /// <summary>No rung would take it, so it is in the buffer awaiting a flush.</summary>
    Buffered = 1,
}

/// <summary>Where a robot is and which way it faces.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaPose
{
    /// <summary>Position along the world x axis, in metres.</summary>
    public float X;

    /// <summary>Position along the world y axis, in metres.</summary>
    public float Y;

    /// <summary>Heading from the world x axis, in radians.</summary>
    public float Theta;
}

/// <summary>How fast a robot is asked to move.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaTwist
{
    /// <summary>Forward speed along the x axis.</summary>
    public float Vx;

    /// <summary>Leftward speed along the y axis.</summary>
    public float Vy;

    /// <summary>Yaw rate about the z axis, positive counter-clockwise.</summary>
    public float Omega;
}
