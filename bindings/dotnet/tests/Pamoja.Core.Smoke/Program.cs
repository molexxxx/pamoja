// Smoke test: confirms the facade loads, the native core is reachable, and each
// capability behaves through it (no broker or hardware required).
using System.Text;
using System.Text.Json;

using Pamoja.Core;

string version = PamojaCore.Version;
Console.WriteLine($"pamoja version: {version}");
Assert(!string.IsNullOrEmpty(version), "version should be a non-empty string");

Assert((int)Qos.AtLeastOnce == 1, "Qos should expose protocol levels");

await using var client = new MqttClient(new MqttClientOptions
{
    ClientId = "smoke",
    Host = "127.0.0.1",
    Port = 47811,
    KeepAliveSecs = 1,
});

Assert(!await client.IsConnectedAsync(), "a fresh client should not be connected");

try
{
    await client.ConnectAsync();
    Fail("connecting to a closed port should throw");
}
catch (PamojaException error)
{
    Assert(
        error.Message.Contains("transport error", StringComparison.Ordinal),
        $"expected a transport error, got: {error.Message}");
}

Assert(!await client.IsConnectedAsync(), "a failed connect should leave the client disconnected");

Identity();
Codecs();
Helpers();

Console.WriteLine("ok");

Conformance();

// Signing a payload and checking it, the way a gateway verifies a reading.
static void Identity()
{
    byte[] seed = new byte[DeviceIdentity.KeyLength];
    Array.Fill(seed, (byte)7);

    using var device = new DeviceIdentity(seed);
    byte[] publicKey = device.PublicKey;
    Assert(publicKey.Length == 32, "a public key should be 32 bytes");

    byte[] signature = device.Sign("21.5");
    Assert(signature.Length == 64, "a signature should be 64 bytes");
    Assert(DeviceIdentity.Verify(publicKey, "21.5", signature), "a signature should verify");
    Assert(
        !DeviceIdentity.Verify(publicKey, "21.6", signature),
        "a tampered payload should not verify");

    string fingerprint = device.Fingerprint;
    Assert(fingerprint.Length == 16, "a fingerprint is 16 characters");
    Assert(
        fingerprint.All(character => "0123456789abcdef".Contains(character, StringComparison.Ordinal)),
        "a fingerprint is lowercase hex");
    Assert(
        DeviceIdentity.FingerprintOf(publicKey) == fingerprint,
        "the same key gives the same fingerprint");
}

// Moving a document to the compact form a metered link should carry, and back.
static void Codecs()
{
    // Keys are written in sorted order here because the transcoder canonicalizes
    // them, so this document survives a round trip byte for byte.
    byte[] json = Encoding.UTF8.GetBytes("{\"c\":21.5,\"id\":\"probe-1\"}");
    byte[] cbor = Codec.JsonToCbor(json);
    Assert(cbor.Length < json.Length, "CBOR should be smaller than the JSON it came from");
    Assert(
        Encoding.UTF8.GetString(Codec.CborToJson(cbor)) == Encoding.UTF8.GetString(json),
        "a document should round-trip");

    byte[] unsorted = Codec.JsonToCbor(Encoding.UTF8.GetBytes("{\"id\":\"probe-1\",\"c\":21.5}"));
    Assert(
        Encoding.UTF8.GetString(Codec.CborToJson(unsorted)) == Encoding.UTF8.GetString(json),
        "object keys come back sorted, so the encoding is canonical");

    try
    {
        Codec.JsonToCbor(Encoding.UTF8.GetBytes("not json"));
        Fail("malformed JSON should throw");
    }
    catch (PamojaException)
    {
    }

    long[] samples = [10, 11, 13, 12, 900];
    Assert(Codec.UnpackSamples(Codec.PackSamples(samples)).SequenceEqual(samples),
        "samples should round-trip");

    var quantizer = new Quantizer(100.0f);
    float[] readings = [20.0f, 20.1f, 20.2f, 20.3f];
    byte[] packed = quantizer.Encode(readings);
    Assert(packed.Length < readings.Length * 4, "packed readings should beat four bytes each");
    float[] restored = quantizer.Decode(packed);
    for (int i = 0; i < readings.Length; i++)
    {
        Assert(Math.Abs(restored[i] - readings[i]) < 0.05f, "readings decode to precision");
    }
}

// The helper math a field node runs between reading a sensor and acting on it.
static void Helpers()
{
    using var smoother = new Smoother(0.5f);
    Assert(smoother.Value is null, "a fresh smoother has no value");
    smoother.Update(10.0f);
    float smoothed = smoother.Update(20.0f);
    Assert(smoothed > 10.0f && smoothed < 20.0f, "smoothing should lag the step");
    smoother.Reset();
    Assert(smoother.Value is null, "reset should clear the value");

    using var fridge = Thermostat.Cooling(8.0f, 1.0f);
    Assert(!fridge.Update(7.0f), "a cool fridge leaves the compressor off");
    Assert(fridge.Update(9.5f), "a warm fridge switches the compressor on");
    Assert(fridge.IsOn, "the thermostat reports its state");

    using var tank = new Depletion(10.0f);
    Assert(tank.Update(100.0f) is null, "the first reading sets no rate");
    Assert(tank.Update(90.0f) > 0, "a falling level projects a countdown");

    using var probe = Calibration.TwoPoint(0.0f, 0.0f, 1024.0f, 100.0f);
    Assert(Math.Abs(probe.Apply(512.0f) - 50.0f) < 0.01f, "a two-point fit maps its midpoint");

    Assert(Kit.Deadband(0.2f, 0.0f, 0.5f) == 0.0f, "noise inside the band does not act");

    var centre = new Coordinate(-1.2921, 36.8219);
    var away = new Coordinate(-1.2930, 36.8219);
    using var pen = new Geofence(centre, 50.0);
    Assert(pen.Update(centre) == Boundary.Inside, "the first fix is inside");
    Assert(pen.Update(away) == Boundary.Exited, "the crossing fix reports once");
    Assert(pen.Update(away) == Boundary.Outside, "later fixes stay outside");
    Assert(!pen.Contains(away), "the fix is outside the fence");
    Assert(Kit.DistanceBetween(centre, away) > 50.0, "the fix is beyond the radius");
}

static void Assert(bool condition, string message)
{
    if (!condition)
    {
        Fail(message);
    }
}

static void Fail(string message)
{
    Console.Error.WriteLine($"assertion failed: {message}");
    Environment.Exit(1);
}

// The .NET side of the cross-language conformance suite: the same vectors every
// other binding runs, so a facade that drifts here fails rather than quietly
// disagreeing with Rust, Node, and Python.
static void Conformance()
{
    using JsonDocument document = JsonDocument.Parse(
        File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "vectors.json")));
    JsonElement vectors = document.RootElement;

    // The vectors carry f32 values widened to f64, so they compare exactly; the
    // tolerance covers the accumulation order of the iterative helpers.
    double tolerance = vectors.GetProperty("tolerance").GetDouble();

    ConformIdentity(vectors.GetProperty("identity"));
    ConformCodec(vectors.GetProperty("codec"));
    ConformHelpers(vectors, tolerance);
    ConformGeofence(vectors.GetProperty("geofence"));

    Console.WriteLine("conformance ok");
}

static void ConformIdentity(JsonElement vector)
{
    byte[] seed = Convert.FromHexString(vector.GetProperty("seed").GetString()!);
    byte[] publicKey = Convert.FromHexString(vector.GetProperty("publicKey").GetString()!);
    byte[] signature = Convert.FromHexString(vector.GetProperty("signature").GetString()!);
    string payload = vector.GetProperty("payload").GetString()!;

    using var device = new DeviceIdentity(seed);
    Assert(device.PublicKey.SequenceEqual(publicKey), "public key matches");
    Assert(device.Fingerprint == vector.GetProperty("fingerprint").GetString(), "fingerprint matches");
    Assert(
        device.Sign(payload).SequenceEqual(signature),
        "the signature is deterministic for this seed and payload");

    Assert(DeviceIdentity.Verify(publicKey, payload, signature), "the signature verifies");
    Assert(
        !DeviceIdentity.Verify(publicKey, vector.GetProperty("tamperedPayload").GetString()!, signature),
        "a tampered payload does not verify");
}

static void ConformCodec(JsonElement vector)
{
    byte[] cbor = Convert.FromHexString(vector.GetProperty("cbor").GetString()!);
    byte[] json = Encoding.UTF8.GetBytes(vector.GetProperty("json").GetString()!);

    Assert(Codec.JsonToCbor(json).SequenceEqual(cbor), "JSON encodes to CBOR");
    Assert(Codec.CborToJson(cbor).SequenceEqual(json), "CBOR decodes to the document");
    Assert(
        Codec.JsonToCbor(Encoding.UTF8.GetBytes(vector.GetProperty("unsortedJson").GetString()!))
            .SequenceEqual(cbor),
        "keys are sorted on the way through, so the encoding is canonical");

    JsonElement deltas = vector.GetProperty("deltas");
    long[] samples = deltas.GetProperty("samples").EnumerateArray()
        .Select(entry => entry.GetInt64()).ToArray();
    byte[] packedSamples = Convert.FromHexString(deltas.GetProperty("packed").GetString()!);
    Assert(Codec.PackSamples(samples).SequenceEqual(packedSamples), "samples pack");
    Assert(Codec.UnpackSamples(packedSamples).SequenceEqual(samples), "samples unpack");

    JsonElement q = vector.GetProperty("quantizer");
    float[] readings = q.GetProperty("readings").EnumerateArray()
        .Select(entry => entry.GetSingle()).ToArray();
    byte[] packedReadings = Convert.FromHexString(q.GetProperty("packed").GetString()!);
    var quantizer = new Quantizer(q.GetProperty("scale").GetSingle());
    Assert(quantizer.Encode(readings).SequenceEqual(packedReadings), "readings pack");

    double readingTolerance = q.GetProperty("tolerance").GetDouble();
    float[] decoded = quantizer.Decode(packedReadings);
    for (int i = 0; i < readings.Length; i++)
    {
        Assert(
            Math.Abs(decoded[i] - readings[i]) <= readingTolerance,
            "reading decodes to precision");
    }
}

static void ConformHelpers(JsonElement vectors, double tolerance)
{
    JsonElement vector = vectors.GetProperty("smoother");
    using var smoother = new Smoother(vector.GetProperty("weight").GetSingle());
    Walk(vector, "samples", "outputs", (sample, want) =>
        Close(smoother.Update(sample), want, tolerance, "smoother output"));

    vector = vectors.GetProperty("pid");
    using var controller = new Pid(
        vector.GetProperty("kp").GetSingle(),
        vector.GetProperty("ki").GetSingle(),
        vector.GetProperty("kd").GetSingle());
    float setpoint = vector.GetProperty("setpoint").GetSingle();
    float dt = vector.GetProperty("dt").GetSingle();
    Walk(vector, "measurements", "outputs", (measurement, want) =>
        Close(controller.Update(setpoint, measurement, dt), want, tolerance, "pid output"));

    vector = vectors.GetProperty("thermostat");
    using var thermostat = Thermostat.Cooling(
        vector.GetProperty("setpoint").GetSingle(),
        vector.GetProperty("hysteresis").GetSingle());
    float[] readings = Floats(vector, "readings");
    bool[] states = vector.GetProperty("outputs").EnumerateArray()
        .Select(entry => entry.GetBoolean()).ToArray();
    for (int i = 0; i < readings.Length; i++)
    {
        Assert(thermostat.Update(readings[i]) == states[i], "thermostat output");
    }

    vector = vectors.GetProperty("depletion");
    using var depletion = new Depletion(vector.GetProperty("threshold").GetSingle());
    float[] levels = Floats(vector, "levels");
    JsonElement[] expected = vector.GetProperty("outputs").EnumerateArray().ToArray();
    for (int i = 0; i < levels.Length; i++)
    {
        uint? got = depletion.Update(levels[i]);
        uint? want = expected[i].ValueKind == JsonValueKind.Null
            ? null
            : expected[i].GetUInt32();
        Assert(got == want, "depletion output");
    }

    vector = vectors.GetProperty("calibration");
    using var calibration = Calibration.TwoPoint(
        vector.GetProperty("rawLow").GetSingle(),
        vector.GetProperty("valueLow").GetSingle(),
        vector.GetProperty("rawHigh").GetSingle(),
        vector.GetProperty("valueHigh").GetSingle());
    Walk(vector, "inputs", "outputs", (raw, want) =>
        Close(calibration.Apply(raw), want, tolerance, "calibration output"));

    vector = vectors.GetProperty("deadband");
    float center = vector.GetProperty("center").GetSingle();
    float width = vector.GetProperty("width").GetSingle();
    Walk(vector, "inputs", "outputs", (value, want) =>
        Close(Kit.Deadband(value, center, width), want, tolerance, "deadband output"));
}

static void ConformGeofence(JsonElement vector)
{
    JsonElement centre = vector.GetProperty("center");
    using var fence = new Geofence(
        new Coordinate(
            centre.GetProperty("latitude").GetDouble(),
            centre.GetProperty("longitude").GetDouble()),
        vector.GetProperty("radiusM").GetDouble());

    JsonElement[] fixes = vector.GetProperty("fixes").EnumerateArray().ToArray();
    string[] boundaries = vector.GetProperty("boundaries").EnumerateArray()
        .Select(entry => entry.GetString()!).ToArray();

    for (int i = 0; i < fixes.Length; i++)
    {
        Boundary got = fence.Update(new Coordinate(
            fixes[i].GetProperty("latitude").GetDouble(),
            fixes[i].GetProperty("longitude").GetDouble()));
        Assert(got.ToString() == boundaries[i], "boundary state");
    }
}

// Reads a float array from a vector.
static float[] Floats(JsonElement vector, string name) =>
    vector.GetProperty(name).EnumerateArray().Select(entry => entry.GetSingle()).ToArray();

// Walks an input and expected-output pair from a vector.
static void Walk(JsonElement vector, string inputs, string outputs, Action<float, float> check)
{
    float[] given = Floats(vector, inputs);
    float[] want = Floats(vector, outputs);
    for (int i = 0; i < given.Length; i++)
    {
        check(given[i], want[i]);
    }
}

// Asserts two numbers agree within the vectors' tolerance.
static void Close(float got, float want, double tolerance, string message) =>
    Assert(Math.Abs(got - want) <= tolerance, $"{message}: expected {want}, got {got}");
