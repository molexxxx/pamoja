using Pamoja.Bus;

using static Guides.Guide;

namespace Guides;

/// <summary>The event bus guide example; see docs/guides/bus.md.</summary>
public static class BusGuide
{
    /// <summary>Runs the example.</summary>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // A sampler announces a reading and whatever cares about readings picks it up,
        // with neither side holding a reference to the other.
        using EventBus hub = new EventBus(8);
        using EventBus sampler = hub.Subscribe();
        using EventBus logger = hub.Subscribe();

        await hub.PublishAsync("battery.low"u8.ToArray());
        Expect(
            (await sampler.NextAsync())!.AsSpan().SequenceEqual("battery.low"u8),
            "the sampler's endpoint received the event");
        Expect(
            (await logger.NextAsync())!.AsSpan().SequenceEqual("battery.low"u8),
            "and so did the logger's");

        // An endpoint taken later starts from the next event, so it never sees what went
        // out before it existed.
        using EventBus late = hub.Subscribe();
        await hub.PublishAsync("link.up"u8.ToArray());
        Expect(
            (await late.NextAsync())!.AsSpan().SequenceEqual("link.up"u8),
            "the endpoint taken last begins at the event after it");
        Expect(
            (await sampler.NextAsync())!.AsSpan().SequenceEqual("link.up"u8),
            "an endpoint that was already there follows on in order");

        // The buffer is per endpoint and bounded, so an endpoint further behind than the
        // capacity drops what it missed and resumes with the most recent events.
        using EventBus slow = new EventBus(2);
        using EventBus reader = slow.Subscribe();
        for (byte count = 0; count < 5; count++)
        {
            await slow.PublishAsync(new byte[] { count });
        }

        Expect((await reader.NextAsync())![0] == 3, "the events it fell behind on were dropped");
        Expect((await reader.NextAsync())![0] == 4, "and it resumes with the most recent");
        // ANCHOR_END: example
    }
}
