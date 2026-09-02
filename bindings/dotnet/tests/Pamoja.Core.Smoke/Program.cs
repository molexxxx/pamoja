// Smoke test: confirms the facade loads, the native core is reachable, and each
// capability behaves through it (no broker or hardware required).
using System.Text;
using System.Text.Json;

using Pamoja.Core;
using Pamoja.Core.Interop;

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
FieldIo();
SensingAndActuation();
RadioAndReach();

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

// The wires a gateway actually has: framed serial packets, an RS485 request and
// the reply it draws, a CAN frame, and the address a chip answers on.
static void FieldIo()
{
    byte[] payload = [0xC0, 0xDB, 0x00, 0x2A];
    Assert(
        Serial.SlipDecode(Serial.SlipEncode(payload)).SequenceEqual(payload),
        "a SLIP frame round-trips");
    Assert(
        Serial.CobsDecode(Serial.CobsEncode(payload)).SequenceEqual(payload),
        "a COBS frame round-trips");

    using var decoder = new SlipDecoder();
    byte[][] frames = decoder.Feed([(byte)'o', (byte)'k', 0xC0, 0xDB, 0xC0, (byte)'g', (byte)'o', 0xC0]);
    Assert(frames.Length == 2, "the frames either side of a corrupt one survive");
    Assert(decoder.Discarded == 1, "the corrupt frame is counted");

    byte[] request = Modbus.ReadHoldingRegisters(0x11, 0x006B, 3);
    Assert(
        request.SequenceEqual(new byte[] { 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87 }),
        "the request carries the address, the PDU, and the CRC");

    byte[] replyBody = [0x11, 0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
    byte[] replyFrame = [.. replyBody, .. BitConverter.GetBytes(Modbus.Crc16(replyBody))];
    using ModbusFrame reply = Modbus.ParseFrame(replyFrame);
    Assert(reply.Exception is null, "a served request reports no exception");
    Assert(reply.Registers().SequenceEqual<ushort>([0x022B, 0x0000, 0x0064]), "registers read back");

    try
    {
        replyFrame[2] ^= 0xFF;
        Modbus.ParseFrame(replyFrame);
        Fail("a frame mangled on the wire should throw");
    }
    catch (PamojaException)
    {
    }

    CanFrame frame = Can.Frame(0x20A, [0x01, 0xF4]);
    Assert(frame.Dlc == 2 && frame.Data.Length == 2, "a classic frame carries its payload");
    CanFrame remote = Can.RemoteFrame(0x20A, 4);
    Assert(remote.Length == 4 && remote.Data.Length == 0, "a remote frame asks without carrying");

    J1939Message? engine = Can.DecodeJ1939(0x0CF00400);
    Assert(engine is not null && engine.Pgn == 61444, "the engine broadcast decodes");
    Assert(Can.DecodeJ1939(0x123, extended: false) is null, "J1939 needs an extended identifier");

    Assert(I2c.AddressFrame(0x76).SequenceEqual(new byte[] { 0xEC }), "a write frame shifts in r/w");
    Assert(I2c.IsReserved(0x00) && I2c.IsGeneralCall(0x00), "the general call is reserved");
    Assert(Spi.ClockFor(3) is { Cpol: true, Cpha: true }, "mode 3 idles high and samples late");
    Assert(
        Pin.LevelFor(PinPolarity.ActiveLow, asserted: true) == PinLevel.Low,
        "an active-low relay is energised by a low level");
}

// The parts wired to a board: a compensated environment reading, a thermometer
// that checks its own bytes, a servo pulse, and the stats over a rolling window.
static void SensingAndActuation()
{
    byte[] scratchpad = [0x91, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
    scratchpad[8] = Ds18b20.Crc8(scratchpad.AsSpan(0, 8));
    Ds18b20Reading reading = Ds18b20.ParseScratchpad(scratchpad);
    Assert(reading.MicroCelsius == 25_062_500, "the thermometer decodes its register");
    Assert(reading.ResolutionBits == 12, "and reports its resolution");

    try
    {
        scratchpad[0] ^= 0xFF;
        Ds18b20.ParseScratchpad(scratchpad);
        Fail("a scratchpad failing its CRC should throw");
    }
    catch (PamojaException)
    {
    }

    Assert(Ina219.Calibration(1_000, 2) == 0x5000, "the datasheet design example");
    Assert(Ina219.PowerMicrowatts(100, 1_000) == 2_000_000, "the power LSB is twenty times");

    Ads1115Config reset = Ads1115.ConfigFromBits(Ads1115.ConfigReset);
    Assert(Ads1115.ConfigBits(reset) == Ads1115.ConfigReset, "the config round-trips");
    Assert(Ads1115.FullScaleMicrovolts(1) == 4_096_000, "gain code 1 is plus or minus 4.096 V");

    Assert(Pwm.FullOff()[3] == 0x10, "fully off is its own encoding, not a zero duty");
    Assert(Pca9685.ChannelRegister(0) == 0x06, "the first channel's register block");

    using var motor = new Stepper(StepDrive.HalfStep);
    byte first = motor.Coils;
    for (int step = 0; step < Stepper.StepCount(StepDrive.HalfStep); step++)
    {
        motor.Step(StepDirection.Forward);
    }

    Assert(motor.Coils == first, "one electrical cycle returns to its first pattern");
    Assert(motor.Steps == 8, "and the position counts every step");

    using var window = new Window();
    foreach (float value in new[] { 10f, 20f, 30f })
    {
        window.Push(value);
    }

    Assert(window.Count == 3 && window.Capacity == 32, "the window fills to its capacity");
    Assert(Math.Abs((window.Mean() ?? 0f) - 20f) < 1e-5f, "and averages its readings");

    using var median = new Median();
    foreach (float value in new[] { 20f, 21f, 20.5f })
    {
        median.Update(value);
    }

    Assert(median.Update(900f) < 30f, "a median does not follow a single spike");

    using var trend = new Trend();
    foreach (float value in new[] { 1f, 2f, 3f, 4f })
    {
        trend.Push(value);
    }

    Assert(Math.Abs((trend.Slope ?? 0f) - 1f) < 1e-4f, "a rising signal has a positive slope");

    using var anomaly = new Anomaly(3f);
    for (int i = 0; i < 8; i++)
    {
        anomaly.Check(20f);
    }

    Assert(anomaly.Check(900f), "a reading far outside the window is flagged");
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
    ConformSerial(vectors.GetProperty("serial"));
    ConformModbus(vectors.GetProperty("modbus"));
    ConformCan(vectors.GetProperty("can"));
    ConformGpio(vectors.GetProperty("gpio"));
    ConformSensors(vectors.GetProperty("sensors"));
    ConformActuators(vectors.GetProperty("actuators"));
    ConformWindows(vectors.GetProperty("windows"), tolerance);
    ConformLora(vectors.GetProperty("lora"));
    ConformMesh(vectors.GetProperty("mesh"));
    ConformRouting(vectors.GetProperty("routing"));
    ConformLorawan(vectors.GetProperty("lorawan"));

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

static void ConformSerial(JsonElement vector)
{
    byte[] payload = Convert.FromHexString(vector.GetProperty("payload").GetString()!);
    byte[] slipFrame = Convert.FromHexString(vector.GetProperty("slipFrame").GetString()!);
    byte[] cobsFrame = Convert.FromHexString(vector.GetProperty("cobsFrame").GetString()!);

    Assert(Serial.SlipEncode(payload).SequenceEqual(slipFrame), "SLIP frame matches");
    Assert(Serial.SlipDecode(slipFrame).SequenceEqual(payload), "SLIP payload matches");
    Assert(Serial.CobsEncode(payload).SequenceEqual(cobsFrame), "COBS frame matches");
    Assert(Serial.CobsDecode(cobsFrame).SequenceEqual(payload), "COBS payload matches");

    Assert(
        Serial.SlipMaxEncodedLen(payload.Length) == vector.GetProperty("slipMaxEncodedLen").GetInt32(),
        "SLIP worst case matches");
    Assert(
        Serial.CobsMaxEncodedLen(payload.Length) == vector.GetProperty("cobsMaxEncodedLen").GetInt32(),
        "COBS worst case matches");

    try
    {
        Serial.SlipDecode(Convert.FromHexString(vector.GetProperty("corruptSlipFrame").GetString()!));
        Fail("a frame with a bad escape should throw");
    }
    catch (PamojaException)
    {
    }

    JsonElement stream = vector.GetProperty("slipStream");
    byte[] bytes = Convert.FromHexString(stream.GetProperty("bytes").GetString()!);
    int chunk = stream.GetProperty("chunk").GetInt32();
    using var decoder = new SlipDecoder();
    List<byte[]> frames = [];
    for (int at = 0; at < bytes.Length; at += chunk)
    {
        frames.AddRange(decoder.Feed(bytes.AsSpan(at, Math.Min(chunk, bytes.Length - at))));
    }

    string[] want = stream.GetProperty("frames").EnumerateArray()
        .Select(entry => entry.GetString()!).ToArray();
    Assert(frames.Count == want.Length, "the good frames survive the corrupt one");
    for (int index = 0; index < want.Length; index++)
    {
        Assert(Convert.ToHexString(frames[index]).ToLowerInvariant() == want[index], "frame matches");
    }

    Assert(
        decoder.Discarded == stream.GetProperty("discarded").GetUInt64(),
        "the discarded count matches");
}

static void ConformModbus(JsonElement vector)
{
    JsonElement read = vector.GetProperty("readHoldingRegisters");
    Assert(
        Modbus.ReadHoldingRegisters(
            read.GetProperty("address").GetByte(),
            read.GetProperty("start").GetUInt16(),
            read.GetProperty("count").GetUInt16())
            .SequenceEqual(Convert.FromHexString(read.GetProperty("frame").GetString()!)),
        "read-holding-registers frame matches");

    JsonElement single = vector.GetProperty("writeSingleRegister");
    Assert(
        Modbus.WriteSingleRegister(
            single.GetProperty("address").GetByte(),
            single.GetProperty("register").GetUInt16(),
            single.GetProperty("value").GetUInt16())
            .SequenceEqual(Convert.FromHexString(single.GetProperty("frame").GetString()!)),
        "write-single-register frame matches");

    JsonElement many = vector.GetProperty("writeMultipleRegisters");
    ushort[] values = many.GetProperty("values").EnumerateArray()
        .Select(entry => entry.GetUInt16()).ToArray();
    Assert(
        Modbus.WriteMultipleRegisters(
            many.GetProperty("address").GetByte(), many.GetProperty("start").GetUInt16(), values)
            .SequenceEqual(Convert.FromHexString(many.GetProperty("frame").GetString()!)),
        "write-multiple-registers frame matches");

    JsonElement bits = vector.GetProperty("writeMultipleCoils");
    bool[] states = bits.GetProperty("values").EnumerateArray()
        .Select(entry => entry.GetBoolean()).ToArray();
    Assert(
        Modbus.WriteMultipleCoils(
            bits.GetProperty("address").GetByte(), bits.GetProperty("start").GetUInt16(), states)
            .SequenceEqual(Convert.FromHexString(bits.GetProperty("frame").GetString()!)),
        "write-multiple-coils frame matches");

    JsonElement crc = vector.GetProperty("crc");
    Assert(
        Modbus.Crc16(Convert.FromHexString(crc.GetProperty("data").GetString()!))
            == crc.GetProperty("value").GetUInt16(),
        "the checksum matches");

    JsonElement replyVector = vector.GetProperty("reply");
    using ModbusFrame reply =
        Modbus.ParseFrame(Convert.FromHexString(replyVector.GetProperty("frame").GetString()!));
    Assert(reply.Address == replyVector.GetProperty("address").GetByte(), "reply address matches");
    Assert(
        reply.FunctionCode == replyVector.GetProperty("functionCode").GetByte(),
        "reply function matches");
    Assert(reply.Exception is null, "a served request reports no exception");
    Assert(
        reply.Registers().SequenceEqual(
            replyVector.GetProperty("registers").EnumerateArray()
                .Select(entry => entry.GetUInt16())),
        "reply registers match");

    // Registers above 0x7FFF, which catch a binding that reads them as signed.
    JsonElement highVector = vector.GetProperty("highRegisterReply");
    using ModbusFrame high =
        Modbus.ParseFrame(Convert.FromHexString(highVector.GetProperty("frame").GetString()!));
    Assert(
        high.Registers().SequenceEqual(
            highVector.GetProperty("registers").EnumerateArray()
                .Select(entry => entry.GetUInt16())),
        "registers above 0x7FFF read back unsigned");

    JsonElement bitVector = vector.GetProperty("bitReply");
    using ModbusFrame bitReply =
        Modbus.ParseFrame(Convert.FromHexString(bitVector.GetProperty("frame").GetString()!));
    Assert(
        bitReply.Coils(bitVector.GetProperty("count").GetUInt16()).SequenceEqual(
            bitVector.GetProperty("coils").EnumerateArray().Select(entry => entry.GetBoolean())),
        "reply coils match");

    JsonElement refusedVector = vector.GetProperty("exceptionReply");
    using ModbusFrame refused =
        Modbus.ParseFrame(Convert.FromHexString(refusedVector.GetProperty("frame").GetString()!));
    Assert(
        refused.Exception == refusedVector.GetProperty("exception").GetByte(),
        "the exception code matches");

    try
    {
        Modbus.ParseFrame(Convert.FromHexString(vector.GetProperty("corruptFrame").GetString()!));
        Fail("a frame mangled on the wire should throw");
    }
    catch (PamojaException)
    {
    }
}

static void ConformCan(JsonElement vector)
{
    JsonElement classicVector = vector.GetProperty("classic");
    CanFrame classic = Can.Frame(
        classicVector.GetProperty("id").GetUInt32(),
        Convert.FromHexString(classicVector.GetProperty("data").GetString()!),
        classicVector.GetProperty("extended").GetBoolean());
    Assert(classic.Dlc == classicVector.GetProperty("dlc").GetByte(), "classic DLC matches");

    JsonElement fdVector = vector.GetProperty("fd");
    CanFrame fd = Can.FdFrame(
        fdVector.GetProperty("id").GetUInt32(),
        Convert.FromHexString(fdVector.GetProperty("data").GetString()!),
        fdVector.GetProperty("extended").GetBoolean());
    Assert(fd.Dlc == fdVector.GetProperty("dlc").GetByte(), "CAN-FD DLC matches");
    Assert(fd.Fd && fd.Extended, "the frame keeps its flags");

    JsonElement remoteVector = vector.GetProperty("remote");
    CanFrame remote = Can.RemoteFrame(
        remoteVector.GetProperty("id").GetUInt32(),
        remoteVector.GetProperty("requested").GetInt32(),
        remoteVector.GetProperty("extended").GetBoolean());
    Assert(remote.Length == remoteVector.GetProperty("len").GetInt32(), "remote length matches");
    Assert(
        remote.Data.Length == remoteVector.GetProperty("dataLen").GetInt32(),
        "a remote frame carries no bytes");

    try
    {
        Can.Frame(0x100, new byte[vector.GetProperty("tooLongForClassic").GetInt32()]);
        Fail("a classic frame carries at most eight bytes");
    }
    catch (PamojaException)
    {
    }

    try
    {
        Can.FdFrame(0x100, new byte[vector.GetProperty("invalidFdLength").GetInt32()]);
        Fail("13 bytes is not a length CAN-FD can carry");
    }
    catch (PamojaException)
    {
    }

    foreach (JsonElement entry in vector.GetProperty("lengths").EnumerateArray())
    {
        Assert(
            Can.LenToDlc(entry.GetProperty("len").GetInt32()) == entry.GetProperty("dlc").GetByte(),
            "the length encodes to its code");
    }

    foreach (JsonElement entry in vector.GetProperty("codes").EnumerateArray())
    {
        Assert(
            Can.DlcToLen(entry.GetProperty("dlc").GetByte()) == entry.GetProperty("len").GetInt32(),
            "the code decodes to its length");
    }

    foreach (JsonElement entry in vector.GetProperty("j1939").EnumerateArray())
    {
        uint id = entry.GetProperty("id").GetUInt32();
        J1939Message? message = Can.DecodeJ1939(id);
        Assert(message is not null, "the identifier decodes");
        Assert(message!.Pgn == entry.GetProperty("pgn").GetUInt32(), "parameter group matches");
        Assert(message.Priority == entry.GetProperty("priority").GetByte(), "priority matches");
        Assert(message.Source == entry.GetProperty("source").GetByte(), "source matches");

        JsonElement destination = entry.GetProperty("destination");
        byte? want = destination.ValueKind == JsonValueKind.Null ? null : destination.GetByte();
        Assert(message.Destination == want, "destination matches");
        Assert(
            message.Broadcast == entry.GetProperty("broadcast").GetBoolean(),
            "broadcast flag matches");
        Assert(
            Can.ComposeJ1939(message.Priority, message.Pgn, message.Source, want ?? 0) == id,
            "the identifier round-trips");
    }

    Assert(
        Can.DecodeJ1939(vector.GetProperty("standardIsNotJ1939").GetUInt32(), extended: false)
            is null,
        "J1939 never rides an 11-bit identifier");
}

static void ConformGpio(JsonElement vector)
{
    foreach (JsonElement entry in vector.GetProperty("i2c").EnumerateArray())
    {
        ushort address = entry.GetProperty("address").GetUInt16();
        bool tenBit = entry.GetProperty("tenBit").GetBoolean();

        Assert(
            I2c.AddressFrame(address, read: false, tenBit: tenBit).SequenceEqual(
                Convert.FromHexString(entry.GetProperty("writeFrame").GetString()!)),
            "write frame matches");
        Assert(
            I2c.AddressFrame(address, read: true, tenBit: tenBit).SequenceEqual(
                Convert.FromHexString(entry.GetProperty("readFrame").GetString()!)),
            "read frame matches");
        Assert(
            I2c.FrameLen(address, tenBit) == entry.GetProperty("frameLen").GetInt32(),
            "frame length matches");
        Assert(
            I2c.IsReserved(address, tenBit) == entry.GetProperty("reserved").GetBoolean(),
            "reserved matches");
        Assert(
            I2c.IsGeneralCall(address, tenBit) == entry.GetProperty("generalCall").GetBoolean(),
            "general call matches");
    }

    try
    {
        I2c.AddressFrame(vector.GetProperty("outOfRangeSevenBit").GetUInt16());
        Fail("a 7-bit address above 0x7F should throw");
    }
    catch (PamojaException)
    {
    }

    try
    {
        I2c.AddressFrame(vector.GetProperty("outOfRangeTenBit").GetUInt16(), tenBit: true);
        Fail("a 10-bit address above 0x3FF should throw");
    }
    catch (PamojaException)
    {
    }

    foreach (JsonElement entry in vector.GetProperty("spi").EnumerateArray())
    {
        byte mode = entry.GetProperty("mode").GetByte();
        bool cpol = entry.GetProperty("cpol").GetBoolean();
        bool cpha = entry.GetProperty("cpha").GetBoolean();
        SpiClock clock = Spi.ClockFor(mode);
        Assert(clock.Cpol == cpol && clock.Cpha == cpha, "the mode names its clock");
        Assert(Spi.ModeFor(cpol, cpha) == mode, "the clock names its mode");
    }

    try
    {
        Spi.ClockFor(vector.GetProperty("invalidSpiMode").GetByte());
        Fail("there are only four SPI modes");
    }
    catch (PamojaException)
    {
    }

    foreach (JsonElement entry in vector.GetProperty("edges").EnumerateArray())
    {
        PinEdge edge = Enum.Parse<PinEdge>(entry.GetProperty("edge").GetString()!);
        PinLevel from = Enum.Parse<PinLevel>(entry.GetProperty("from").GetString()!);
        PinLevel to = Enum.Parse<PinLevel>(entry.GetProperty("to").GetString()!);
        Assert(
            Pin.Triggers(edge, from, to) == entry.GetProperty("triggered").GetBoolean(),
            "the trigger fires on its own transition");
    }

    foreach (JsonElement entry in vector.GetProperty("polarities").EnumerateArray())
    {
        PinPolarity polarity = Enum.Parse<PinPolarity>(entry.GetProperty("polarity").GetString()!);
        bool asserted = entry.GetProperty("asserted").GetBoolean();
        PinLevel level = Pin.LevelFor(polarity, asserted);
        Assert(
            level == Enum.Parse<PinLevel>(entry.GetProperty("level").GetString()!),
            "the polarity maps the state onto a level");
        Assert(
            Pin.IsAsserted(polarity, level) == entry.GetProperty("isAsserted").GetBoolean(),
            "and maps it back");
    }
}

static void ConformSensors(JsonElement vector)
{
    JsonElement bme = vector.GetProperty("bme280");
    using var calibration = new Bme280Calibration(
        Convert.FromHexString(bme.GetProperty("calibrationTempPress").GetString()!),
        Convert.FromHexString(bme.GetProperty("calibrationHumidity").GetString()!));
    Bme280Measurement reading = calibration.Compensate(
        Convert.FromHexString(bme.GetProperty("measurement").GetString()!));

    Assert(
        Math.Abs(reading.Celsius - bme.GetProperty("celsius").GetSingle()) < 1e-3f,
        "BME280 temperature matches");
    Assert(reading.Pascals == bme.GetProperty("pascals").GetUInt32(), "BME280 pressure matches");
    Assert(
        Math.Abs(reading.RelativeHumidityPercent
            - bme.GetProperty("relativeHumidityPercent").GetSingle()) < 1e-3f,
        "BME280 humidity matches");

    JsonElement ds = vector.GetProperty("ds18b20");
    Ds18b20Reading decoded = Ds18b20.ParseScratchpad(
        Convert.FromHexString(ds.GetProperty("scratchpad").GetString()!));
    Assert(
        decoded.RawTemperature == ds.GetProperty("rawTemperature").GetInt16(),
        "DS18B20 register matches");
    Assert(
        decoded.MicroCelsius == ds.GetProperty("microCelsius").GetInt32(),
        "DS18B20 temperature matches");
    Assert(
        decoded.ResolutionBits == ds.GetProperty("resolutionBits").GetByte(),
        "DS18B20 resolution matches");
    Assert(
        Ds18b20.Crc8(Convert.FromHexString(ds.GetProperty("crcData").GetString()!))
            == ds.GetProperty("crc").GetByte(),
        "DS18B20 checksum matches");

    try
    {
        Ds18b20.ParseScratchpad(
            Convert.FromHexString(ds.GetProperty("corruptScratchpad").GetString()!));
        Fail("a read corrupted on the bus should throw");
    }
    catch (PamojaException)
    {
    }

    try
    {
        Ds18b20.ConfigByte(ds.GetProperty("invalidResolution").GetByte());
        Fail("a resolution the part does not offer should throw");
    }
    catch (PamojaException)
    {
    }

    foreach (JsonElement entry in ds.GetProperty("resolutions").EnumerateArray())
    {
        byte bits = entry.GetProperty("bits").GetByte();
        byte configByte = entry.GetProperty("configByte").GetByte();
        Assert(Ds18b20.ConfigByte(bits) == configByte, "config byte matches");
        Assert(
            Ds18b20.StepMicroCelsius(bits) == entry.GetProperty("stepMicroCelsius").GetUInt32(),
            "resolution step matches");
        Assert(
            Ds18b20.MaxConversionMicros(bits)
                == entry.GetProperty("maxConversionMicros").GetUInt32(),
            "conversion time matches");
        Assert(Ds18b20.ResolutionBits(configByte) == bits, "the resolution round-trips");
    }

    JsonElement ina = vector.GetProperty("ina219");
    uint lsb = ina.GetProperty("currentLsbMicroamps").GetUInt32();
    Assert(
        Ina219.Calibration(lsb, ina.GetProperty("shuntMilliohms").GetUInt32())
            == ina.GetProperty("calibration").GetUInt16(),
        "INA219 calibration matches");
    Assert(
        Ina219.MinimumCurrentLsbMicroamps(ina.GetProperty("maxExpectedMicroamps").GetUInt32())
            == ina.GetProperty("minimumCurrentLsbMicroamps").GetUInt32(),
        "INA219 minimum resolution matches");
    Assert(
        Ina219.ShuntMicrovolts(ina.GetProperty("rawShunt").GetInt16())
            == ina.GetProperty("shuntMicrovolts").GetInt32(),
        "INA219 shunt voltage matches");
    Assert(
        Ina219.BusMillivolts(ina.GetProperty("rawBus").GetUInt16())
            == ina.GetProperty("busMillivolts").GetUInt32(),
        "INA219 bus voltage matches");
    Assert(
        Ina219.CurrentMicroamps(ina.GetProperty("rawCurrent").GetInt16(), lsb)
            == ina.GetProperty("currentMicroamps").GetInt32(),
        "INA219 current matches");
    Assert(
        Ina219.PowerMicrowatts(ina.GetProperty("rawPower").GetUInt16(), lsb)
            == ina.GetProperty("powerMicrowatts").GetUInt32(),
        "INA219 power matches");

    JsonElement ads = vector.GetProperty("ads1115");
    ushort configReset = ads.GetProperty("configReset").GetUInt16();
    Ads1115Config reset = Ads1115.ConfigFromBits(configReset);
    JsonElement want = ads.GetProperty("resetConfig");
    Assert(
        reset.StartConversion == want.GetProperty("startConversion").GetBoolean(),
        "ADS1115 start bit matches");
    Assert(reset.Mux == want.GetProperty("mux").GetByte(), "ADS1115 mux matches");
    Assert(reset.Pga == want.GetProperty("pga").GetByte(), "ADS1115 gain matches");
    Assert(
        reset.SingleShot == want.GetProperty("singleShot").GetBoolean(),
        "ADS1115 mode matches");
    Assert(
        reset.DataRate == want.GetProperty("dataRate").GetByte(),
        "ADS1115 data rate matches");
    Assert(
        reset.ComparatorQueue == want.GetProperty("comparatorQueue").GetByte(),
        "ADS1115 comparator queue matches");
    Assert(Ads1115.ConfigBits(reset) == configReset, "the configuration round-trips");

    foreach (JsonElement entry in ads.GetProperty("gains").EnumerateArray())
    {
        byte pga = entry.GetProperty("pga").GetByte();
        Assert(
            Ads1115.FullScaleMicrovolts(pga)
                == entry.GetProperty("fullScaleMicrovolts").GetUInt32(),
            "ADS1115 full scale matches");
        Assert(
            Ads1115.ToNanovolts(pga, 32_767)
                == entry.GetProperty("nanovoltsAtFullScale").GetInt64(),
            "ADS1115 conversion matches");
    }

    foreach (JsonElement entry in ads.GetProperty("rates").EnumerateArray())
    {
        Assert(
            Ads1115.SamplesPerSecond(entry.GetProperty("dataRate").GetByte())
                == entry.GetProperty("samplesPerSecond").GetUInt16(),
            "ADS1115 sample rate matches");
    }
}

static void ConformActuators(JsonElement vector)
{
    JsonElement pca = vector.GetProperty("pca9685");
    Assert(
        Pca9685.InternalOscHz == pca.GetProperty("internalOscHz").GetUInt32(),
        "the oscillator matches");
    Assert(Pca9685.Channels == pca.GetProperty("channels").GetByte(), "the channel count matches");
    Assert(Pca9685.Counts == pca.GetProperty("counts").GetUInt16(), "the counts match");

    foreach (JsonElement entry in pca.GetProperty("channelRegisters").EnumerateArray())
    {
        Assert(
            Pca9685.ChannelRegister(entry.GetProperty("channel").GetByte())
                == entry.GetProperty("register").GetByte(),
            "channel register matches");
    }

    try
    {
        Pca9685.ChannelRegister(pca.GetProperty("invalidChannel").GetByte());
        Fail("a channel beyond the part should throw");
    }
    catch (PamojaException)
    {
    }

    Assert(
        Pca9685.PrescaleForFrequency(
            pca.GetProperty("updateRateHz").GetUInt32(),
            pca.GetProperty("internalOscHz").GetUInt32())
            == pca.GetProperty("prescale").GetByte(),
        "the prescale matches");

    JsonElement pwm = vector.GetProperty("pwm");
    Assert(
        Pwm.Duty(pwm.GetProperty("duty").GetProperty("off").GetUInt16()).SequenceEqual(
            Convert.FromHexString(pwm.GetProperty("duty").GetProperty("bytes").GetString()!)),
        "duty bytes match");
    JsonElement servo = pwm.GetProperty("servoCentre");
    Assert(
        Pwm.Servo(
            servo.GetProperty("pulseMicros").GetUInt32(),
            servo.GetProperty("updateRateHz").GetUInt32())
            .SequenceEqual(Convert.FromHexString(servo.GetProperty("bytes").GetString()!)),
        "servo bytes match");
    Assert(
        Pwm.FullOn().SequenceEqual(
            Convert.FromHexString(pwm.GetProperty("fullOn").GetString()!)),
        "full-on bytes match");
    Assert(
        Pwm.FullOff().SequenceEqual(
            Convert.FromHexString(pwm.GetProperty("fullOff").GetString()!)),
        "full-off bytes match");

    JsonElement motor = vector.GetProperty("stepper");
    int stepCount = motor.GetProperty("stepCount").GetInt32();
    using var stepper = new Stepper(StepDrive.HalfStep);
    List<byte> cycle = [stepper.Coils];
    for (int step = 0; step < stepCount; step++)
    {
        cycle.Add(stepper.Step(StepDirection.Forward));
    }

    byte[] wantCycle = motor.GetProperty("forwardCycle").EnumerateArray()
        .Select(entry => entry.GetByte()).ToArray();
    Assert(cycle.SequenceEqual(wantCycle), "the forward cycle matches");
    Assert(stepper.Steps == stepCount, "the position counts every step");
    Assert(Stepper.StepCount(StepDrive.HalfStep) == stepCount, "one half-step cycle matches");
    Assert(
        Stepper.StepsForDegrees(
            motor.GetProperty("degrees").GetSingle(),
            motor.GetProperty("stepsPerRevolution").GetUInt32())
            == motor.GetProperty("stepsForDegrees").GetInt32(),
        "a quarter turn is a quarter of the revolution");
}

static void ConformWindows(JsonElement vector, double tolerance)
{
    Assert(
        NativeMethods.WindowCapacity == vector.GetProperty("capacity").GetInt32(),
        "the documented capacity matches");

    JsonElement windowVector = vector.GetProperty("window");
    float[] readings = windowVector.GetProperty("readings").EnumerateArray()
        .Select(entry => entry.GetSingle()).ToArray();
    JsonElement[] states = windowVector.GetProperty("states").EnumerateArray().ToArray();
    using var window = new Window();
    for (int index = 0; index < readings.Length; index++)
    {
        window.Push(readings[index]);
        JsonElement want = states[index];
        Assert(window.Count == want.GetProperty("len").GetInt32(), "window length matches");
        Assert(
            Math.Abs((window.Mean() ?? 0f) - want.GetProperty("mean").GetSingle()) <= tolerance,
            "window mean matches");
        Assert(
            Math.Abs((window.Min() ?? 0f) - want.GetProperty("min").GetSingle()) <= tolerance,
            "window minimum matches");
        Assert(
            Math.Abs((window.Max() ?? 0f) - want.GetProperty("max").GetSingle()) <= tolerance,
            "window maximum matches");
    }

    JsonElement medianVector = vector.GetProperty("median");
    float[] medianOutputs = medianVector.GetProperty("outputs").EnumerateArray()
        .Select(entry => entry.GetSingle()).ToArray();
    using var median = new Median();
    int position = 0;
    foreach (JsonElement entry in medianVector.GetProperty("readings").EnumerateArray())
    {
        Assert(
            Math.Abs(median.Update(entry.GetSingle()) - medianOutputs[position]) <= tolerance,
            "median matches");
        position++;
    }

    JsonElement trendVector = vector.GetProperty("trend");
    JsonElement[] slopes = trendVector.GetProperty("slopes").EnumerateArray().ToArray();
    using var trend = new Trend();
    position = 0;
    foreach (JsonElement entry in trendVector.GetProperty("readings").EnumerateArray())
    {
        trend.Push(entry.GetSingle());
        JsonElement want = slopes[position];
        if (want.ValueKind == JsonValueKind.Null)
        {
            Assert(trend.Slope is null, "no slope without enough readings");
        }
        else
        {
            Assert(
                Math.Abs((trend.Slope ?? 0f) - want.GetSingle()) <= 1e-4f,
                "trend slope matches");
        }

        position++;
    }

    JsonElement anomalyVector = vector.GetProperty("anomaly");
    bool[] flags = anomalyVector.GetProperty("flags").EnumerateArray()
        .Select(entry => entry.GetBoolean()).ToArray();
    using var anomaly = new Anomaly(anomalyVector.GetProperty("sigmas").GetSingle());
    position = 0;
    foreach (JsonElement entry in anomalyVector.GetProperty("readings").EnumerateArray())
    {
        Assert(
            anomaly.Check(entry.GetSingle()) == flags[position],
            "the detector flags the reading that stands out");
        position++;
    }
}

// Budgeting airtime, framing a mesh packet, routing it, and securing a LoRaWAN
// uplink: everything a node needs to reach a network it cannot see.
static void RadioAndReach()
{
    var link = new LoraLink(12, 125_000);
    Assert(link.SpreadingFactor == 12, "SF12 is the longest-range setting");
    Assert(link.AirtimeMicros(10) == 991_232, "the published LoRa airtime");
    Assert(
        link.MinOffTimeMicros(20, 10) == link.AirtimeMicros(20) * 99,
        "a 1% duty cycle costs ninety-nine times the airtime in silence");
    Assert(link.MinOffTimeMicros(20, 0) is null, "a zero duty cycle forbids transmitting");
    Assert(link.MessagesPerHour(20, 10) > 0, "and a 1% budget still allows some");

    MeshFrame reading = Mesh.BroadcastFrame(0x1234_5678, 1, "level=high"u8);
    MeshFrame received = Mesh.Parse(reading.Bytes);
    Assert(received.Broadcast, "a broadcast is addressed to every node");
    Assert(
        Encoding.UTF8.GetString(received.Payload) == "level=high",
        "and carries its reading");

    using var seen = new SeenPackets();
    Assert(seen.Record(received.Src, received.Id), "the first copy is new");
    Assert(!seen.Record(received.Src, received.Id), "a second copy is a duplicate");

    MeshFrame? forwarded = Mesh.Relayed(received.Bytes);
    Assert(forwarded is not null && forwarded.HopLimit == received.HopLimit - 1,
        "relaying spends one hop");

    byte[] corrupt = (byte[])received.Bytes.Clone();
    corrupt[^3] ^= 0xFF;
    try
    {
        Mesh.Parse(corrupt);
        Fail("a mangled frame should be refused");
    }
    catch (PamojaException)
    {
    }

    using var router = new Router(0x01);
    router.Observe(0x09, 0x05, 2);
    Assert(router.Forward(0x09).NextHop == 0x05, "a learned route relays");
    router.Observe(0x09, 0x07, 1);
    Assert(router.Forward(0x09).NextHop == 0x07, "a cheaper neighbour wins");
    Assert(
        router.Forward(0x01).Action == ForwardAction.Deliver,
        "a packet for this node is delivered");
    Assert(
        router.Forward(0x20).Action == ForwardAction.Flood,
        "and an unknown destination falls back to flooding");

    byte[] nwkSKey = new byte[16];
    byte[] appSKey = new byte[16];
    Array.Fill(nwkSKey, (byte)0x2B);
    Array.Fill(appSKey, (byte)0x99);
    using var session = new LorawanSession(0x2601_1BDA, nwkSKey, appSKey);
    byte[] uplink = session.EncodeUplink(42, 1, "temp=4.8"u8, new LorawanOptions { Confirmed = true });
    LorawanRxData rx = session.Decode(uplink, 42);
    Assert(rx.Direction == LorawanDirection.Uplink, "the frame went up");
    Assert(rx.Confirmed, "and asked to be acknowledged");
    Assert(Encoding.UTF8.GetString(rx.Payload) == "temp=4.8", "the payload decrypts");

    byte[] forged = (byte[])uplink.Clone();
    forged[^1] ^= 0xFF;
    try
    {
        session.Decode(forged, 42);
        Fail("a forged frame should be refused");
    }
    catch (PamojaException)
    {
    }

    byte[] appKey = new byte[16];
    Array.Fill(appKey, (byte)0x2B);
    using var node = new LorawanDevice(
        [1, 2, 3, 4, 5, 6, 7, 8],
        [0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18],
        appKey);
    Assert(node.JoinRequest(0x0102).Length == 23, "a join request is 23 bytes");
    try
    {
        byte[] never = new byte[17];
        Array.Fill(never, (byte)0x20);
        node.AcceptJoin(never, 0x0102);
        Fail("a join accept the network never signed should not activate a session");
    }
    catch (PamojaException)
    {
    }
}

static LoraLink LinkOf(JsonElement described)
{
    var link = new LoraLink(
        described.GetProperty("spreadingFactor").GetByte(),
        described.GetProperty("bandwidthHz").GetUInt32())
        .WithCodingRate(described.GetProperty("codingRateDenominator").GetByte())
        .WithPreamble(described.GetProperty("preambleSymbols").GetUInt16());
    if (!described.GetProperty("explicitHeader").GetBoolean())
    {
        link = link.WithImplicitHeader();
    }

    if (!described.GetProperty("crc").GetBoolean())
    {
        link = link.WithoutCrc();
    }

    return link;
}

static void ConformLora(JsonElement vector)
{
    foreach (JsonElement described in vector.GetProperty("links").EnumerateArray())
    {
        LoraLink link = LinkOf(described);
        Assert(
            link.SymbolTimeMicros == described.GetProperty("symbolTimeUs").GetUInt64(),
            "symbol time");

        foreach (JsonElement airtime in described.GetProperty("airtimes").EnumerateArray())
        {
            Assert(
                link.AirtimeMicros(airtime.GetProperty("payloadLen").GetInt32())
                    == airtime.GetProperty("airtimeUs").GetUInt64(),
                "time on air");
        }

        foreach (JsonElement budget in described.GetProperty("budgets").EnumerateArray())
        {
            Assert(
                link.MinOffTimeMicros(
                    budget.GetProperty("payloadLen").GetInt32(),
                    budget.GetProperty("permille").GetUInt32())
                    == budget.GetProperty("offTimeUs").GetUInt64(),
                "the silence a duty cycle forces");
        }
    }

    foreach (JsonElement clamp in vector.GetProperty("clamped").EnumerateArray())
    {
        Assert(
            new LoraLink(clamp.GetProperty("asked").GetByte(), 125_000).SpreadingFactor
                == clamp.GetProperty("used").GetByte(),
            "a spreading factor outside 7 to 12 is clamped");
    }

    // Rust saturates the off time when transmitting is forbidden; the facade
    // reports null instead, so a caller cannot mistake it for a real wait.
    JsonElement forbidden = vector.GetProperty("forbidden");
    string name = forbidden.GetProperty("link").GetString()!;
    JsonElement described2 = vector.GetProperty("links").EnumerateArray()
        .First(entry => entry.GetProperty("name").GetString() == name);
    LoraLink forbiddenLink = LinkOf(described2);
    Assert(
        forbiddenLink.MinOffTimeMicros(
            forbidden.GetProperty("payloadLen").GetInt32(),
            forbidden.GetProperty("permille").GetUInt32()) is null,
        "a zero duty cycle forbids transmitting");
    Assert(
        forbiddenLink.MessagesPerHour(
            forbidden.GetProperty("payloadLen").GetInt32(),
            forbidden.GetProperty("permille").GetUInt32()) == 0,
        "and so allows no messages at all");
}

static void ConformMesh(JsonElement vector)
{
    Assert(Mesh.MaxFrame == vector.GetProperty("maxFrame").GetInt32(), "the frame ceiling");
    Assert(Mesh.MaxPayload == vector.GetProperty("maxPayload").GetInt32(), "the payload ceiling");
    Assert(
        Mesh.Broadcast == vector.GetProperty("broadcastAddress").GetUInt32(),
        "the broadcast address");

    JsonElement unicast = vector.GetProperty("unicast");
    MeshFrame built = Mesh.Frame(
        unicast.GetProperty("src").GetUInt32(),
        unicast.GetProperty("dst").GetUInt32(),
        unicast.GetProperty("id").GetUInt16(),
        Convert.FromHexString(unicast.GetProperty("payload").GetString()!),
        unicast.GetProperty("hopLimit").GetByte());
    Assert(
        Convert.ToHexString(built.Bytes).ToLowerInvariant() == unicast.GetProperty("bytes").GetString(),
        "an addressed frame matches byte for byte");

    JsonElement broadcast = vector.GetProperty("broadcast");
    built = Mesh.BroadcastFrame(
        broadcast.GetProperty("src").GetUInt32(),
        broadcast.GetProperty("id").GetUInt16(),
        Convert.FromHexString(broadcast.GetProperty("payload").GetString()!));
    Assert(
        Convert.ToHexString(built.Bytes).ToLowerInvariant() == broadcast.GetProperty("bytes").GetString(),
        "a broadcast frame matches byte for byte");

    byte[] onAir = Convert.FromHexString(broadcast.GetProperty("bytes").GetString()!);
    MeshFrame parsed = Mesh.Parse(onAir);
    Assert(parsed.Broadcast, "and parses back as a broadcast");

    MeshFrame? relayed = Mesh.Relayed(onAir);
    Assert(relayed is not null, "a fresh frame has hops to spend");
    Assert(
        Convert.ToHexString(relayed!.Bytes).ToLowerInvariant()
            == vector.GetProperty("relayed").GetProperty("bytes").GetString(),
        "relaying spends a hop");

    Assert(
        Mesh.Relayed(Convert.FromHexString(vector.GetProperty("exhausted").GetString()!)) is null,
        "a frame with no hops left must not be relayed");

    try
    {
        Mesh.Parse(Convert.FromHexString(vector.GetProperty("corrupt").GetString()!));
        Fail("a frame the air mangled must be refused");
    }
    catch (PamojaException)
    {
    }

    JsonElement crc = vector.GetProperty("crc");
    Assert(
        Mesh.Crc16(Convert.FromHexString(crc.GetProperty("check").GetString()!))
            == crc.GetProperty("checkValue").GetUInt16(),
        "the published CRC-16/CCITT-FALSE check value");
    Assert(
        Mesh.Crc16(Convert.FromHexString(crc.GetProperty("data").GetString()!))
            == crc.GetProperty("value").GetUInt16(),
        "the frame checksum");

    using var seen = new SeenPackets();
    Assert(
        seen.Capacity == vector.GetProperty("seenCapacity").GetInt32(),
        "the cache size");
    JsonElement keys = vector.GetProperty("seen").GetProperty("keys");
    bool[] answers = vector.GetProperty("seen").GetProperty("new").EnumerateArray()
        .Select(entry => entry.GetBoolean()).ToArray();
    int position = 0;
    foreach (JsonElement key in keys.EnumerateArray())
    {
        Assert(
            seen.Record(key[0].GetUInt32(), key[1].GetUInt16()) == answers[position],
            "each packet is new exactly once");
        position++;
    }
}

static void AssertDecision(Router router, JsonElement want)
{
    ForwardDecision decision = router.Forward(want.GetProperty("dst").GetUInt32());
    Assert(
        decision.Action.ToString() == want.GetProperty("action").GetString(),
        "the routing action");
    JsonElement nextHop = want.GetProperty("nextHop");
    if (nextHop.ValueKind == JsonValueKind.Null)
    {
        Assert(decision.NextHop is null, "no next hop belongs to this decision");
    }
    else
    {
        Assert(decision.NextHop == nextHop.GetUInt32(), "the neighbour to unicast to");
    }
}

static void ConformRouting(JsonElement vector)
{
    using var router = new Router(vector.GetProperty("address").GetUInt32());
    Assert(router.Capacity == vector.GetProperty("capacity").GetInt32(), "the table size");

    foreach (JsonElement observation in vector.GetProperty("observations").EnumerateArray())
    {
        Assert(
            router.Observe(
                observation.GetProperty("origin").GetUInt32(),
                observation.GetProperty("via").GetUInt32(),
                observation.GetProperty("cost").GetUInt16())
                == observation.GetProperty("changed").GetBoolean(),
            "learning changes the table");
    }

    Assert(router.Count == vector.GetProperty("learned").GetInt32(), "the routes it kept");

    JsonElement route = vector.GetProperty("route");
    Route? learned = router.RouteTo(route.GetProperty("dst").GetUInt32());
    Assert(learned is not null, "the route was learned");
    Assert(learned!.Value.NextHop == route.GetProperty("nextHop").GetUInt32(), "the cheapest way");
    Assert(learned.Value.Cost == route.GetProperty("cost").GetUInt16(), "and what it costs");

    foreach (JsonElement want in vector.GetProperty("decisions").EnumerateArray())
    {
        AssertDecision(router, want);
    }

    JsonElement forgotten = vector.GetProperty("afterForgetting");
    router.Forget(forgotten.GetProperty("dst").GetUInt32());
    AssertDecision(router, forgotten.GetProperty("decision"));
    Assert(
        router.Count == forgotten.GetProperty("learned").GetInt32(),
        "forgetting drops exactly one route");
}

static void ConformLorawan(JsonElement vector)
{
    using var session = new LorawanSession(
        vector.GetProperty("devAddr").GetUInt32(),
        Convert.FromHexString(vector.GetProperty("nwkSKey").GetString()!),
        Convert.FromHexString(vector.GetProperty("appSKey").GetString()!));
    Assert(
        session.DevAddr == vector.GetProperty("devAddr").GetUInt32(),
        "the session is bound to its address");

    JsonElement up = vector.GetProperty("uplink");
    byte[] uplink = session.EncodeUplink(
        up.GetProperty("fcnt").GetUInt32(),
        up.GetProperty("fport").GetByte(),
        Convert.FromHexString(up.GetProperty("payload").GetString()!),
        new LorawanOptions
        {
            Confirmed = up.GetProperty("confirmed").GetBoolean(),
            Adr = up.GetProperty("adr").GetBoolean(),
            Ack = up.GetProperty("ack").GetBoolean(),
        });
    Assert(
        Convert.ToHexString(uplink).ToLowerInvariant() == up.GetProperty("frame").GetString(),
        "a secured uplink matches byte for byte");

    LorawanRxData rx = session.Decode(uplink, up.GetProperty("fcnt").GetUInt32());
    Assert(rx.Direction == LorawanDirection.Uplink, "the frame went up");
    Assert(rx.Confirmed == up.GetProperty("confirmed").GetBoolean(), "the confirmed bit");
    Assert(rx.Adr == up.GetProperty("adr").GetBoolean(), "the ADR bit");
    Assert(
        Convert.ToHexString(rx.Payload).ToLowerInvariant() == up.GetProperty("payload").GetString(),
        "the payload decrypts");

    JsonElement down = vector.GetProperty("downlink");
    byte[] downlink = session.EncodeDownlink(
        down.GetProperty("fcnt").GetUInt32(),
        down.GetProperty("fport").GetByte(),
        Convert.FromHexString(down.GetProperty("payload").GetString()!),
        new LorawanOptions
        {
            Ack = down.GetProperty("ack").GetBoolean(),
            FPending = down.GetProperty("fpending").GetBoolean(),
            Fopts = Convert.FromHexString(down.GetProperty("fopts").GetString()!),
        });
    Assert(
        Convert.ToHexString(downlink).ToLowerInvariant() == down.GetProperty("frame").GetString(),
        "a secured downlink matches byte for byte");

    LorawanRxData received = session.Decode(downlink, down.GetProperty("fcnt").GetUInt32());
    Assert(received.Direction == LorawanDirection.Downlink, "the frame came down");
    Assert(received.FPending == down.GetProperty("fpending").GetBoolean(), "the pending bit");
    Assert(
        Convert.ToHexString(received.Fopts).ToLowerInvariant() == down.GetProperty("fopts").GetString(),
        "the MAC commands survive");

    try
    {
        session.Decode(
            Convert.FromHexString(vector.GetProperty("forgedUplink").GetString()!),
            up.GetProperty("fcnt").GetUInt32());
        Fail("a frame altered after signing must not verify");
    }
    catch (PamojaException)
    {
    }

    try
    {
        session.Decode(uplink, vector.GetProperty("wrongCounter").GetUInt32());
        Fail("a frame out of its place in the counter stream must not verify");
    }
    catch (PamojaException)
    {
    }

    JsonElement join = vector.GetProperty("join");
    using var device = new LorawanDevice(
        Convert.FromHexString(join.GetProperty("devEui").GetString()!),
        Convert.FromHexString(join.GetProperty("appEui").GetString()!),
        Convert.FromHexString(join.GetProperty("appKey").GetString()!));
    Assert(
        Convert.ToHexString(device.JoinRequest(join.GetProperty("devNonce").GetUInt16()))
            .ToLowerInvariant() == join.GetProperty("request").GetString(),
        "the join request matches byte for byte");

    try
    {
        device.AcceptJoin(
            Convert.FromHexString(join.GetProperty("forgedAccept").GetString()!),
            join.GetProperty("devNonce").GetUInt16());
        Fail("a join the network never signed must not activate a session");
    }
    catch (PamojaException)
    {
    }
}
