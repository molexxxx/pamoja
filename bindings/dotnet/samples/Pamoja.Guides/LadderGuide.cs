using Pamoja.Core;
using Pamoja.Ladder;
using Pamoja.Loopback;
using Pamoja.Sync;

using static Guides.Guide;

namespace Guides;

/// <summary>The transport ladder guide example; see docs/guides/ladder.md.</summary>
public static class LadderGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the example has run.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // Two links off the same node: a near mesh hop and a metered backhaul. Each is a
        // separate broker, so which one carried a reading is visible from its subscriber.
        using var mesh = new LoopbackBroker();
        using var backhaul = new LoopbackBroker();
        using var gateway = backhaul.Link();
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync("sensors/1/temperature");

        // Rungs are tried in the order they are added, cheapest first. The mesh hop loses
        // every packet here; the backhaul carries one send, then drops the next two.
        using var ladder = new Ladder(Store.Memory());
        ladder.Rung(Transport.Degraded(mesh.Rung(), dropEvery: 1));
        ladder.Rung(Transport.Degraded(backhaul.Rung(), up: 1, down: 2));
        await ladder.ConnectAsync();

        // The mesh hop refuses, so the reading goes out over the backhaul and arrives on
        // the broker only that rung publishes to.
        const string topic = "sensors/1/temperature";
        Expect(
            await ladder.SendAsync(topic, "21.5"u8.ToArray()) == Delivery.Sent,
            "a dead rung falls through to the next one");
        Expect(
            (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("21.5"u8) == true,
            "and the reading arrives over the rung that took it");

        // Now nothing will take a send, so the next reading is buffered rather than lost.
        Expect(
            await ladder.SendAsync(topic, "21.6"u8.ToArray()) == Delivery.Buffered,
            "with every rung down the reading is buffered");
        Expect(await ladder.BufferedAsync() == 1, "and the backlog holds it");

        // A flush while the links are still down forwards nothing and leaves the backlog
        // intact, because a record is removed only once a rung has accepted it.
        Expect(await ladder.FlushAsync() == 0, "a flush with no link forwards nothing");
        Expect(await ladder.BufferedAsync() == 1, "and loses nothing");

        // The backhaul is reachable again, so the buffered reading goes out exactly once.
        Expect(await ladder.FlushAsync() == 1, "the reading goes out once a link returns");
        Expect(await ladder.BufferedAsync() == 0, "leaving nothing queued");
        Expect(
            (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("21.6"u8) == true,
            "and it arrives exactly once");
        // ANCHOR_END: example
    }
}
