using Pamoja;
using Pamoja.Core;
using Pamoja.Loopback;

using static Guides.Guide;

namespace Guides;

/// <summary>The loopback guide example; see docs/guides/loopback.md.</summary>
public static class LoopbackGuide
{
    /// <summary>Runs the example.</summary>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // One broker and two links off it, all in this process. Nothing binds a port
        // and nothing has to be running for the traffic below to flow.
        using var broker = new LoopbackBroker();
        using LoopbackTransport publisher = broker.Link();
        using LoopbackTransport subscriber = broker.Link();
        await publisher.ConnectAsync();
        await subscriber.ConnectAsync();

        // A `+` stands for exactly one level, so the deeper topic is not delivered
        // here and the first message this subscriber sees is the second publish.
        await subscriber.SubscribeAsync("line/+/temp");
        await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());
        await publisher.SendAsync("line/mixer/temp", "21.5"u8.ToArray());

        TransportMessage? message = await subscriber.ReceiveAsync();
        Expect(message?.Topic == "line/mixer/temp", "the topic survives the trip");
        Expect(
            message!.Payload.AsSpan().SequenceEqual("21.5"u8),
            "and so does the reading");

        // A `#` covers the levels that remain, so a second link takes the whole
        // subtree, including the reading the single-level filter passed over.
        using LoopbackTransport watcher = broker.Link();
        await watcher.ConnectAsync();
        await watcher.SubscribeAsync("line/#");
        await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());

        TransportMessage? deep = await watcher.ReceiveAsync();
        Expect(deep?.Topic == "line/mixer/temp/raw", "the deeper topic arrives here");
        Expect(deep!.Payload.AsSpan().SequenceEqual("2150"u8), "with its own payload");

        // A link that has been disconnected reports the failure instead of dropping
        // the reading, which is the case a test wants to reach without unplugging.
        await publisher.DisconnectAsync();
        bool refused = false;
        try
        {
            await publisher.SendAsync("line/mixer/temp", "21.6"u8.ToArray());
        }
        catch (PamojaException)
        {
            refused = true;
        }
        Expect(refused, "a disconnected link refuses to publish");
        // ANCHOR_END: example
    }
}
