using Pamoja.Codec;
using Pamoja.Core;
using Pamoja.Kit;
using Pamoja.Loopback;
using Pamoja.Security;
using Pamoja.Sensors;

using static Guides.Guide;

namespace Guides;

/// <summary>
/// The first example on the README and the site: a reading taken off a wire on a field
/// node, sent over a link, and checked on the gateway that receives it, with nothing
/// plugged in and nothing running.
/// </summary>
public static class Quickstart
{
    // The device's identity is provisioned once and never leaves it. The gateway is told
    // only the public half, which is how it recognises this device later.
    private const string Topic = "sensors/1/temperature";

    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes when the gateway has checked the reading.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        byte[] seed = new byte[DeviceIdentity.KeyLength];
        Array.Fill(seed, (byte)7);

        // The link. A loopback broker stands in for MQTT or CoAP, so this runs with no
        // network and nothing listening. Point the node at a real transport and nothing
        // below changes.
        using var broker = new LoopbackBroker();
        using LoopbackTransport node = broker.Link();
        using LoopbackTransport gateway = broker.Link();
        await node.ConnectAsync();
        await gateway.ConnectAsync();
        await gateway.SubscribeAsync(Topic);

        using var device = new DeviceIdentity(seed);
        byte[] known = device.PublicKey;
        Console.WriteLine($"gateway trusts device {DeviceIdentity.FingerprintOf(known)}");

        // A stand-in for the thermometer. On a running node these nine bytes arrive from
        // the 1-Wire bus; here the library builds what a part at 25.0625 C would send.
        byte[] offTheBus = Ds18b20.BuildScratchpad(25.0625f, 12, 75, -10);

        // On the node. The part checksums every read, so a value mangled on a long run is
        // an error rather than a plausible temperature a couple of degrees off.
        float celsius = Ds18b20.ParseScratchpad(offTheBus).MicroCelsius / 1e6f;
        Console.WriteLine($"read      {celsius:F4} C");

        // Readings jitter, so smooth them, and send a batch rather than one at a time.
        // Successive readings differ by very little, so the differences cost a fraction of
        // what the readings would on a link that charges by the byte.
        using var smoother = new Smoother(0.5f);
        long[] batch =
        [
            .. new[] { celsius, celsius + 0.5f, celsius + 0.4f }
                .Select(sample => (long)Math.Round(smoother.Update(sample) * 100)),
        ];
        byte[] packed = Codec.PackSamples(batch);
        Console.WriteLine($"packed    {batch.Length} readings into {packed.Length} bytes");

        // Sign the batch and send it. The signature travels with the payload as one
        // message, so there is nothing to keep together and split correctly at the far end.
        await node.SendAsync(Topic, device.SignMessage(packed));

        // On the gateway. Verifying returns the payload, so a reading that was altered on
        // the way, or signed by some other device, never reaches the code that unpacks it.
        TransportMessage? received = await gateway.ReceiveAsync();
        byte[]? payload = DeviceIdentity.VerifyMessage(known, received!.Payload);
        if (payload is null)
        {
            Console.WriteLine("gateway   rejected the reading");
        }
        else
        {
            Console.WriteLine(
                $"gateway   accepted {string.Join(", ", Codec.UnpackSamples(payload))}"
                + " in hundredths of a degree");
        }
        // ANCHOR_END: example

        Expect(celsius == 25.0625f, "the register decodes to 25.0625 C");
        Expect(batch.SequenceEqual(new long[] { 2506, 2531, 2539 }), "smoothing lags the steps");
        Expect(packed.Length < batch.Length * 8, "packing beats eight bytes a sample");
        Expect(received.Topic == Topic, "the message arrives on the topic it was sent to");
        Expect(payload is not null, "and the gateway recognises the device that signed it");
        Expect(
            Codec.UnpackSamples(payload!).SequenceEqual(batch),
            "so the batch it unpacks is the one the node sent");

        // A message edited in transit does not verify, so the gateway never unpacks it.
        byte[] edited = [.. received.Payload];
        edited[^1] ^= 0xFF;
        Expect(DeviceIdentity.VerifyMessage(known, edited) is null, "an edited message fails");
    }
}
