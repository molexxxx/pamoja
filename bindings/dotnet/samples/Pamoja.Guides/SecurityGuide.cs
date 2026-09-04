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
        // The seed is provisioned into the device and never leaves it. This one is
        // RFC 8032 test vector 2, so the key it derives and the signature below are
        // published constants rather than values checked against themselves.
        using var device = new DeviceIdentity(Convert.FromHexString(
            "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"));
        byte[] message = [0x72];
        byte[] published = Convert.FromHexString(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da"
            + "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00");
        Expect(
            device.Sign(message).SequenceEqual(published),
            "the signature is the one the vector publishes");

        // Only the 32-byte public key travels to the gateway.
        byte[] gatewayKey = device.PublicKey;
        Expect(
            DeviceIdentity.FingerprintOf(gatewayKey) == "3d4017c3e843895a",
            "the fingerprint labels the key the vector fixes");

        // Signing is deterministic, so the same reading always yields the same 64 bytes;
        // there is no randomness to get wrong on a microcontroller.
        const string reading = "meter-4 1182.750 kWh";
        byte[] signature = device.Sign(reading);
        Expect(device.Sign(reading).SequenceEqual(signature), "signing is deterministic");
        Expect(DeviceIdentity.Verify(gatewayKey, reading, signature), "the reading is authentic");

        // A digit changed in transit fails, and so does a signature offered under another
        // device's key.
        Expect(
            !DeviceIdentity.Verify(gatewayKey, "meter-4 1082.750 kWh", signature),
            "an altered reading does not verify");
        byte[] impostorSeed = new byte[DeviceIdentity.KeyLength];
        Array.Fill(impostorSeed, (byte)0x5A);
        using var impostor = new DeviceIdentity(impostorSeed);
        Expect(
            !DeviceIdentity.Verify(impostor.PublicKey, reading, signature),
            "another device's key does not verify it either");
        // ANCHOR_END: example
    }
}
