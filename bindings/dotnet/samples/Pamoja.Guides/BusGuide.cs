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

        await hub.PublishAsync("battery.low");
        string toControl = (await control.NextTextAsync())!;
        string toLogger = (await logger.NextTextAsync())!;
        Console.WriteLine($"control saw {toControl}, the logger saw {toLogger}");

        // A subscriber taken later starts from the next event, so it never sees what went
        // out before it existed.
        using EventBus late = hub.Subscribe();
        await hub.PublishAsync("link.up");
        string firstSeen = (await late.NextTextAsync())!;
        Console.WriteLine($"the late subscriber's first event is {firstSeen}");

        // The buffer is per subscriber and bounded, so one further behind than the
        // capacity drops what it missed and resumes with the most recent events. A slow
        // reader costs itself, not the publisher.
        using EventBus slow = new EventBus(2);
        using EventBus reader = slow.Subscribe();
        for (int count = 0; count < 5; count++)
        {
            await slow.PublishAsync(count.ToString());
        }

        string resumed = (await reader.NextTextAsync())!;
        Console.WriteLine(
            $"after five events into a buffer of two, the reader resumes at {resumed}");
        // ANCHOR_END: example

        Expect(toControl == "battery.low", "control heard it");
        Expect(toLogger == "battery.low", "and so did the logger");
        Expect(firstSeen == "link.up", "a late subscriber starts fresh");
        Expect(resumed == "3", "a slow reader resumes at the oldest event still buffered");
    }
}
