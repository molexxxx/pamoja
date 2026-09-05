using Pamoja;
using Pamoja.Core;
using Pamoja.Loopback;

using static Guides.Guide;

namespace Guides;

/// <summary>The in-process broker guide example; see docs/guides/loopback.md.</summary>
public static class LoopbackGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes when both filters have been shown.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // One broker and two links off it, all in this process. Nothing binds a port and
        // nothing has to be running for the traffic below to flow, which is what makes
        // this the link to develop a node against before it has a real one.
        using var broker = new LoopbackBroker();
        using LoopbackTransport publisher = broker.Link();
        using LoopbackTransport subscriber = broker.Link();
        await publisher.ConnectAsync();
        await subscriber.ConnectAsync();

        // A `+` stands for exactly one level, so this takes the mixer's temperature but
        // not the raw reading a level below it.
        await subscriber.SubscribeAsync("line/+/temp");
        await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());
        await publisher.SendAsync("line/mixer/temp", "21.5"u8.ToArray());

        TransportMessage message = (await subscriber.ReceiveAsync())!;
        Console.WriteLine(
            $"line/+/temp took {System.Text.Encoding.UTF8.GetString(message.Payload)}"
            + $" from {message.Topic}");

        // A `#` covers every level that remains, so a second link takes the whole subtree,
        // including the reading the single-level filter passed over.
        using LoopbackTransport watcher = broker.Link();
        await watcher.ConnectAsync();
        await watcher.SubscribeAsync("line/#");
        await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());

        TransportMessage deep = (await watcher.ReceiveAsync())!;
        Console.WriteLine(
            $"line/#     took {System.Text.Encoding.UTF8.GetString(deep.Payload)}"
            + $" from {deep.Topic}");

        // A link that has been disconnected reports the failure instead of dropping the
        // reading, which is the case a test wants to reach without unplugging anything.
        await publisher.DisconnectAsync();
        try
        {
            await publisher.SendAsync("line/mixer/temp", "21.6"u8.ToArray());
            Console.WriteLine("a disconnected link took a reading, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"disconnected refused the reading: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(message.Topic == "line/mixer/temp", "the single-level filter matched");
        Expect(message.Payload.AsSpan().SequenceEqual("21.5"u8), "and carried the reading");
        Expect(deep.Topic == "line/mixer/temp/raw", "the multi-level filter went deeper");
        Expect(deep.Payload.AsSpan().SequenceEqual("2150"u8), "and carried the raw value");
    }
}
