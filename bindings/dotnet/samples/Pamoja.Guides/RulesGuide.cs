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
                "when": { "topic": "garden/bed-1/moisture", "compare": "below",
                          "threshold": 30.0, "hysteresis": 5.0 },
                "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
                          { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
                "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                               { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ] },
              { "name": "flood-alarm",
                "when": { "topic": "garden/bed-1/moisture", "compare": "above",
                          "threshold": 60.0, "hysteresis": 5.0 },
                "then": [ { "do": "publish", "topic": "garden/alarm", "payload": "waterlogged" } ] }
            ] }
            """;

        // The evaluator judges each reading and says what the rules call for; the program
        // moves the messages and holds the valve, which is the engine's work in Rust.
        using var evaluator = RuleEvaluator.FromJson(file);
        Console.WriteLine(
            $"watches   {string.Join(", ", evaluator.Topics)}, and drives {string.Join(", ", evaluator.Actuators)}");

        // Three parties on one broker: the node that reads the bed, the program that holds
        // the valve, and a watcher on the topics the rules publish to.
        using var broker = new LoopbackBroker();
        using LoopbackTransport probe = broker.Link();
        using LoopbackTransport link = broker.Link();
        using LoopbackTransport watcher = broker.Link();
        await probe.ConnectAsync();
        await link.ConnectAsync();
        await watcher.ConnectAsync();
        await watcher.SubscribeAsync("garden/bed-1/valve");
        await watcher.SubscribeAsync("garden/alarm");
        foreach (string topic in evaluator.Topics)
        {
            await link.SubscribeAsync(topic);
        }

        var valve = new List<bool>();
        static string Described(RuleAction action) => action.Kind == RuleActionKind.Drive
            ? $"drive {action.Actuator} {(action.On == true ? "on" : "off")}"
            : $"publish {action.Payload} to {action.Topic}";

        // The bed dries out, is watered, and floods. A rule fires only as its condition
        // sets or clears, and the readings in between change nothing. At 65 two rules fire
        // on one reading, in the order the file lists them.
        foreach (int reading in new[] { 42, 31, 28, 33, 65, 50 })
        {
            await probe.SendAsync("garden/bed-1/moisture", reading.ToString());
            TransportMessage message = (await link.ReceiveAsync())!;
            IReadOnlyList<RuleFired> fired = evaluator.Evaluate(message.Topic, (float)message.Number!);
            string at = $"{reading,-10}";
            if (fired.Count == 0)
            {
                Console.WriteLine($"{at}nothing fired");
            }

            foreach (RuleFired one in fired)
            {
                foreach (RuleAction action in one.Actions)
                {
                    if (action.Kind == RuleActionKind.Drive)
                    {
                        valve.Add(action.On == true);
                    }
                    else
                    {
                        await link.SendAsync(action.Topic!, action.Payload!);
                    }
                }

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
        Expect(evaluator.IsSet("water-when-dry") == false, "and the rule has cleared");

        // ANCHOR: wrong
        // A reading that is not a number, such as the NaN a failed probe reports, is
        // refused on a watched topic rather than leaving every rule as it was with nothing
        // to say why.
        using var judge = RuleEvaluator.FromJson(file);
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
