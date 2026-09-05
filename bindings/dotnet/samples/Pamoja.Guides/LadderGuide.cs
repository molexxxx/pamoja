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
    /// <returns>A task that completes once the backlog has gone out.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        const string Topic = "sensors/1/temperature";

        // Two links off the same node: a near mesh hop and a metered backhaul. Each is a
        // separate broker, so which one carried a reading is visible from its subscriber.
        using var mesh = new LoopbackBroker();
        using var backhaul = new LoopbackBroker();
        using var gateway = backhaul.Link();
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync(Topic);

        // Rungs are tried in the order they are added, cheapest first. The mesh hop loses
        // every packet here; the backhaul carries one send, then drops the next two.
        using var ladder = new Ladder(Store.Memory());
        ladder.Rung(Transport.Degraded(mesh.Rung(), dropEvery: 1));
        ladder.Rung(Transport.Degraded(backhaul.Rung(), up: 1, down: 2));
        await ladder.ConnectAsync();

        // The mesh hop refuses, so the reading goes out over the backhaul and arrives on
        // the broker only that rung publishes to.
        Delivery first = await ladder.SendAsync(Topic, "21.5"u8.ToArray());
        TransportMessage arrived = (await gateway.ReceiveAsync())!;
        Console.WriteLine(
            $"first reading: {first}, gateway got"
            + $" {System.Text.Encoding.UTF8.GetString(arrived.Payload)}");

        // Now nothing will take a send, so the next reading is buffered rather than lost.
        Delivery second = await ladder.SendAsync(Topic, "21.6"u8.ToArray());
        int waiting = await ladder.BufferedAsync();
        Console.WriteLine($"second reading: {second}, {waiting} waiting in the queue");

        // A flush while the links are still down forwards nothing and leaves the backlog
        // intact, because a record is removed only once a rung has accepted it.
        int whileDown = await ladder.FlushAsync();
        Console.WriteLine(
            $"flush while down forwarded {whileDown}, queue still {await ladder.BufferedAsync()}");

        // The backhaul is reachable again, so the buffered reading goes out exactly once.
        int whenUp = await ladder.FlushAsync();
        TransportMessage late = (await gateway.ReceiveAsync())!;
        Console.WriteLine(
            $"flush when up forwarded {whenUp}, gateway got"
            + $" {System.Text.Encoding.UTF8.GetString(late.Payload)}");
        // ANCHOR_END: example

        Expect(first == Delivery.Sent, "a dead rung falls through to the next one");
        Expect(arrived.Payload.AsSpan().SequenceEqual("21.5"u8), "over the rung that took it");
        Expect(second == Delivery.Buffered, "with every rung down the reading is buffered");
        Expect(waiting == 1, "and the backlog holds it");
        Expect(whileDown == 0, "a flush with no link forwards nothing");
        Expect(whenUp == 1, "the reading goes out once a link returns");
        Expect(await ladder.BufferedAsync() == 0, "leaving nothing queued");
        Expect(late.Payload.AsSpan().SequenceEqual("21.6"u8), "and it arrives exactly once");
    }
}
