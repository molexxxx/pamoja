using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// Connection settings passed to <see cref="NativeMethods.pamoja_mqtt_client_new"/>,
/// mirroring <c>PamojaMqttConfig</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// <see cref="ClientId"/> and <see cref="Host"/> are pointers to null-terminated
/// UTF-8 strings borrowed for the duration of the call. A <see cref="KeepAliveSecs"/>
/// or <see cref="Capacity"/> of <c>0</c> selects the core default.
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
}
