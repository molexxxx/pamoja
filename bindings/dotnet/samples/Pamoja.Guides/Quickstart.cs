using System.Globalization;

using Pamoja.Codec;
using Pamoja.Kit;
using Pamoja.Security;
using Pamoja.Sensors;

using static Guides.Guide;

namespace Guides;

/// <summary>
/// The first example on the README and the site: a reading off a wire, smoothed,
/// signed, and packed for a metered link, with nothing plugged in.
/// </summary>
public static class Quickstart
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
        byte[] scratchpad = [0x91, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
        scratchpad[8] = Ds18b20.Crc8(scratchpad.AsSpan(0, 8));
        float celsius = Ds18b20.ParseScratchpad(scratchpad).MicroCelsius / 1e6f;
        Expect(celsius == 25.0625f, "the register decodes to 25.0625 C");

        // Smooth the noise out of successive readings.
        using var smoother = new Smoother(0.5f);
        smoother.Update(celsius);
        float smoothed = smoother.Update(celsius + 1.0f);
        Expect(smoothed > celsius && smoothed < celsius + 1.0f, "smoothing lags the step");

        // Sign the reading so a gateway can prove which device sent it.
        byte[] seed = new byte[DeviceIdentity.KeyLength];
        Array.Fill(seed, (byte)7);
        using var device = new DeviceIdentity(seed);
        string payload = smoothed.ToString("F2", CultureInfo.InvariantCulture);
        byte[] signature = device.Sign(payload);
        Expect(DeviceIdentity.Verify(device.PublicKey, payload, signature), "the signature verifies");

        // Pack a batch of readings for a link where every byte costs money.
        long[] samples = [2506, 2507, 2509, 2508, 2510];
        byte[] packed = Codec.PackSamples(samples);
        Expect(packed.Length < samples.Length * 8, "packing beats eight bytes a sample");
        Expect(Codec.UnpackSamples(packed).SequenceEqual(samples), "and the batch round-trips");
        // ANCHOR_END: example
    }
}
