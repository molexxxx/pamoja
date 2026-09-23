using System.Globalization;
using System.Text;

using Pamoja;
using Pamoja.Codec;
using Pamoja.Lora;

using static Guides.Guide;

namespace Guides;

/// <summary>The codecs guide example; see docs/guides/codec.md.</summary>
public static class CodecGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate
        // because it reaches farthest. An uplink there carries only a few bytes of payload.
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Us915);
        int budget = plan.MaxPayload(0)!.Value.Application;
        string Fits(int bytes) => bytes <= budget ? "fits one uplink" : "too big for one uplink";
        Console.WriteLine($"uplink    carries {budget} bytes at the slowest US915 data rate");

        // One reading as the JSON a web service would take. CBOR carries the same
        // document in fewer bytes, but every key name still rides along with every
        // reading.
        byte[] json = Encoding.UTF8.GetBytes("""{"depth_cm":142.5,"air_c":-6.5,"battery_mv":3712}""");
        byte[] cbor = Codec.JsonToCbor(json);
        Console.WriteLine($"json      {json.Length} bytes, {Fits(json.Length)}");
        Console.WriteLine($"cbor      {cbor.Length} bytes, {Fits(cbor.Length)}");
        string restored = Encoding.UTF8.GetString(Codec.CborToJson(cbor));
        Console.WriteLine($"cbor      reads back as {restored}");

        // A batch the gauge and the server agree on needs no key names. Six hourly
        // depths, kept to the millimeter, pack to a count, the first depth, and five
        // small steps.
        var quantizer = new Quantizer(10.0f);
        float[] depths = [142.5f, 143.8f, 145.2f, 146.0f, 145.7f, 145.5f];
        byte[] depthBatch = quantizer.Encode(depths);
        int depthBytes = depthBatch.Length;
        Console.WriteLine($"depths    {depths.Length} readings in {depthBytes} bytes, {Fits(depthBytes)}");
        IEnumerable<string> depthsBack = quantizer.Decode(depthBatch)
            .Select(depth => depth.ToString("F1", CultureInfo.InvariantCulture));
        Console.WriteLine($"depths    read back as {string.Join(", ", depthsBack)}");

        // Battery millivolts are whole numbers already, so they pack with no scale, and
        // a falling voltage packs as small as a rising one.
        long[] battery = [3712, 3709, 3705, 3702, 3698, 3695];
        byte[] batteryBatch = Codec.PackSamples(battery);
        int batteryBytes = batteryBatch.Length;
        Console.WriteLine($"battery   {battery.Length} readings in {batteryBytes} bytes, {Fits(batteryBytes)}");
        Console.WriteLine($"battery   reads back as {string.Join(", ", Codec.UnpackSamples(batteryBatch))}");

        // Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The
        // quantizer refuses the batch rather than send the gap as a depth.
        try
        {
            quantizer.Encode([145.5f, float.NaN]);
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"depths    refused a batch with a missing depth: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(cbor.Length < json.Length, "CBOR is the smaller form on the wire");
        Expect(cbor.Length > budget, "but a document with its keys still misses the uplink");
        Expect(depthBytes <= budget && batteryBytes <= budget, "while each batch fits");
        Expect(restored.StartsWith("{\"air_c\"", StringComparison.Ordinal), "keys come back sorted");
        Expect(
            quantizer.Decode(depthBatch).Zip(depths).All(pair => Math.Abs(pair.First - pair.Second) <= 0.05f),
            "depths come back within the scale");
        Expect(Codec.UnpackSamples(batteryBatch).SequenceEqual(battery), "the battery batch round-trips");
    }
}
