using System.Text;

using Pamoja.Codec;

using static Guides.Guide;

namespace Guides;

/// <summary>The codecs guide example; see docs/guides/codec.md.</summary>
public static class CodecGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The same reading as JSON and as CBOR. Nothing is lost, and 21.5 rides as a
        // half-precision float, the shortest form RFC 8949 allows for it.
        byte[] asJson = Encoding.UTF8.GetBytes("{\"c\":21.5,\"ok\":true}");
        byte[] cbor = Codec.JsonToCbor(asJson);
        Console.WriteLine($"json      {asJson.Length} bytes");
        Console.WriteLine($"cbor      {cbor.Length} bytes");

        // A gateway that speaks JSON gets it back unchanged, so the compact form is a
        // transport choice rather than a different data model.
        byte[] restored = Codec.CborToJson(cbor);
        Console.WriteLine($"back to json, unchanged: {restored.SequenceEqual(asJson)}");

        // A batch of readings packs to a count, then the difference between each sample
        // and the one before it. Successive readings differ by very little, so the
        // differences cost about a byte each where the samples would cost eight.
        long[] samples = [10, 11, 13, 12, 900];
        byte[] packed = Codec.PackSamples(samples);
        Console.WriteLine($"batch     {samples.Length} samples in {packed.Length} bytes");
        Console.WriteLine($"unpacked  {string.Join(", ", Codec.UnpackSamples(packed))}");

        // Readings that arrive as floats pack the same way once a scale is chosen. Nothing
        // in the bytes records that scale, so sender and receiver have to agree on it.
        var quantizer = new Quantizer(100.0f);
        float[] celsius = [20.0f, 20.1f, 20.2f, 20.3f];
        byte[] packedCelsius = quantizer.Encode(celsius);
        float[] recovered = quantizer.Decode(packedCelsius);
        Console.WriteLine($"degrees   {celsius.Length} readings in {packedCelsius.Length} bytes");
        Console.WriteLine($"recovered {string.Join(", ", recovered.Select(v => v.ToString("F1")))}");
        // ANCHOR_END: example

        // The bytes each specification fixes are pinned once, in the crate tests and the
        // generated conformance vectors, so a guide asserts behaviour instead.
        Expect(cbor.Length < asJson.Length, "CBOR is the smaller form on the wire");
        Expect(restored.SequenceEqual(asJson), "and it comes back as the same JSON");
        Expect(Codec.UnpackSamples(packed).SequenceEqual(samples), "the batch round-trips");
        Expect(packed.Length < samples.Length * 8, "in fewer bytes than the samples cost");
        Expect(
            recovered.Zip(celsius).All(pair => Math.Abs(pair.First - pair.Second) <= 0.01f),
            "and come back within the scale");
    }
}
