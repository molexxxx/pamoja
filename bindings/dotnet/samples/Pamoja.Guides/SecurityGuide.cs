using System.Text;

using Pamoja.Security;

using static Guides.Guide;

namespace Guides;

/// <summary>The device identity guide example; see docs/guides/security.md.</summary>
public static class SecurityGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The seed is provisioned into the device once and never leaves it. A real one
        // comes from the factory or a secure element; any 32 bytes stand in here.
        byte[] seed = new byte[DeviceIdentity.KeyLength];
        Array.Fill(seed, (byte)7);
        using var device = new DeviceIdentity(seed);

        // Only the 32-byte public key travels to the gateway. Its fingerprint is the short
        // form an operator reads off a screen to tell one device from another.
        byte[] gatewayKey = device.PublicKey;
        Console.WriteLine($"device     {DeviceIdentity.FingerprintOf(gatewayKey)}");

        // Signing is deterministic, so the same reading always produces the same 64 bytes
        // and there is no randomness to get wrong on a microcontroller.
        const string reading = "meter-4 1182.750 kWh";
        byte[] signature = device.Sign(reading);
        Console.WriteLine(DeviceIdentity.Verify(gatewayKey, reading, signature)
            ? $"accepted   {reading}"
            : "rejected   a reading the device really did sign, which should never happen");

        // A digit changed in transit no longer matches what was signed.
        const string edited = "meter-4 1082.750 kWh";
        Console.WriteLine(DeviceIdentity.Verify(gatewayKey, edited, signature)
            ? "accepted   an edited reading, which should never happen"
            : $"rejected   {edited}");

        // Nor does the same reading offered under another device's key.
        byte[] impostorSeed = new byte[DeviceIdentity.KeyLength];
        Array.Fill(impostorSeed, (byte)90);
        using var impostor = new DeviceIdentity(impostorSeed);
        Console.WriteLine(DeviceIdentity.Verify(impostor.PublicKey, reading, signature)
            ? "accepted   an impostor, which should never happen"
            : "rejected   a signature offered under another device's key");

        // On a link the signature and the reading usually travel as one message,
        // signature first, and the gateway gets the reading back only once it has
        // checked it.
        byte[] message = device.SignMessage(reading);
        int size = message.Length;
        Console.WriteLine($"message    {size} bytes on the wire, the signature and the reading together");
        byte[]? carried = DeviceIdentity.VerifyMessage(gatewayKey, message);
        Console.WriteLine(carried is not null
            ? $"accepted   {Encoding.UTF8.GetString(carried)}, read out of the message"
            : "rejected   a message the device really did sign, which should never happen");

        // A message that lost its last byte on the way is refused whole.
        byte[]? cut = DeviceIdentity.VerifyMessage(gatewayKey, message.AsSpan(0, size - 1));
        Console.WriteLine(cut is not null
            ? "accepted   a message cut short, which should never happen"
            : "rejected   a message that lost its last byte on the way");
        // ANCHOR_END: example

        Expect(device.Sign(reading).SequenceEqual(signature), "signing is deterministic");
        Expect(DeviceIdentity.Verify(gatewayKey, reading, signature), "the reading is authentic");
        Expect(
            !DeviceIdentity.Verify(gatewayKey, edited, signature),
            "an edited reading does not verify");
        Expect(
            !DeviceIdentity.Verify(impostor.PublicKey, reading, signature),
            "another device's key does not verify it either");
        Expect(size == DeviceIdentity.SignatureLength + reading.Length, "a message is the signature and the reading");
        Expect(
            Encoding.UTF8.GetString(DeviceIdentity.VerifyMessage(gatewayKey, message)!) == reading,
            "the gateway reads the reading back out");
        Expect(
            DeviceIdentity.VerifyMessage(gatewayKey, message.AsSpan(0, size - 1)) is null,
            "a message cut short is refused");
    }
}
