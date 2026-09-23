using System.Threading.Channels;

using Pamoja;
using Pamoja.Core;
using Pamoja.Ladder;
using Pamoja.Sync;

using static Guides.Guide;

namespace Guides;

/// <summary>The own-link guide example; see docs/guides/link.md.</summary>
public static class LinkGuide
{
    // ANCHOR: parts
    /// <summary>
    /// A cellular modem reached through its vendor's SDK, which this class stands in
    /// for. Nothing in it names a broker or a protocol: it needs only the operations
    /// the contract asks for, and <see cref="ReceiveAsync"/> is what makes it a link
    /// that delivers.
    /// </summary>
    private sealed class Modem : IReceivingTransportHandlers
    {
        private readonly Channel<object> _inbox = Channel.CreateUnbounded<object>();

        public List<TransportMessage> Sent { get; } = new();

        public List<string> Filters { get; } = new();

        public bool NoSignal { get; set; }

        public Task ConnectAsync() => Task.CompletedTask;

        public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
        {
            if (NoSignal)
            {
                return Task.FromException(new IOException("no signal"));
            }

            Sent.Add(new TransportMessage(topic, payload.ToArray()));
            return Task.CompletedTask;
        }

        public Task SubscribeAsync(string topic)
        {
            Filters.Add(topic);
            return Task.CompletedTask;
        }

        public async Task<TransportMessage?> ReceiveAsync() =>
            await _inbox.Reader.ReadAsync() switch
            {
                Exception lost => throw lost,
                var arrived => (TransportMessage)arrived,
            };

        /// <summary>
        /// The vendor's side: a message from the network, or the loss of the session.
        /// </summary>
        /// <param name="arrived">What the SDK hands over.</param>
        public void HandOver(object arrived) => _inbox.Writer.TryWrite(arrived);
    }

    /// <summary>
    /// A satellite messenger sends and never receives. Without
    /// <see cref="IReceivingTransportHandlers"/> it is an uplink, which a ladder never
    /// listens on or subscribes.
    /// </summary>
    private sealed class Satellite : ITransportHandlers
    {
        public List<TransportMessage> Sent { get; } = new();

        public Task ConnectAsync() => Task.CompletedTask;

        public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
        {
            Sent.Add(new TransportMessage(topic, payload.ToArray()));
            return Task.CompletedTask;
        }

        public Task SubscribeAsync(string topic) =>
            Task.FromException(new NotSupportedException("a satellite messenger only sends"));
    }
    // ANCHOR_END: parts

    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the reconnected modem has delivered again.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // The modem is the cheaper link and goes first. The messenger has no
        // ReceiveAsync, so it goes on as an uplink.
        var modem = new Modem();
        var satellite = new Satellite();
        using var ladder = new Ladder(Store.Memory());
        ladder.Rung(Transport.FromHandlers(modem));
        ladder.Rung(Transport.FromHandlers(satellite));
        await ladder.ConnectAsync();

        // A reading goes out over the first link that takes it.
        await ladder.SendAsync("waves/height", "1.8");
        TransportMessage carried = modem.Sent[0];
        Console.WriteLine($"modem     carried {carried.Topic} {carried.Text}");

        // A subscription reaches every link that listens, and only those: the
        // messenger, whose subscribe would fail, is never asked.
        await ladder.SubscribeAsync("commands/#");
        string placed = modem.Filters[0];
        Console.WriteLine($"modem     listens on {placed}, and the satellite was never asked");

        // What the modem hands over comes back through the ladder.
        modem.HandOver(new TransportMessage("commands/interval", "600"));
        TransportMessage command = (await ladder.ReceiveAsync())!;
        Console.WriteLine($"buoy      took {command.Topic} {command.Text}");

        // A link that refuses a send passes the reading down to the next link.
        modem.NoSignal = true;
        await ladder.SendAsync("waves/height", "2.4");
        TransportMessage relayed = satellite.Sent[0];
        Console.WriteLine(
            $"satellite carried {relayed.Topic} {relayed.Text} while the modem had no signal");

        // A link that fails while listening says why, once, and has ended after that.
        // With no other link listening, the ladder then has nothing to wait on.
        modem.HandOver(new IOException("the modem lost its session"));
        string lost = string.Empty;
        try
        {
            await ladder.ReceiveAsync();
        }
        catch (PamojaException error)
        {
            lost = error.Message;
        }

        Console.WriteLine($"buoy      lost the modem: {lost}");
        string idle = string.Empty;
        try
        {
            await ladder.ReceiveAsync();
        }
        catch (PamojaException error)
        {
            idle = error.Message;
        }

        Console.WriteLine($"buoy      has no link left to listen on: {idle}");

        // Connecting again brings the modem back, and the ladder places its filter on
        // it again, so a link's subscribe runs once for every connect.
        await ladder.ConnectAsync();
        Console.WriteLine(
            $"modem     reconnected, and the ladder placed {modem.Filters[1]} on it again");
        modem.HandOver(new TransportMessage("commands/interval", "900"));
        TransportMessage later = (await ladder.ReceiveAsync())!;
        Console.WriteLine($"buoy      took {later.Topic} {later.Text}");
        // ANCHOR_END: example

        Expect(carried.Topic == "waves/height" && carried.Text == "1.8", "the reading reached the modem");
        Expect(modem.Filters.SequenceEqual(new[] { "commands/#", "commands/#" }), "the filter was placed on each connect");
        Expect(command.Text == "600", "a command came back through the ladder");
        Expect(relayed.Text == "2.4", "the satellite took the reading the modem refused");
        Expect(lost == "transport error: the modem lost its session", lost);
        Expect(idle == "resource is closed", idle);
        Expect(later.Text == "900", "the reconnected modem delivered again");
    }
}
