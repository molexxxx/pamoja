using Pamoja;
using Pamoja.Coap;

using static Guides.Guide;

namespace Guides;

/// <summary>The CoAP guide example; see docs/guides/coap.md.</summary>
public static class CoapGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the example has run.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // CoAP runs over UDP and opens no session, so connecting only binds a local
        // socket. Nothing is listening on the far side here, and nothing needs to be.
        using var reporter = new CoapClient(new CoapClientOptions
        {
            Host = "127.0.0.1",
            Port = 5683,
            Reliability = Reliability.NonConfirmable,
        });
        Expect(!await reporter.IsConnectedAsync(), "a fresh endpoint holds no socket");
        await reporter.ConnectAsync();
        Expect(await reporter.IsConnectedAsync(), "connecting binds the local socket");

        // Non-confirmable delivery is at most once: the datagram leaves unacknowledged,
        // which is what a battery-powered node sends when one missed reading costs
        // nothing.
        await reporter.SendAsync("sensors/1/temperature", "21.5"u8.ToArray());

        // Confirmable delivery retransmits until an ACK arrives. RFC 7252 fixes the
        // defaults at a two-second wait and four retransmissions; both are cut short here.
        using var commander = new CoapClient(new CoapClientOptions
        {
            Host = "127.0.0.1",
            Port = 5683,
            Reliability = Reliability.Confirmable,
            AckTimeoutMs = 20,
            MaxRetransmits = 1,
        });
        await commander.ConnectAsync();

        bool unacknowledged = false;
        try
        {
            await commander.SendAsync("actuators/valve", "open"u8.ToArray());
        }
        catch (PamojaException)
        {
            unacknowledged = true;
        }

        Expect(unacknowledged, "an unacknowledged command is reported, not dropped");

        await reporter.DisconnectAsync();
        Expect(!await reporter.IsConnectedAsync(), "disconnecting releases the socket");
        // ANCHOR_END: example
    }
}
