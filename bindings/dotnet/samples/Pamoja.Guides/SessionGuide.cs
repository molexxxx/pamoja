using System.Security.Cryptography;

using Pamoja;
using Pamoja.Session;

using static Guides.Guide;

namespace Guides;

/// <summary>The secured session guide example; see docs/guides/session.md.</summary>
public static class SessionGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // Each device is provisioned with a 32-byte seed and publishes the key it derives.
        // A real seed comes from the factory or a secure element; any 32 bytes stand in.
        byte[] nodeSeed = new byte[32];
        Array.Fill(nodeSeed, (byte)7);
        byte[] gatewaySeed = new byte[32];
        Array.Fill(gatewaySeed, (byte)9);
        using var node = new AgreementKey(nodeSeed);
        using var gateway = new AgreementKey(gatewaySeed);

        // Neither side sends the session key. Both derive it from the shared secret, a
        // salt that travels in the clear, and both public keys, with opposite roles.
        //
        // The salt must be fresh for every session: reusing one derives the same key from
        // the same pair of devices twice. The initiator draws it and sends it in the
        // clear, so the responder uses the salt it received rather than one of its own.
        byte[] salt = RandomNumberGenerator.GetBytes(16);
        using var uplink = new Session(node, gateway.PublicKey, salt, SessionRole.Initiator);
        using var downlink = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);
        Console.WriteLine("both sides derived a key without sending one");

        // The pump id is authenticated but not encrypted, so a router still reads it while
        // any change to it fails the tag.
        SealedMessage reading = uplink.Seal("flow=41.2"u8, "pump-3"u8);
        bool hidden = !reading.Ciphertext.SequenceEqual("flow=41.2"u8.ToArray());
        Console.WriteLine($"sealed    the reading is no longer readable: {hidden}");
        byte[] opened = downlink.Open(reading, "pump-3"u8);
        Console.WriteLine($"opened    {System.Text.Encoding.UTF8.GetString(opened)}");

        // The anti-replay window refuses a counter it has already accepted, so a frame
        // captured off the air and sent again is not delivered a second time.
        try
        {
            downlink.Open(reading, "pump-3"u8);
            Console.WriteLine("a replayed frame was accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"replay    refused: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(hidden, "the reading does not travel in the clear");
        Expect(opened.SequenceEqual("flow=41.2"u8.ToArray()), "the gateway recovers it");
    }
}
