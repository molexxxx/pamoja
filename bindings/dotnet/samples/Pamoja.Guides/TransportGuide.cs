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
    /// <returns>A task that completes once the example has run.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // Whatever a link is underneath, MQTT, CoAP, or the in-process broker below, it
        // reaches the rest of the framework as one Transport. Anything that takes a link
        // takes that, so a node is written once and pointed at whichever link it has.
        using var broker = new LoopbackBroker();
        using var gateway = broker.Link();
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync("sensors/1/temperature");

        // The fault injector is a Transport wrapping a Transport, so it composes anywhere
        // a link does. This one fails its next send and passes everything after through.
        var flaky = Transport.Faulty(broker.Rung(), 1);
        Expect(flaky.IsAvailable, "a transport not yet composed is holdable");

        // Composing consumes the transport, because whatever it was composed into owns it
        // from here. The handle is emptied rather than left aliasing what now belongs to
        // something else, so it cannot be sent on twice.
        using var ladder = new Ladder(Store.Memory());
        ladder.Rung(flaky);
        Expect(!flaky.IsAvailable, "and it is spent once something else owns it");
        await ladder.ConnectAsync();

        // The injected failure lands, so the reading is buffered rather than lost.
        Expect(
            await ladder.SendAsync("sensors/1/temperature", "20.1"u8.ToArray()) == Delivery.Buffered,
            "a refused send is buffered");
        Expect(await ladder.BufferedAsync() == 1, "and the backlog holds it");

        // The next reading joins the back of the queue instead of overtaking it, even
        // though the link would take it now. Order on the wire is the order the readings
        // were taken.
        Expect(
            await ladder.SendAsync("sensors/1/temperature", "20.4"u8.ToArray()) == Delivery.Buffered,
            "the next reading joins the backlog rather than passing it");
        Expect(await ladder.BufferedAsync() == 2, "so both are queued");

        // Flushing forwards the backlog oldest first, and the subscriber sees it in order.
        Expect(await ladder.FlushAsync() == 2, "a flush forwards the whole backlog");
        Expect(await ladder.BufferedAsync() == 0, "leaving nothing queued");
        Expect(
            (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("20.1"u8) == true,
            "the oldest reading arrives first");
        Expect(
            (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("20.4"u8) == true,
            "then the one taken after it");
        // ANCHOR_END: example
    }
}
