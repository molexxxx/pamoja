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
FieldIo();

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
