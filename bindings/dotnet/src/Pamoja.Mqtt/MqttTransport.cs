using Pamoja.Native.Interop;

using Pamoja.Core;

namespace Pamoja.Mqtt;

/// <summary>Builds the <see cref="Transport"/> a ladder rung uses to reach a broker over MQTT.</summary>
public static class MqttTransport
{
    /// <summary>Creates a transport that reaches a broker over MQTT.</summary>
    /// <param name="options">The broker settings.</param>
    /// <returns>The transport, ready to add as a rung.</returns>
    /// <exception cref="ArgumentOutOfRangeException">A QoS is not one of the <see cref="Qos"/> values.</exception>
    /// <exception cref="ArgumentException">A password comes without a username, or a client certificate without its key.</exception>
    public static Transport Open(MqttClientOptions options)
    {
        using var native = new NativeMqttOptions(options);
        return new Transport(NativeMethods.pamoja_transport_mqtt(ref native.Config), "MQTT transport");
    }

}
