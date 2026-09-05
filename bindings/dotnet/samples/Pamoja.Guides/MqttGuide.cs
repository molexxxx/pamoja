using Pamoja;
using Pamoja.Mqtt;

using static Guides.Guide;

namespace Guides;

/// <summary>The MQTT guide example; see docs/guides/mqtt.md.</summary>
public static class MqttGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the gateway has the reading.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // The broker on the site. The guide's CI runs one on localhost; point these at
        // yours and nothing else changes.
        const string Broker = "127.0.0.1";
        const ushort Port = 1883;

        // The gateway takes every temperature on the site. A `+` stands for exactly one
        // level, so this matches every node's temperature and nothing deeper.
        await using var gateway = new MqttClient(new MqttClientOptions
        {
            ClientId = "site-gateway",
            Host = Broker,
            Port = Port,
            Qos = Qos.AtLeastOnce,
        });
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync("sensors/+/temperature");
        Console.WriteLine("gateway   subscribed to sensors/+/temperature");

        // A node publishes under that pattern. At-least-once means the broker
        // acknowledges the message, so a node knows its reading was taken.
        await using var node = new MqttClient(new MqttClientOptions
        {
            ClientId = "node-1",
            Host = Broker,
            Port = Port,
            Qos = Qos.AtLeastOnce,
        });
        await node.ConnectAsync();
        await node.PublishAsync("sensors/1/temperature", "21.5");
        Console.WriteLine("node      published 21.5 to sensors/1/temperature");

        // The gateway receives it with the topic attached, which is how it knows which
        // node sent the reading without the payload having to repeat it.
        MqttMessage received = (await gateway.RecvAsync())!;
        Console.WriteLine(
            $"gateway   got {System.Text.Encoding.UTF8.GetString(received.Payload.Span)}"
            + $" on {received.Topic}");

        // Disconnecting leaves the client reusable, so a node that loses its link can
        // reconnect the same object when the broker comes back.
        await node.DisconnectAsync();
        Console.WriteLine($"node      disconnected, still connected: {await node.IsConnectedAsync()}");

        // A broker that is not there is reported rather than leaving a client that looks
        // connected, so a retry loop has something to test.
        await using var nowhere = new MqttClient(new MqttClientOptions
        {
            ClientId = "node-2",
            Host = Broker,
            Port = 1,
            KeepAliveSecs = 1,
        });
        try
        {
            await nowhere.ConnectAsync();
            Console.WriteLine("an unreachable broker accepted a connection, which cannot be");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"unreachable broker refused: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(received.Topic == "sensors/1/temperature", "the topic travels with the reading");
        Expect(
            received.Payload.Span.SequenceEqual("21.5"u8),
            "and so does what the node measured");
        Expect(!await node.IsConnectedAsync(), "a disconnected client says so");
    }
}
