using Pamoja;
using Pamoja.Coap;

using static Guides.Guide;

namespace Guides;

/// <summary>The CoAP guide example; see docs/guides/coap.md.</summary>
public static class CoapGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the command has given up.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // CoAP runs over UDP and opens no session, so connecting only binds a local
        // socket. Nothing is listening on the far side here, and for a non-confirmable
        // send nothing needs to be.
        using var reporter = new CoapClient(new CoapClientOptions
        {
            Host = "127.0.0.1",
            Port = 5683,
            Reliability = Reliability.NonConfirmable,
        });
        await reporter.ConnectAsync();
        Console.WriteLine($"reporter  connected: {await reporter.IsConnectedAsync()}");

        // Non-confirmable delivery is at most once: the datagram leaves unacknowledged,
        // which is what a battery-powered node sends when a missed reading costs nothing.
        await reporter.SendAsync("sensors/1/temperature", "21.5"u8.ToArray());
        Console.WriteLine("reporter  sent 21.5 and did not wait for an answer");

        // A command is different: it has to arrive. Confirmable delivery retransmits until
        // an acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait
        // and four retransmissions; both are cut short here so the guide does not sit
        // waiting.
        using var commander = new CoapClient(new CoapClientOptions
        {
            Host = "127.0.0.1",
            Port = 5683,
            Reliability = Reliability.Confirmable,
            AckTimeoutMs = 20,
            MaxRetransmits = 1,
        });
        await commander.ConnectAsync();
        try
        {
            await commander.SendAsync("actuators/valve", "open"u8.ToArray());
            Console.WriteLine("commander the valve acknowledged the command");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"commander gave up unacknowledged: {error.Message}");
        }

        await reporter.DisconnectAsync();
        Console.WriteLine($"reporter  disconnected: {!await reporter.IsConnectedAsync()}");
        // ANCHOR_END: example

        Expect(!await reporter.IsConnectedAsync(), "a disconnected endpoint says so");
    }
}
