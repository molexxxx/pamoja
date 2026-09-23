using Pamoja;
using Pamoja.Core;
using Pamoja.Loopback;
using Pamoja.Sync;

using static Guides.Guide;

namespace Guides;

/// <summary>The store-and-forward guide example; see docs/guides/sync.md.</summary>
public static class SyncGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the backlog has reached the gateway.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        const string Topic = "apiary/hive-3/weight";

        // The scale logs its weight to a queue on its SD card, bounded so a long
        // outage cannot fill the card. The directory is the queue, so the scale can
        // lose power at any moment and lose nothing it logged.
        DirectoryInfo dir = Directory.CreateTempSubdirectory("pamoja-hive-");
        Store outbox = Store.File(dir.FullName, 3);
        foreach (string weight in new[] { "41.2", "41.5", "40.9" })
        {
            await outbox.AppendAsync(weight);
        }

        int logged = await outbox.CountAsync();
        Console.WriteLine($"hive      logged {logged} weights with no link, the most its store holds");

        // A full store refuses the next weight rather than dropping one it already
        // holds.
        try
        {
            await outbox.AppendAsync("41.1");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"hive      was refused a 4th: {error.Message}");
        }

        // The scale reboots. Its queue is the directory, so it comes back whole and
        // in order.
        outbox.Dispose();
        outbox = Store.File(dir.FullName, 3);
        int held = await outbox.CountAsync();
        string oldest = (await outbox.PeekTextAsync())!;
        Console.WriteLine($"hive      restarted and still holds {held}, oldest first: {oldest}");

        // The cellular uplink carries one weight, then drops. A weight leaves the
        // queue only once a link has taken it, so what the uplink never took stays,
        // in order.
        using var cellular = new LoopbackBroker();
        using Transport uplink = Transport.Degraded(cellular.Rung(), up: 1, down: 10);
        await uplink.ConnectAsync();
        int forwarded = 0;
        try
        {
            await outbox.DrainToAsync(uplink, Topic);
        }
        catch (PamojaException error)
        {
            forwarded = held - await outbox.CountAsync();
            Console.WriteLine($"uplink    forwarded {forwarded}, then failed: {error.Message}");
        }

        int left = await outbox.CountAsync();
        string next = (await outbox.PeekTextAsync())!;
        Console.WriteLine($"hive      still holds {left}, oldest first: {next}");

        // The beekeeper's gateway comes within reach, and the scale drains the rest
        // onto it.
        using var visit = new LoopbackBroker();
        using LoopbackTransport gateway = visit.Link();
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync(Topic);
        using Transport toGateway = visit.Rung();
        await toGateway.ConnectAsync();
        await outbox.DrainToAsync(toGateway, Topic);
        var took = new List<string>();
        for (int weight = 0; weight < left; weight++)
        {
            took.Add((await gateway.ReceiveAsync())!.Text);
        }

        Console.WriteLine($"gateway   took {string.Join(", ", took)} when the beekeeper came by");
        int empty = await outbox.CountAsync();
        Console.WriteLine($"hive      holds {empty} once the backlog is through");

        outbox.Dispose();
        dir.Delete(recursive: true);
        // ANCHOR_END: example

        Expect((logged, held, forwarded, left, empty) == (3, 3, 1, 2, 0), "the counts along the way");
        Expect(oldest == "41.2" && next == "41.5", "the queue kept its order");
        Expect(took.SequenceEqual(new[] { "41.5", "40.9" }), "the gateway took the rest in order");
    }
}
