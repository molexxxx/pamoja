using System.Text;

using Pamoja;
using Pamoja.Core;
using Pamoja.Loopback;
using Pamoja.Profile;
using Pamoja.Security;

using static System.FormattableString;
using static Guides.Guide;

namespace Guides;

/// <summary>The device-profile guide example; see docs/guides/profile.md.</summary>
public static class ProfileGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the example has run.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // A profile is a file. This one ships in the catalog under profiles/: it holds a
        // brooder at 32 C by switching a heat lamp, says what it reads, and says how a
        // dashboard draws it.
        string text = File.ReadAllText("profiles/brooder-heater.json");
        using var profile = Profile.FromJson(text);
        Reads reads = profile.Reads!.Value;
        Console.WriteLine($"profile   {profile.Name} reads {reads.Quantity} in {reads.Unit} and reports on {profile.Topic}");

        // A node is the profile and the parts that make it run: a sensor, an output, and a
        // link. A morning of readings stands in for the probe, and a dashboard listens on the
        // same broker.
        using var broker = new LoopbackBroker();
        using var link = broker.Link();
        using var dashboard = broker.Link();
        await link.ConnectAsync();
        await dashboard.ConnectAsync();
        await dashboard.SubscribeAsync(profile.Topic);
        var morning = new Queue<float>([27.5f, 31.8f, 32.6f, 32.1f, 31.4f]);
        bool lamp = false;
        using var node = new Node(
            profile,
            () => ValueTask.FromResult(morning.Dequeue()),
            link,
            drive: on =>
            {
                lamp = on;
                return ValueTask.CompletedTask;
            });

        // Each tick reads, decides, switches the lamp, and publishes the reading. The lamp
        // comes on at 31.5 C or below and goes off at 32.5 C or above, and in between it stays
        // as it was; a reading more than 4 C from 32 raises an alert as well.
        bool was = false;
        for (int at = 0; at < 5; at++)
        {
            Tick tick = await node.TickAsync();
            bool on = tick.Reaction.Actuator == true;
            string change = on ? (was ? "lamp stays on" : "lamp on") : was ? "lamp off" : "lamp stays off";
            string alert = tick.Reaction.Alert is { } raised ? $", alert {raised.Kind}" : string.Empty;
            Console.WriteLine(Invariant($"{Invariant($"{tick.Reading} C"),-10}{change}{alert}"));
            was = on;
        }

        // The dashboard heard every reading the node published.
        var heard = new List<string>();
        for (int at = 0; at < 5; at++)
        {
            TransportMessage? message = await dashboard.ReceiveAsync(TimeSpan.FromSeconds(5));
            heard.Add(Invariant($"{message!.Number}"));
        }

        Console.WriteLine($"heard     {string.Join(", ", heard)} on {profile.Topic}");

        // Between ticks the node waits as long as its battery allows: often on a healthy
        // charge, sparingly on a low one. RunAsync does this until cancelled, waiting each
        // interval.
        foreach (float charge in new[] { 0.8f, 0.3f, 0.1f })
        {
            (var mode, TimeSpan wait) = node.Schedule(charge);
            Console.WriteLine(Invariant($"battery   at {charge * 100:F0}% it runs {mode} and waits {wait.TotalSeconds} s"));
        }

        // The same file says how a dashboard draws the node.
        ElementSpec element = profile.Presentation!.Elements[0];
        string graphic = element.Viz.ToString().ToLowerInvariant();
        Console.WriteLine(Invariant(
            $"draws     {element.Key} in {element.Unit} on a {graphic}, safe from {element.Band![0]} to {element.Band[1]}"));
        // ANCHOR_END: example

        Expect(lamp, "the morning ends with the lamp on");
        Kinds();
        await CustomAsync(text);
        using (Profile delivered = await NetworkAsync(text))
        {
            Expect(delivered.ToJson() == profile.ToJson(), "the signed profile is the one sent");
        }

        Wrong(text, profile);
    }

    private static void Kinds()
    {
        // ANCHOR: kinds
        // A level warns before a tank or a well runs dry. The shipped well profile counts
        // 0.5 m as dry and warns once the last fall puts dry six samples away or nearer.
        using (var wellLevel = Profile.WellLevel())
        using (Controller well = wellLevel.Controller())
        {
            foreach (float depth in new[] { 5.0f, 4.4f, 3.8f })
            {
                Console.WriteLine(well.Evaluate(depth).Alert is { Kind: AlertKind.RunningOut } alert
                    ? Invariant($"well      {depth} m: dry in {alert.Samples} samples at this rate, RunningOut")
                    : Invariant($"well      {depth} m: no warning yet"));
            }
        }

        // A surge warns when a reading moves too far in one sample. The shipped flood sensor
        // warns when a river rises more than 0.3 m between two readings.
        using var floodSensor = Profile.FloodSensor();
        using (Controller river = floodSensor.Controller())
        {
            foreach (float gauge in new[] { 1.2f, 1.35f, 1.9f })
            {
                Console.WriteLine(river.Evaluate(gauge).Alert is { Kind: AlertKind.ChangingFast } alert
                    ? Invariant($"river     {gauge} m: up {alert.Rate:F2} m in one sample, ChangingFast")
                    : Invariant($"river     {gauge} m: no warning"));
            }
        }
        // ANCHOR_END: kinds
    }

    // ANCHOR: custom
    /// <summary>
    /// A policy of the program's own: the lamp on below the setpoint, and a condition of its
    /// own when the chicks are chilled.
    /// </summary>
    private sealed class BrooderGuard(IReadOnlyDictionary<string, object> parameters) : IPolicy
    {
        private readonly float _setpoint = Convert.ToSingle(parameters["setpoint"]);
        private readonly float _chilledBelow = Convert.ToSingle(parameters["setpoint"]) - Convert.ToSingle(parameters["safe_band"]);

        public Reaction Evaluate(float reading) => new(
            reading < _setpoint,
            reading < _chilledBelow ? new Alert(AlertKind.Custom, null, null, null, "Chilled", reading) : null);
    }

    private static async Task CustomAsync(string text)
    {
        // A manifest may name a control kind the library never shipped, with its parameters
        // beside it. The program registers the code that decides it under that name, and the
        // node runs whichever kind the file names.
        using var guarded = Profile.FromJson(text.Replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""));
        var registry = new PolicyRegistry().Register("brooder_guard", parameters => new BrooderGuard(parameters));
        Console.WriteLine($"custom    {guarded.Control.CustomKind} is decided by the program's own code, registered under its name");
        using var broker = new LoopbackBroker();
        using var link = broker.Link();
        await link.ConnectAsync();
        bool lamp = false;
        using var node = new Node(
            guarded,
            () => ValueTask.FromResult(27.5f),
            link,
            drive: on =>
            {
                lamp = on;
                return ValueTask.CompletedTask;
            },
            policy: registry.Resolve(guarded));
        Tick tick = await node.TickAsync();
        string alert = tick.Reaction.Alert?.Code ?? tick.Reaction.Alert?.Kind.ToString() ?? "none";
        Console.WriteLine(Invariant($"{Invariant($"{tick.Reading} C"),-10}lamp {(lamp ? "on" : "off")}, alert {alert}"));
    }
    // ANCHOR_END: custom

    // ANCHOR: network
    private static async Task<Profile> NetworkAsync(string text)
    {
        // A profile can arrive over a link as well as from a disk: on an MQTT topic the
        // gateway publishes to, or as the body of an HTTP response. The gateway signs what it
        // sends and each node holds only the gateway's public key, so a profile is checked
        // before it is read, and one from anywhere else never runs.
        byte[] seed = new byte[DeviceIdentity.KeyLength];
        Array.Fill(seed, (byte)7);
        using var gateway = new DeviceIdentity(seed);
        byte[] trusted = gateway.PublicKey;
        using var broker = new LoopbackBroker();
        using var uplink = broker.Link();
        using var downlink = broker.Link();
        await uplink.ConnectAsync();
        await downlink.ConnectAsync();
        const string fleet = "fleet/brooders/profile";
        await downlink.SubscribeAsync(fleet);

        byte[] manifest = Encoding.UTF8.GetBytes(text);
        await uplink.SendAsync(fleet, gateway.SignMessage(manifest));
        TransportMessage? message = await downlink.ReceiveAsync(TimeSpan.FromSeconds(5));
        byte[]? signed = DeviceIdentity.VerifyMessage(trusted, message!.Payload);
        var delivered = Profile.FromJson(Encoding.UTF8.GetString(signed!));
        Console.WriteLine($"network   {delivered.Name} arrived on {fleet}, signed by the gateway, and loads");

        Array.Fill(seed, (byte)9);
        using var stranger = new DeviceIdentity(seed);
        await uplink.SendAsync(fleet, stranger.SignMessage(manifest));
        message = await downlink.ReceiveAsync(TimeSpan.FromSeconds(5));
        if (DeviceIdentity.VerifyMessage(trusted, message!.Payload) is null)
        {
            Console.WriteLine("network   one signed by any other key is refused before it is read");
        }

        return delivered;
    }
    // ANCHOR_END: network

    private static void Wrong(string text, Profile profile)
    {
        // ANCHOR: wrong
        // A probe that fails reports a reading that is not a number. The controller raises
        // it rather than going quiet, and the lamp holds its state; what off means for the
        // chicks is the node's call.
        using (Controller controller = profile.Controller())
        {
            controller.Evaluate(27.5f);
            Reaction failed = controller.Evaluate(float.NaN);
            if (failed.Alert is { } invalid)
            {
                string holds = failed.Actuator == true ? "on" : "off";
                Console.WriteLine($"probe     a reading of NaN raises {invalid.Kind}, and the lamp holds {holds}");
            }
        }

        // A controller built again for each reading forgets the lamp was on, so inside the
        // deadband it switches the lamp off.
        bool? first;
        bool? then;
        using (Controller once = profile.Controller())
        {
            first = once.Evaluate(27.5f).Actuator;
        }

        using (Controller again = profile.Controller())
        {
            then = again.Evaluate(31.8f).Actuator;
        }

        if (first == true && then == false)
        {
            Console.WriteLine("fresh     built again for each reading, the controller turns the lamp off at 31.8 C");
        }

        // A manifest no node could run is refused as it loads, with the reason. So is a
        // misspelled field, with the one it was probably meant to be and where it sits,
        // rather than leaving the default in its place without a word.
        foreach (string edited in new[]
        {
            text.Replace("\"hysteresis\": 0.5", "\"hysteresis\": 0.0"),
            text.Replace("\"saver_secs\": 600", "\"saver_secs\": 60"),
            text.Replace("\"saver_below\"", "\"saver_bellow\""),
        })
        {
            try
            {
                using var accepted = Profile.FromJson(edited);
                Console.WriteLine("a manifest no node could run was accepted, which should never happen");
            }
            catch (PamojaException error)
            {
                Console.WriteLine($"refused   {error.Message}");
            }
        }

        // A kind the library does not ship loads with its parameters, but no built-in
        // controller decides it, so a node without a registry that knows it is refused
        // rather than running one that never switches the lamp.
        using var unknown = Profile.FromJson(text.Replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""));
        try
        {
            using Controller inert = unknown.Controller();
            Console.WriteLine("a custom kind ran without its policy, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"refused   {error.Message}");
        }
        // ANCHOR_END: wrong
    }
}
