using System.Security.Cryptography;
using System.Text;

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
        Console.WriteLine("agreed    both sides derived a key without sending one");

        // The pump id is authenticated but not encrypted, so a router still reads it while
        // any change to it fails the tag.
        SealedMessage reading = uplink.Seal("flow=41.2"u8, "pump-3"u8);
        string hidden = reading.Ciphertext.SequenceEqual("flow=41.2"u8.ToArray()) ? "still" : "no longer";
        Console.WriteLine($"sealed    counter {reading.Counter}, and what goes on the wire is {hidden} the reading");
        byte[] opened = downlink.Open(reading, "pump-3"u8);
        Console.WriteLine($"opened    {Encoding.UTF8.GetString(opened)}");

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

        // A router that rewrites the pump id breaks the tag, so the gateway refuses the
        // frame rather than file the reading under the wrong pump. A frame that fails to
        // open leaves its counter unused.
        SealedMessage later = uplink.Seal("flow=41.3"u8, "pump-3"u8);
        try
        {
            downlink.Open(later, "pump-4"u8);
            Console.WriteLine("a rewritten pump id was accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"altered   refused: {error.Message}");
        }

        // Radio frames can arrive out of order. The window accepts any counter it has not
        // seen among the 64 below the newest, so the frame that was held up still opens.
        SealedMessage newest = uplink.Seal("flow=41.5"u8, "pump-3"u8);
        string first = Encoding.UTF8.GetString(downlink.Open(newest, "pump-3"u8));
        string second = Encoding.UTF8.GetString(downlink.Open(later, "pump-3"u8));
        Console.WriteLine($"late      counter {newest.Counter} opened first, then counter {later.Counter}: {first}, then {second}");

        // The gateway answers on the same session. Its frames carry the other direction in
        // their nonce, so a reply can never be taken for, or replayed as, one from the node.
        SealedMessage order = downlink.Seal("valve=close"u8, "pump-3"u8);
        string answer = Encoding.UTF8.GetString(uplink.Open(order, "pump-3"u8));
        Console.WriteLine($"reply     {answer}, sealed by the gateway and opened by the node");
        // ANCHOR_END: example

        Expect(hidden == "no longer", "the reading does not travel in the clear");
        Expect(opened.SequenceEqual("flow=41.2"u8.ToArray()), "the gateway recovers it");
    }
}
