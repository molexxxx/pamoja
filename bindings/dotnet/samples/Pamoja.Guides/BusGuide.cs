using Pamoja.Bus;

using static Guides.Guide;

namespace Guides;

/// <summary>The event bus guide example; see docs/guides/bus.md.</summary>
public static class BusGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the slow reader has resumed.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // A sampler announces something and whatever cares picks it up, with neither side
        // holding a reference to the other. This is how the parts of one node are wired.
        using EventBus hub = new EventBus(8);
        using EventBus control = hub.Subscribe();
        using EventBus logger = hub.Subscribe();

        await hub.PublishAsync("battery.low"u8.ToArray());
        byte[] toControl = (await control.NextAsync())!;
        byte[] toLogger = (await logger.NextAsync())!;
        Console.WriteLine(
            $"control saw {System.Text.Encoding.UTF8.GetString(toControl)},"
            + $" the logger saw {System.Text.Encoding.UTF8.GetString(toLogger)}");

        // A subscriber taken later starts from the next event, so it never sees what went
        // out before it existed.
        using EventBus late = hub.Subscribe();
        await hub.PublishAsync("link.up"u8.ToArray());
        byte[] firstSeen = (await late.NextAsync())!;
        Console.WriteLine(
            $"the late subscriber's first event is"
            + $" {System.Text.Encoding.UTF8.GetString(firstSeen)}");

        // The buffer is per subscriber and bounded, so one further behind than the
        // capacity drops what it missed and resumes with the most recent events. A slow
        // reader costs itself, not the publisher.
        using EventBus slow = new EventBus(2);
        using EventBus reader = slow.Subscribe();
        for (byte count = 0; count < 5; count++)
        {
            await slow.PublishAsync(new byte[] { count });
        }

        byte[] resumed = (await reader.NextAsync())!;
        Console.WriteLine(
            $"after five events into a buffer of two, the reader resumes at {resumed[0]}");
        // ANCHOR_END: example

        Expect(toControl.AsSpan().SequenceEqual("battery.low"u8), "control heard it");
        Expect(toLogger.AsSpan().SequenceEqual("battery.low"u8), "and so did the logger");
        Expect(firstSeen.AsSpan().SequenceEqual("link.up"u8), "a late subscriber starts fresh");
        Expect(resumed[0] == 3, "a slow reader resumes at the oldest event still buffered");
    }
}
