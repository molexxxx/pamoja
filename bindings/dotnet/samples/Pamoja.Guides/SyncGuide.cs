using Pamoja;
using Pamoja.Sync;

using static Guides.Guide;

namespace Guides;

/// <summary>The store-and-forward guide example; see docs/guides/sync.md.</summary>
public static class SyncGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the example has run.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // A node with nowhere to send buffers its readings. This queue is held in memory,
        // so it lasts as long as the process; Store.File(dir) is the same queue on disk.
        using var outbox = Store.Memory();
        foreach (var reading in new[] { "20.1", "20.4", "20.2" })
        {
            await outbox.AppendAsync(System.Text.Encoding.UTF8.GetBytes(reading));
        }

        Expect(await outbox.CountAsync() == 3, "three readings are queued");

        // Peek reads the oldest record without taking it, so a send that fails part-way
        // leaves the queue exactly as it was.
        Expect(
            (await outbox.PeekAsync())?.AsSpan().SequenceEqual("20.1"u8) == true,
            "peek returns the oldest record");
        Expect(await outbox.CountAsync() == 3, "and leaves it in the queue");

        // The link returns and the queue drains oldest first, in the order the readings
        // were taken rather than the order they happen to come off a heap.
        var drained = new List<string>();
        while (await outbox.PopAsync() is { } record)
        {
            drained.Add(System.Text.Encoding.UTF8.GetString(record));
        }

        Expect(drained.SequenceEqual(["20.1", "20.4", "20.2"]), "drained oldest first");
        Expect(await outbox.CountAsync() == 0, "leaving the queue empty");

        // A bounded store refuses the append that would overflow it. A full queue is
        // backpressure the caller is told about, not a reading dropped behind its back.
        using var bounded = Store.Memory(2);
        await bounded.AppendAsync("20.1"u8.ToArray());
        await bounded.AppendAsync("20.4"u8.ToArray());

        bool refused = false;
        try
        {
            await bounded.AppendAsync("20.2"u8.ToArray());
        }
        catch (PamojaException)
        {
            refused = true;
        }

        Expect(refused, "a full store refuses rather than drops");
        Expect(await bounded.CountAsync() == 2, "and keeps what it already holds");
        // ANCHOR_END: example
    }
}
