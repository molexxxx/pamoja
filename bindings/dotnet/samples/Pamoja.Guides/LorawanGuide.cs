using Pamoja;
using Pamoja.Lorawan;

using static Guides.Guide;

namespace Guides;

/// <summary>The LoRaWAN activation guide example; see docs/guides/lorawan.md.</summary>
public static class LorawanGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The root key is provisioned into the device at the factory and known to the
        // network server. It is the only secret either side starts with; any 16 bytes
        // stand in here.
        byte[] appKey = new byte[16];
        Array.Fill(appKey, (byte)7);

        // The device asks to join with a nonce it has not used before, which is what stops
        // an old accept being replayed at it.
        const ushort DevNonce = 1;
        using var node = new LorawanDevice(new byte[8], new byte[8], appKey);

        // The network grants the join. It draws its own nonce, names the network the
        // device is joining, and assigns the address it will answer to from then on.
        const uint DevAddr = 0x26012E43;
        var offer = new LorawanGrant(appNonce: 2, netId: 19, devAddr: DevAddr);
        byte[] accept = offer.Accept(appKey, DevNonce);
        Console.WriteLine($"granted   address 0x{DevAddr:X8} in a {accept.Length}-byte accept");

        // The device verifies it against the root key. A join accept carries no device
        // identifier, so only that key decides whether it is for this device.
        using LorawanJoinAccept joined = node.AcceptJoin(accept, DevNonce);
        Console.WriteLine($"joined    the device took address 0x{joined.DevAddr:X8}");

        // Neither side transmits a session key. Both derive the same pair from the root
        // key and the two nonces, so the network reads what the device sends without ever
        // having been told how.
        using LorawanSession network = offer.Session(appKey, DevNonce);
        using LorawanSession activated = joined.Session();
        byte[] uplink = activated.EncodeUplink(1, 1, "level=high"u8);
        LorawanRxData received = network.Decode(uplink, 1);
        Console.WriteLine(
            $"uplink    the network read {System.Text.Encoding.UTF8.GetString(received.Payload)}");

        // A single byte changed in the air fails that check, so no one else can admit the
        // device or put words in its mouth.
        byte[] forged = [.. accept];
        forged[1] ^= 0xFF;
        try
        {
            node.AcceptJoin(forged, DevNonce).Dispose();
            Console.WriteLine("a forged accept was taken, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"forged    accept refused: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(joined.DevAddr == DevAddr, "the device takes the address it was granted");
        Expect(
            received.Payload.AsSpan().SequenceEqual("level=high"u8),
            "and the network reads the reading it sent");
    }
}
