using Pamoja;
using System.Security.Cryptography;

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
        // Each device is provisioned with a 32-byte seed and publishes the key it
        // derives. These are the X25519 pair RFC 7748 section 6.1 publishes, so the
        // derivation is pinned to the specification rather than checked against itself.
        using var node = new AgreementKey(Convert.FromHexString(
            "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a"));
        using var gateway = new AgreementKey(Convert.FromHexString(
            "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb"));
        Expect(
            Convert.ToHexString(node.PublicKey).ToLowerInvariant()
                == "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a",
            "the public key is the one the vector publishes");

        // Neither side sends the session key. Both derive it from the shared secret, a
        // salt that travels in the clear, and both public keys. The roles are opposite.
        // The salt must be fresh for every session: reusing one derives the same key from
        // the same pair of devices twice. The initiator draws it and sends it in the clear,
        // so the responder here uses the salt it received rather than one of its own.
        byte[] salt = RandomNumberGenerator.GetBytes(16);
        using var uplink = new Session(node, gateway.PublicKey, salt, SessionRole.Initiator);
        using var downlink = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);

        // The pump id is authenticated but not encrypted, so a router still reads it
        // while any change to it fails the tag.
        SealedMessage reading = uplink.Seal("flow=41.2"u8, "pump-3"u8);
        Expect(
            !reading.Ciphertext.SequenceEqual("flow=41.2"u8.ToArray()),
            "the reading does not travel in the clear");
        Expect(
            downlink.Open(reading, "pump-3"u8).SequenceEqual("flow=41.2"u8.ToArray()),
            "the gateway recovers the reading");

        // The anti-replay window refuses a counter it has already accepted, so a frame
        // captured off the air and sent again is not delivered a second time.
        bool refused = false;
        try
        {
            downlink.Open(reading, "pump-3"u8);
        }
        catch (PamojaException)
        {
            refused = true;
        }
        Expect(refused, "the same message is refused a second time");
        // ANCHOR_END: example
    }
}
