using System.Globalization;

using Pamoja;
using Pamoja.Core;
using Pamoja.Loopback;
using Pamoja.Profile;

using static Guides.Guide;

namespace Guides;

/// <summary>The rules guide example; see docs/guides/rules.md.</summary>
public static class RulesGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the rules have fired and the mistakes are shown.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // A rule is a file: the topic it watches, the line a reading crosses, the release
        // band that stops it firing over and over, and what to do on the way down and on
        // the way back. Two rules watch one bed here: one waters it when it dries past 30
        // and stops once it is wetter than 35, and one raises an alarm when it is soaked
        // past 60.
        const string file = """
            { "rules": [
              { "name": "water-when-dry",
                "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
                "then": [ { "drive": "bed-valve", "on": true },
                          { "publish": "garden/bed-1/valve", "payload": "open" } ],
                "otherwise": [ { "drive": "bed-valve", "on": false },
                               { "publish": "garden/bed-1/valve", "payload": "closed" } ] },
              { "name": "flood-alarm",
                "when": { "topic": "garden/bed-1/moisture", "above": 60.0, "hysteresis": 5.0 },
                "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
            ] }
            """;

        // Three parties on one broker: the node that reads the bed, the engine that holds
        // the valve, and a watcher on the topics the rules publish to.
        using var broker = new LoopbackBroker();
        using LoopbackTransport probe = broker.Link();
        using LoopbackTransport watcher = broker.Link();
        using LoopbackTransport link = broker.Link();
        await probe.ConnectAsync();
        await watcher.ConnectAsync();
        await link.ConnectAsync();
        await watcher.SubscribeAsync("garden/bed-1/valve");
        await watcher.SubscribeAsync("garden/alarm");

        // The engine runs the file off its link: it listens on every topic a rule watches,
        // switches the outputs it was given by the names the file uses, and publishes over
        // the same link. Here the valve is a list of the settings it was given.
        var valve = new List<bool>();
        using var engine = new RuleEngine(file, link, new Dictionary<string, Func<bool, ValueTask>>
        {
            ["bed-valve"] = on =>
            {
                valve.Add(on);
                return ValueTask.CompletedTask;
            },
        });
        await engine.ListenAsync();
        Console.WriteLine(
            $"watches   {string.Join(", ", engine.Topics)}, and drives {string.Join(", ", engine.Actuators)}");

        static string Described(RuleAction action) => action.Kind == RuleActionKind.Drive
            ? $"drive {action.Actuator} {(action.On == true ? "on" : "off")}"
            : $"publish {action.Payload} to {action.Topic}";

        // The bed dries out, is watered, and floods. A rule fires only as its condition
        // sets or clears, and the readings in between change nothing. At 65 two rules fire
        // on one reading, in the order the file lists them.
        foreach (int reading in new[] { 42, 31, 28, 33, 65, 50 })
        {
            await probe.SendAsync("garden/bed-1/moisture", reading.ToString(CultureInfo.InvariantCulture));
            IReadOnlyList<RuleFired> fired = (await engine.StepAsync(TimeSpan.FromSeconds(5)))!;
            string at = $"{reading,-10}";
            if (fired.Count == 0)
            {
                Console.WriteLine($"{at}nothing fired");
            }

            foreach (RuleFired one in fired)
            {
                string edge = one.Edge == Pamoja.Kit.Edge.Set ? "set" : "cleared";
                Console.WriteLine(one.Actions.Count == 0
                    ? $"{at}{one.Rule} {edge}, with nothing to do"
                    : $"{at}{one.Rule} {edge}: {string.Join(", ", one.Actions.Select(Described))}");
            }
        }

        // The watcher heard every message the rules published, in the order they went out.
        var heard = new List<string>();
        for (int i = 0; i < 3; i++)
        {
            heard.Add((await watcher.ReceiveAsync())!.Text);
        }

        Console.WriteLine($"heard     {string.Join(", ", heard)}");
        Console.WriteLine($"valve     switched {valve.Count} times, and it is {(valve[^1] ? "on" : "off")}");
        // ANCHOR_END: example

        Expect(heard.SequenceEqual(["open", "closed", "waterlogged"]), "the watcher heard every edge");
        Expect(valve.SequenceEqual([true, false]), "the valve opened and closed once");
        Expect(engine.IsSet("water-when-dry") == false, "and the rule has cleared");
        // ANCHOR: wrong
        // A program that moves its own messages hands each reading to an evaluator, the
        // engine's deciding half on its own, and carries out what it says.
        using var judge = RuleEvaluator.FromJson(file);

        // A reading that is not a number, such as the NaN a failed probe reports, is
        // refused on a watched topic rather than leaving every rule as it was with nothing
        // to say why.
        try
        {
            judge.Evaluate("garden/bed-1/moisture", float.NaN);
            Console.WriteLine("a reading of NaN was judged, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"refused   {error.Message}");
        }

        // A topic no rule watches is not judged at all, so even a NaN there says nothing.
        if (judge.Evaluate("garden/bed-2/moisture", float.NaN).Count == 0)
        {
            Console.WriteLine("ignored   no rule watches garden/bed-2/moisture, so even a NaN there is not judged");
        }

        // A file no engine could run is refused as it loads, with the rule and the reason.
        foreach (string edited in new[]
        {
            file.Replace("garden/bed-1/moisture", "garden/+/moisture"),
            file.Replace("\"flood-alarm\"", "\"water-when-dry\""),
        })
        {
            try
            {
                using var accepted = RuleEvaluator.FromJson(edited);
                Console.WriteLine("a file no engine could run was accepted, which should never happen");
            }
            catch (PamojaException error)
            {
                Console.WriteLine($"refused   {error.Message}");
            }
        }

        // With no release band, readings that hover at the line set and clear the rule on
        // every sample, and each edge switches the valve. The band of 5 holds it through
        // them.
        static int Fires(string text)
        {
            using var rules = RuleEvaluator.FromJson(text);
            return new[] { 29.9f, 30.1f, 29.8f, 30.2f }
                .Sum(reading => rules.Evaluate("garden/bed-1/moisture", reading).Count);
        }

        int bare = Fires(file.Replace("\"hysteresis\": 5.0", "\"hysteresis\": 0.0"));
        int banded = Fires(file);
        Console.WriteLine(
            $"chatter   4 readings hovering at 30 fire the rule {bare} times with no release band, {banded} with a band of 5");
        // ANCHOR_END: wrong
    }
}
