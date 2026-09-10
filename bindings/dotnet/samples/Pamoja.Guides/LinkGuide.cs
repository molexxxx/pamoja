using System.Text;
using System.Threading.Channels;

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
    /// A link over two queues, standing in for a radio or cloud SDK. Nothing about it
    /// names a broker: it needs only the operations the contract asks for, and
    /// <see cref="ReceiveAsync"/> is what makes it a link that delivers rather than an
    /// uplink.
    /// </summary>
    private sealed class QueueLink : IReceivingTransportHandlers
    {
        private readonly Channel<TransportMessage?> _inbox =
            Channel.CreateUnbounded<TransportMessage?>();

        public List<TransportMessage> Sent { get; } = new();

        public List<string> Filters { get; } = new();

        public Task ConnectAsync() => Task.CompletedTask;

        public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
        {
            Sent.Add(new TransportMessage(topic, payload.ToArray()));
            return Task.CompletedTask;
        }

        public Task SubscribeAsync(string topic)
        {
            Filters.Add(topic);
            return Task.CompletedTask;
        }

        public async Task<TransportMessage?> ReceiveAsync() => await _inbox.Reader.ReadAsync();

        /// <summary>
        /// The vendor side: a message arriving from the radio, which the link hands on
        /// in the shape the contract asks for.
        /// </summary>
        /// <param name="topic">The topic it arrived on.</param>
        /// <param name="text">What it carried.</param>
        public void Deliver(string topic, string text) =>
            _inbox.Writer.TryWrite(new TransportMessage(topic, Encoding.UTF8.GetBytes(text)));
    }
    // ANCHOR_END: parts

    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once a command has come back through the ladder.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // The link is a rung like any shipped transport, and the ladder is the link a
        // node is written against.
        var link = new QueueLink();
        using var ladder = new Ladder(Store.Memory());
        ladder.Rung(Transport.FromHandlers(link));
        await ladder.ConnectAsync();

        // A reading out through the ladder lands in the link, topic and bytes intact.
        await ladder.SendAsync("sensors/1", "21.5");
        TransportMessage carried = link.Sent[0];
        Console.WriteLine(
            $"link carried: {carried.Topic} {carried.Text}");

        // A subscription placed on the ladder reaches the link.
        await ladder.SubscribeAsync("commands/#");
        string filter = link.Filters[0];
        Console.WriteLine($"link subscribed to: {filter}");

        // What the link delivers comes back through the ladder.
        link.Deliver("commands/1", "open");
        TransportMessage command = (await ladder.ReceiveAsync())!;
        Console.WriteLine(
            $"command over the ladder: {command.Topic} {command.Text}");
        // ANCHOR_END: example

        Expect(carried.Topic == "sensors/1", "the reading reached the link");
        Expect(carried.Payload.AsSpan().SequenceEqual("21.5"u8), "with its bytes");
        Expect(filter == "commands/#", "the subscription reached the link");
        Expect(command.Topic == "commands/1", "a command came back through the ladder");
        Expect(command.Payload.AsSpan().SequenceEqual("open"u8), "carrying what the link delivered");
    }
}
