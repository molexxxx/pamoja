using System.Text;

using Pamoja;
using Pamoja.Sync;

using static Guides.Guide;

namespace Guides;

/// <summary>The store-and-forward guide example; see docs/guides/sync.md.</summary>
public static class SyncGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the queue has drained.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // A node with nowhere to send buffers its readings. This queue is held in memory,
        // so it lasts as long as the process; Store.File(dir) is the same queue on disk,
        // which is what a node uses to survive a reboot with its backlog intact.
        using var outbox = Store.Memory();
        foreach (string reading in new[] { "20.1", "20.4", "20.2" })
        {
            await outbox.AppendAsync(Encoding.UTF8.GetBytes(reading));
        }

        Console.WriteLine($"queued    {await outbox.CountAsync()} readings with no link");

        // Peek reads the oldest record without taking it, so a send that fails part-way
        // leaves the queue exactly as it was.
        byte[] oldest = (await outbox.PeekAsync())!;
        Console.WriteLine(
            $"oldest    {Encoding.UTF8.GetString(oldest)}"
            + $" and still {await outbox.CountAsync()} held");

        // The link returns and the queue drains oldest first, in the order the readings
        // were taken rather than the order they happen to come back off a buffer.
        List<string> drained = [];
        while (await outbox.PopAsync() is { } record)
        {
            drained.Add(Encoding.UTF8.GetString(record));
        }

        Console.WriteLine($"drained   {string.Join(", ", drained)}");

        // A bounded queue refuses the append that would overflow it. A full store is
        // backpressure the caller is told about, not a reading dropped behind its back.
        using var bounded = Store.Memory(2);
        await bounded.AppendAsync("20.1"u8.ToArray());
        await bounded.AppendAsync("20.4"u8.ToArray());
        try
        {
            await bounded.AppendAsync("20.2"u8.ToArray());
            Console.WriteLine("a full queue took a third reading, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"full      refused the third reading: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(oldest.AsSpan().SequenceEqual("20.1"u8), "peek reads the oldest record");
        Expect(drained.SequenceEqual(["20.1", "20.4", "20.2"]), "drained oldest first");
        Expect(await outbox.CountAsync() == 0, "leaving the queue empty");
        Expect(await bounded.CountAsync() == 2, "and a full queue keeps what it took");
    }
}
