using System.Globalization;
using System.Text;

using Pamoja.Core;
using Pamoja.Kit;
using Pamoja.Ladder;
using Pamoja.Loopback;
using Pamoja.Sim;
using Pamoja.Sync;

using static Guides.Guide;

namespace Guides;

/// <summary>
/// A part pamoja has never heard of: a soil probe and a valve of the maker's own, run
/// against a rule and published with nothing plugged in.
/// </summary>
public static class DeviceGuide
{
    private const string Topic = "garden/bed-1/moisture";

    // ANCHOR: parts
    /// <summary>
    /// A capacitive soil probe on an analog-to-digital converter. Whatever reads the chip
    /// is <paramref name="readCounts"/>: a replay here, the converter's own driver on the
    /// node. The probe turns counts into percent, and nothing downstream needs to know
    /// there was a chip at all.
    /// </summary>
    private sealed class SoilProbe(Func<Task<float>> readCounts, Calibration calibration)
    {
        public async Task<float> ReadAsync() => calibration.Apply(await readCounts());
    }

    /// <summary>
    /// A solenoid valve on a relay. It keeps what it was last told and counts the
    /// changes, which is what a test needs and what a real one does before it drives
    /// the pin.
    /// </summary>
    private sealed class Valve
    {
        public bool Open { get; private set; }

        public int Switched { get; private set; }

        public Task ApplyAsync(bool open)
        {
            if (open != Open)
            {
                Open = open;
                Switched++;
            }

            return Task.CompletedTask;
        }
    }
    // ANCHOR_END: parts

    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes when the gateway has every reading.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // Bone dry read 3200 counts on this probe and a soaked pot read 1400, measured
        // once and kept. The replay hands back the counts a bed reads as it dries and is
        // watered.
        using var counts = new Replay([2900f, 2700f, 2300f, 2450f, 2750f, 2500f]);
        var probe = new SoilProbe(counts.ReadAsync, Calibration.TwoPoint(3200f, 0f, 1400f, 100f));
        var valve = new Valve();

        // Water below 30% and stop above 45%. A valve that adds water is what `Heating`
        // names, so the band sits at 37.5 with 7.5 either side.
        using var rule = Thermostat.Heating(37.5f, 7.5f);

        // The link, with its first two sends failing the way a radio does at dusk. The
        // ladder keeps what it could not send and replays it in order once a send goes
        // through.
        using var broker = new LoopbackBroker();
        using LoopbackTransport gateway = broker.Link();
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync(Topic);
        using var ladder = new Ladder(Store.Memory());
        ladder.Rung(Transport.Faulty(broker.Rung(), 2));
        await ladder.ConnectAsync();

        // The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
        for (int sample = 0; sample < 6; sample++)
        {
            float moisture = await probe.ReadAsync();
            await valve.ApplyAsync(rule.Update(moisture));
            string report = moisture.ToString("F1", CultureInfo.InvariantCulture);
            Delivery delivery = await ladder.SendAsync(Topic, report);
            string state = valve.Open ? "open" : "closed";
            Console.WriteLine($"bed at {report}%, valve {state}, {delivery}");
            if (await ladder.BufferedAsync() > 0)
            {
                int caughtUp = await ladder.FlushAsync();
                if (caughtUp > 0)
                {
                    Console.WriteLine($"link back, {caughtUp} readings caught up");
                }
            }
        }

        Console.WriteLine($"the valve switched {valve.Switched} times");

        // On the gateway, in the order they were read, outage included.
        List<string> got = [];
        for (int n = 0; n < 6; n++)
        {
            TransportMessage message = (await gateway.ReceiveAsync())!;
            got.Add(message.Text);
        }

        Console.WriteLine($"gateway got {string.Join(", ", got)}");
        // ANCHOR_END: example

        Expect(
            got.SequenceEqual(new[] { "16.7", "27.8", "50.0", "41.7", "25.0", "38.9" }),
            "the gateway has all six readings in the order they were read");
        Expect(valve.Switched == 3, "the valve opened, closed, and opened again");
        Expect(valve.Open, "and is open at the end");
        Expect(await ladder.BufferedAsync() == 0, "nothing is left in the queue");
    }
}
