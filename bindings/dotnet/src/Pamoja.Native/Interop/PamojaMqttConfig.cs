using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// Connection settings passed to <see cref="NativeMethods.pamoja_mqtt_client_new"/>,
/// mirroring <c>PamojaMqttConfig</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// <see cref="ClientId"/> and <see cref="Host"/> are pointers to null-terminated
/// UTF-8 strings borrowed for the duration of the call. A <see cref="KeepAliveSecs"/>,
/// <see cref="Capacity"/>, or <see cref="MaxPacketSize"/> of <c>0</c> selects the
/// core default. <see cref="Username"/>, <see cref="Password"/>, <see cref="Will"/>, and
/// <see cref="Tls"/> are each <see cref="IntPtr.Zero"/> when unused.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaMqttConfig
{
    /// <summary>Pointer to the UTF-8 MQTT client identifier presented to the broker.</summary>
    public IntPtr ClientId;

    /// <summary>Pointer to the UTF-8 broker hostname or IP address.</summary>
    public IntPtr Host;

    /// <summary>The broker TCP port, conventionally 1883 for plaintext MQTT.</summary>
    public ushort Port;

    /// <summary>Keep-alive interval in seconds, or 0 for the default of 30.</summary>
    public uint KeepAliveSecs;

    /// <summary>Bound on outstanding client requests, or 0 for the default of 64.</summary>
    public uint Capacity;

    /// <summary>Default quality of service for publishes and subscriptions.</summary>
    public PamojaQos Qos;

    /// <summary>
    /// The largest packet the connection sends or accepts, in bytes, or 0 for the
    /// default of 10,240.
    /// </summary>
    public uint MaxPacketSize;

    /// <summary>Pointer to the UTF-8 name to sign in with, or zero.</summary>
    public IntPtr Username;

    /// <summary>Pointer to the UTF-8 password to sign in with, or zero.</summary>
    public IntPtr Password;

    /// <summary>Pointer to a <see cref="PamojaMqttWill"/>, or zero.</summary>
    public IntPtr Will;

    /// <summary>Pointer to a <see cref="PamojaMqttTls"/>, or zero for plain TCP.</summary>
    public IntPtr Tls;
}

/// <summary>A last-will message, mirroring <c>PamojaMqttWill</c> in <c>pamoja.h</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaMqttWill
{
    /// <summary>Pointer to the UTF-8 topic the broker publishes it to.</summary>
    public IntPtr Topic;

    /// <summary>Pointer to the payload bytes.</summary>
    public IntPtr Payload;

    /// <summary>How many bytes <see cref="Payload"/> holds.</summary>
    public nuint PayloadLen;

    /// <summary>The quality of service it is published at.</summary>
    public PamojaQos Qos;

    /// <summary>1 to have the broker retain it, 0 not to.</summary>
    public byte Retain;
}

/// <summary>TLS settings, mirroring <c>PamojaMqttTls</c> in <c>pamoja.h</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaMqttTls
{
    /// <summary>Pointer to the PEM of the authorities to trust.</summary>
    public IntPtr CaPem;

    /// <summary>How many bytes <see cref="CaPem"/> holds, or 0 to trust the system's authorities.</summary>
    public nuint CaPemLen;

    /// <summary>Pointer to the PEM of a client certificate.</summary>
    public IntPtr CertificatePem;

    /// <summary>How many bytes <see cref="CertificatePem"/> holds, or 0 for none.</summary>
    public nuint CertificatePemLen;

    /// <summary>Pointer to the PEM of the client certificate's key.</summary>
    public IntPtr KeyPem;

    /// <summary>How many bytes <see cref="KeyPem"/> holds, or 0 for none.</summary>
    public nuint KeyPemLen;
}
