using System.Globalization;

using Pamoja.Codec;
using Pamoja.Kit;
using Pamoja.Security;
using Pamoja.Sensors;

using static Guides.Guide;

namespace Guides;

/// <summary>
/// The first example on the README and the site: one field node's reading taken off a
/// wire, smoothed, signed, and packed for a link that charges by the byte, start to
/// finish with nothing plugged in.
/// </summary>
public static class Quickstart
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A stand-in for the thermometer. On a running node these nine bytes arrive from
        // the 1-Wire bus; here the library builds what a part sitting at 25.0625 C would
        // send, so the program runs with nothing plugged in.
        byte[] offTheBus = Ds18b20.BuildScratchpad(25.0625f, 12, 75, -10);

        // Everything below is the node's own code, and none of it cares where the bytes
        // came from. The part checksums every read, so a value mangled on a long run comes
        // back as an error instead of a plausible temperature a couple of degrees off.
        float celsius = Ds18b20.ParseScratchpad(offTheBus).MicroCelsius / 1e6f;
        Console.WriteLine($"read      {celsius:F4} C"); // read      25.0625 C

        // Readings jitter. A smoother follows the trend without keeping a history to do
        // it, which matters on a part with kilobytes of RAM.
        using var smoother = new Smoother(0.5f);
        smoother.Update(celsius);
        float smoothed = smoother.Update(celsius + 1.0f);
        Console.WriteLine($"smoothed  {smoothed:F4} C"); // smoothed  25.5625 C

        // Sign it, so the gateway can tell this device's readings from anyone else's.
        byte[] seed = new byte[DeviceIdentity.KeyLength];
        Array.Fill(seed, (byte)7);
        using var device = new DeviceIdentity(seed);
        string reading = smoothed.ToString("F2", CultureInfo.InvariantCulture);
        byte[] signature = device.Sign(reading);
        if (!DeviceIdentity.Verify(device.PublicKey, reading, signature))
        {
            throw new InvalidOperationException("the gateway would reject this reading");
        }

        Console.WriteLine($"signed    {reading} C, and the signature checks out");

        // Send a batch rather than a reading at a time. Successive samples differ by very
        // little, so writing down the differences costs a fraction of eight bytes each.
        long[] batch = [2506, 2507, 2509, 2508, 2510];
        byte[] packed = Codec.PackSamples(batch);
        Console.WriteLine($"packed    {batch.Length} readings into {packed.Length} bytes");
        // ANCHOR_END: example

        Expect(celsius == 25.0625f, "the register decodes to 25.0625 C");
        Expect(smoothed > celsius && smoothed < celsius + 1.0f, "smoothing lags the step");
        Expect(packed.Length < batch.Length * 8, "packing beats eight bytes a sample");
        Expect(Codec.UnpackSamples(packed).SequenceEqual(batch), "and the batch round-trips");
    }
}
