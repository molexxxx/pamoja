using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

using Pamoja.Core;

namespace Pamoja.Mqtt;

/// <summary>Builds the <see cref="Transport"/> a ladder rung uses to reach a broker over MQTT.</summary>
public static class MqttTransport
{
    /// <summary>Creates a transport that reaches a broker over MQTT.</summary>
    /// <param name="options">The broker settings.</param>
    /// <returns>The transport, ready to add as a rung.</returns>
    public static Transport Open(MqttClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        IntPtr clientId = Marshal.StringToCoTaskMemUTF8(options.ClientId);
        IntPtr host = Marshal.StringToCoTaskMemUTF8(options.Host);
        try
        {
            PamojaMqttConfig config = new()
            {
                ClientId = clientId,
                Host = host,
                Port = options.Port,
                KeepAliveSecs = options.KeepAliveSecs ?? 0,
                Capacity = options.Capacity ?? 0,
                Qos = (PamojaQos)(options.Qos ?? Qos.AtLeastOnce),
            };
            return new Transport(NativeMethods.pamoja_transport_mqtt(ref config), "MQTT transport");
        }
        finally
        {
            Marshal.FreeCoTaskMem(clientId);
            Marshal.FreeCoTaskMem(host);
        }
    }

}
