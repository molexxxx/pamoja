using Pamoja;
using Pamoja.Mqtt;

using static Guides.Guide;

namespace Guides;

/// <summary>The MQTT guide example; see docs/guides/mqtt.md.</summary>
public static class MqttGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the example has run.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire.
        Expect((int)Qos.AtMostOnce == 0, "at most once is level 0");
        Expect((int)Qos.AtLeastOnce == 1, "at least once is level 1");
        Expect((int)Qos.ExactlyOnce == 2, "exactly once is level 2");

        // Nothing listens on this port, so the broker is unreachable. Constructing the
        // client touches nothing; only connecting does.
        await using var client = new MqttClient(new MqttClientOptions
        {
            ClientId = "guide-node",
            Host = "127.0.0.1",
            Port = 47811,
            KeepAliveSecs = 1,
            Qos = Qos.ExactlyOnce,
        });
        Expect(!await client.IsConnectedAsync(), "a fresh client holds no connection");

        // A refused connection surfaces as a transport error and leaves the client as it
        // was, so the same object can be retried once the broker is back.
        bool refused = false;
        try
        {
            await client.ConnectAsync();
        }
        catch (PamojaException error)
        {
            refused = error.Message.StartsWith("transport error", StringComparison.Ordinal);
        }

        Expect(refused, "an unreachable broker is reported, not swallowed");
        Expect(
            !await client.IsConnectedAsync(),
            "a failed connect leaves the client disconnected");
        // ANCHOR_END: example
    }
}
