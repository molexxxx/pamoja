using System.Text.Json;

using Pamoja.Core;
using Pamoja.Kit;
using Pamoja.Loopback;

using static Guides.Guide;

namespace Guides;

/// <summary>The rules guide example; see docs/guides/rules.md.</summary>
public static class RulesGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the rule has fired both ways.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // A rule is a file: the topic it watches, the line a reading crosses, the release
        // band that stops it firing over and over, and what to do on the way down and on
        // the way back. The same file runs in every language; here the program reads it
        // as data and drives the loop itself, with the kit's trigger deciding the
        // condition.
        using JsonDocument rules = JsonDocument.Parse("""
            { "rules": [ {
              "name": "water-when-dry",
              "when": { "topic": "garden/bed-1/moisture", "compare": "below",
                        "threshold": 30.0, "hysteresis": 5.0 },
              "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
                        { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
              "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                             { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ]
            } ] }
            """);
        JsonElement rule = rules.RootElement.GetProperty("rules")[0];
        JsonElement when = rule.GetProperty("when");
        string topic = when.GetProperty("topic").GetString()!;
        string compare = when.GetProperty("compare").GetString()!;
        float threshold = when.GetProperty("threshold").GetSingle();
        float hysteresis = when.GetProperty("hysteresis").GetSingle();
        Console.WriteLine(
            $"the rule watches {topic} {compare} {threshold}, clearing above {threshold + hysteresis}");

        // Three parties on one broker: the node that reads the bed, the engine that holds
        // the valve, and a watcher on the topic the rule publishes to.
        using var broker = new LoopbackBroker();
        using LoopbackTransport probe = broker.Link();
        using LoopbackTransport engine = broker.Link();
        using LoopbackTransport watcher = broker.Link();
        await probe.ConnectAsync();
        await engine.ConnectAsync();
        await watcher.ConnectAsync();
        await watcher.SubscribeAsync("garden/bed-1/valve");
        await engine.SubscribeAsync(topic);

        // The condition is the trigger; the actions are the program's own.
        using Trigger trigger = compare == "below"
            ? Trigger.Below(threshold, hysteresis)
            : Trigger.Above(threshold, hysteresis);
        bool valveOpen = false;
        int switches = 0;
        async Task Run(JsonElement actions)
        {
            foreach (JsonElement action in actions.EnumerateArray())
            {
                if (action.GetProperty("do").GetString() == "drive")
                {
                    valveOpen = action.GetProperty("on").GetBoolean();
                    switches += 1;
                }
                else
                {
                    await engine.SendAsync(
                        action.GetProperty("topic").GetString()!,
                        action.GetProperty("payload").GetString()!);
                }
            }
        }

        // The bed dries out and is watered back: the rule fires once on the way down and
        // once on the way back, and holds its state for the readings in between.
        foreach (float reading in new[] { 42f, 31f, 28f, 33f, 36f })
        {
            await probe.SendAsync(topic, reading.ToString());
            TransportMessage message = (await engine.ReceiveAsync())!;
            Edge? edge = trigger.Update((float)message.Number!);
            if (edge == Edge.Set)
            {
                await Run(rule.GetProperty("then"));
            }
            else if (edge == Edge.Cleared)
            {
                await Run(rule.GetProperty("otherwise"));
            }

            string edgeText = edge?.ToString().ToLowerInvariant() ?? "no edge";
            Console.WriteLine($"{reading}: {edgeText}, valve {(valveOpen ? "on" : "off")}");
        }

        // The watcher on the other topic heard each edge as the rule published it.
        var heard = new List<string>();
        for (int i = 0; i < 2; i++)
        {
            heard.Add((await watcher.ReceiveAsync())!.Text);
        }

        Console.WriteLine($"the watcher heard {string.Join(", ", heard)}");
        Console.WriteLine($"the valve switched {switches} times");
        // ANCHOR_END: example

        Expect(heard.SequenceEqual(new[] { "open", "closed" }), "the watcher heard both edges");
        Expect(!valveOpen, "the valve ends closed");
        Expect(switches == 2, "and switched once each way");
    }
}
