using Pamoja;
using Pamoja.Coap;
using Pamoja.Core;

using static Guides.Guide;

namespace Guides;

/// <summary>The CoAP guide example; see docs/guides/coap.md.</summary>
public static class CoapGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the last row has given up.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // The gateway in the orchard's shed. It takes moisture readings from every row,
        // and holds the irrigation valve's state for the rows to observe. Port 0 lets the
        // system pick a free port, which the rows are pointed at below.
        using var gateway = new CoapServer("127.0.0.1:0");
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync("orchard/+/moisture");
        await gateway.SendAsync("orchard/valve", "closed");
        ushort port = gateway.LocalPort!.Value;
        Console.WriteLine("gateway   takes moisture readings on orchard/+/moisture");

        // A battery-powered sensor in row 7. Its reading is confirmable, so it waits for
        // the gateway's acknowledgment and retransmits until one comes back.
        using var row7 = new CoapClient(new CoapClientOptions
        {
            Host = "127.0.0.1",
            Port = port,
            AckTimeoutMs = 200,
        });
        await row7.ConnectAsync();
        await row7.SendAsync("orchard/row-7/moisture", "31");
        Console.WriteLine("row-7     reported 31, and the gateway acknowledged it");
        TransportMessage reading = (await gateway.ReceiveAsync())!;
        Console.WriteLine($"gateway   took {reading.Text} from {reading.Topic}");

        // Observing the valve registers the row with the gateway, which answers with the
        // valve's state now and notifies every change after it, as RFC 7641 describes.
        await row7.SubscribeAsync("orchard/valve");
        TransportMessage current = (await row7.ReceiveAsync())!;
        Console.WriteLine($"row-7     observes {current.Topic}, which reads {current.Text}");
        await gateway.SendAsync("orchard/valve", "open");
        int observers = gateway.Observers("orchard/valve");
        Console.WriteLine($"gateway   opened the valve for {observers} observer");
        TransportMessage change = (await row7.ReceiveAsync())!;
        Console.WriteLine($"row-7     {change.Topic} now reads {change.Text}");

        // A path the gateway does not take is answered 4.04, and a confirmable send
        // reports that rather than counting the reading as delivered.
        try
        {
            await row7.SendAsync("orchard/row-7/battery", "3.1");
            Console.WriteLine("row-7     the battery reading was taken, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"row-7     battery refused: {error.Message}");
        }

        // Row 8 sends non-confirmable: once and unacknowledged, which costs the least
        // radio time and suits a reading whose loss costs nothing.
        using var row8 = new CoapClient(new CoapClientOptions
        {
            Host = "127.0.0.1",
            Port = port,
            Reliability = Reliability.NonConfirmable,
        });
        await row8.ConnectAsync();
        await row8.SendAsync("orchard/row-8/moisture", "27");
        Console.WriteLine("row-8     sent 27 without waiting for an answer");
        TransportMessage unconfirmed = (await gateway.ReceiveAsync())!;
        Console.WriteLine($"gateway   took {unconfirmed.Text} from {unconfirmed.Topic}");

        // Row 9 is pointed at port 1, where nothing listens. A confirmable send
        // retransmits on a doubling wait and then gives up. RFC 7252's defaults would take
        // more than a minute to get there, so this one waits 20 ms and retransmits once.
        using var row9 = new CoapClient(new CoapClientOptions
        {
            Host = "127.0.0.1",
            Port = 1,
            AckTimeoutMs = 20,
            MaxRetransmits = 1,
        });
        await row9.ConnectAsync();
        try
        {
            await row9.SendAsync("orchard/row-9/moisture", "29");
            Console.WriteLine("row-9     an empty port acknowledged it, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"row-9     gave up unacknowledged: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(reading.Topic == "orchard/row-7/moisture", "row 7's reading arrived");
        Expect(current.Text == "closed", "the observation began with the valve's state");
        Expect(change.Text == "open", "and the change followed");
        Expect(observers == 1, "one row observes the valve");
        Expect(unconfirmed.Topic == "orchard/row-8/moisture", "row 8's reading arrived");
    }
}
