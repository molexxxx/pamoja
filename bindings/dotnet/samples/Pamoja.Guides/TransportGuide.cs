using Pamoja.Core;
using Pamoja.Ladder;
using Pamoja.Loopback;
using Pamoja.Sync;

using static Guides.Guide;

namespace Guides;

/// <summary>The engine surface guide example; see docs/guides/transport.md.</summary>
public static class TransportGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the backlog has been forwarded.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        const string Topic = "sensors/1/temperature";

        // Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it
        // reaches the rest of the framework through one contract. Anything that takes a
        // link works with any of them, so a node is written once and pointed at whichever
        // link it has.
        using var broker = new LoopbackBroker();
        using var gateway = broker.Link();
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync(Topic);

        // The fault injector is itself a link wrapping a link, so it composes anywhere one
        // does. This one fails its next send and passes the rest through.
        using var ladder = new Ladder(Store.Memory());
        ladder.Rung(Transport.Faulty(broker.Rung(), 1));
        await ladder.ConnectAsync();

        // The injected failure lands, so the reading is buffered rather than lost.
        Delivery first = await ladder.SendAsync(Topic, "20.1"u8.ToArray());
        Console.WriteLine($"first reading: {first}, {await ladder.BufferedAsync()} queued");

        // The next reading joins the back of the queue instead of overtaking it, even
        // though the link would take it now. Order on the wire is the order they were
        // taken.
        Delivery second = await ladder.SendAsync(Topic, "20.4"u8.ToArray());
        int queued = await ladder.BufferedAsync();
        Console.WriteLine($"second reading: {second}, {queued} queued");

        // Flushing forwards the backlog oldest first, and the subscriber sees it in order.
        int forwarded = await ladder.FlushAsync();
        TransportMessage earlier = (await gateway.ReceiveAsync())!;
        TransportMessage later = (await gateway.ReceiveAsync())!;
        Console.WriteLine(
            $"flush forwarded {forwarded}, gateway saw"
            + $" {System.Text.Encoding.UTF8.GetString(earlier.Payload)} then"
            + $" {System.Text.Encoding.UTF8.GetString(later.Payload)}");
        // ANCHOR_END: example

        Expect(first == Delivery.Buffered, "a refused send is buffered, not lost");
        Expect(second == Delivery.Buffered, "and the next one queues behind it");
        Expect(queued == 2, "so both are held");
        Expect(forwarded == 2, "a flush forwards the whole backlog");
        Expect(await ladder.BufferedAsync() == 0, "leaving nothing queued");
        Expect(earlier.Payload.AsSpan().SequenceEqual("20.1"u8), "oldest first");
        Expect(later.Payload.AsSpan().SequenceEqual("20.4"u8), "then the one after it");
    }
}
