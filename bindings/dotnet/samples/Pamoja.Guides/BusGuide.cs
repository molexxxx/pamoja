using Pamoja.Bus;

using static Guides.Guide;

namespace Guides;

/// <summary>The event bus guide example; see docs/guides/bus.md.</summary>
public static class BusGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the logger has waited out a quiet bus.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // The station's wiring makes one bus and hands each part what it needs: a
        // publisher to announce, an endpoint to listen. No part holds a reference to
        // another, so any of them can be replaced without touching the rest.
        using var bus = new EventPublisher(2);
        using EventPublisher power = bus.Publisher();
        using EventPublisher sampler = bus.Publisher();
        using EventBus heater = bus.Subscribe();
        using EventBus logger = bus.Subscribe();

        // One announcement reaches every part that listens, and each reads its own copy.
        int reached = power.Publish("battery.low");
        Console.WriteLine($"power     handed battery.low to {reached} parts");
        string heaterTook = await heater.NextTextAsync();
        Console.WriteLine($"heater    took {heaterTook}");
        string loggerTook = await logger.NextTextAsync();
        Console.WriteLine($"logger    took {loggerTook}");

        // Publishing never waits, even while the part's own wait is open, and a part
        // hears what it publishes.
        Task<string> waiting = heater.NextTextAsync();
        heater.Publish("heater.off");
        string heard = await waiting;
        Console.WriteLine($"heater    heard its own {heard}, sent while it waited");

        // A part that joins late sees only what is published after it subscribes.
        // There is no history to replay.
        using EventBus radio = bus.Subscribe();
        power.Publish("battery.ok");
        string first = await radio.NextTextAsync();
        Console.WriteLine($"radio     joined late, so the first event it sees is {first}");

        // Each endpoint buffers two events. The logger, busy writing to flash, falls
        // behind while the sampler publishes five readings: it loses the oldest
        // events, resumes with the newest, and counts what it lost.
        for (int reading = 0; reading < 5; reading++)
        {
            sampler.Publish($"wind {reading}");
        }

        string resumed = await logger.NextTextAsync();
        long missed = logger.Missed;
        Console.WriteLine($"logger    missed {missed} and resumes at {resumed}");
        string newest = await logger.NextTextAsync();
        Console.WriteLine($"logger    then took {newest}");

        // A wait with a limit gives up without taking anything, so a part can do
        // other work between events and lose nothing by it.
        TimeSpan quiet = TimeSpan.FromMilliseconds(50);
        try
        {
            await logger.NextTextAsync(quiet);
            Console.WriteLine("logger    took an event no one published, which should never happen");
        }
        catch (TimeoutException)
        {
            Console.WriteLine($"logger    heard nothing more within {quiet.TotalMilliseconds} ms");
        }
        // ANCHOR_END: example

        Expect(reached == 2, "the announcement reached both listening parts");
        Expect(heaterTook == "battery.low", "the heater took it");
        Expect(loggerTook == "battery.low", "and so did the logger");
        Expect(heard == "heater.off", "a part hears what it publishes");
        Expect(first == "battery.ok", "a late part starts at the next event");
        Expect(missed == 5, "the logger counted what it lost");
        Expect(resumed == "wind 3", "and resumed at the oldest event still buffered");
        Expect(newest == "wind 4", "then took the newest");
    }
}
