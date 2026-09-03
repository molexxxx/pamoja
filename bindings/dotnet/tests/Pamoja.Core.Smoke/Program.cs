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
TrustAndOperation();
await AsyncTransports();
ProfilesAndRobotics();

Console.WriteLine("ok");

Conformance();


// Proving what a node did, saying it in confidence, fixing it in the field, and
// deciding how often it can afford to do any of that.
static void TrustAndOperation()
{
    // A signed, chained log: what a node did, in an order nobody can quietly edit.
    using var keeper = new DeviceIdentity(Repeat(0x21, 32));
    using var log = new AuditLog(keeper);
    using AuditEntry opened = log.Append("valve=open"u8);
    using AuditEntry shut = log.Append("valve=shut"u8);

    Assert(opened.Index == 0, "the first record sits at index zero");
    Assert(
        shut.Previous.AsSpan().SequenceEqual(opened.Digest),
        "each record carries the hash of the one before it");
    Assert(
        Audit.VerifyChain(keeper.PublicKey, [opened, shut]),
        "an untouched chain verifies");

    byte[] edited = shut.ToBytes();
    edited[^1] ^= 0xFF;
    using AuditEntry tampered = AuditEntry.FromBytes(edited);
    Assert(
        !Audit.VerifyChain(keeper.PublicKey, [opened, tampered]),
        "and an altered record breaks it");

    using AuditLog resumed = AuditLog.Resume(keeper, shut);
    using AuditEntry afterReboot = resumed.Append("valve=open"u8);
    Assert(afterReboot.Index == 2, "a reboot leaves no gap");

    // Two devices that know each other's public keys, talking in confidence.
    using var node = new AgreementKey(Repeat(0x01, 32));
    using var gateway = new AgreementKey(Repeat(0x02, 32));
    byte[] salt = Repeat(0x09, 16);
    using var uplink = new Session(node, gateway.PublicKey, salt, SessionRole.Initiator);
    using var downlink = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);

    SealedMessage message = uplink.Seal("4.8C"u8, "pump-3"u8);
    Assert(
        !message.Ciphertext.AsSpan().SequenceEqual("4.8C"u8),
        "the reading does not travel in the clear");
    Assert(
        downlink.Open(message, "pump-3"u8).AsSpan().SequenceEqual("4.8C"u8),
        "the peer recovers it");

    try
    {
        downlink.Open(message, "pump-3"u8);
        Fail("a repeated counter must be refused");
    }
    catch (PamojaException)
    {
    }

    SealedMessage second = uplink.Seal("4.9C"u8, "pump-3"u8);
    byte[] broken = (byte[])second.Ciphertext.Clone();
    broken[0] ^= 0xFF;
    try
    {
        downlink.Open(new SealedMessage(second.Counter, second.Tag, broken), "pump-3"u8);
        Fail("an altered message must be refused");
    }
    catch (PamojaException)
    {
    }

    // Fixing a device in the field: a signed release, staged in pieces, tried,
    // and confirmed only once it has run.
    byte[] vendor = Repeat(0x0A, 16);
    byte[] deviceClass = Repeat(0x0B, 16);
    using var publisher = new DeviceIdentity(Repeat(0x31, 32));
    byte[] image = Repeat(0xA5, 600);
    var manifest = new Manifest(
        Sequence: 2,
        VendorId: vendor,
        ClassId: deviceClass,
        Storage: 1,
        Digest: System.Security.Cryptography.SHA256.HashData(image),
        Size: (uint)image.Length);
    byte[] envelope = Update.SignManifest(manifest, publisher);
    Assert(
        Update.VerifyEnvelope(envelope, publisher.PublicKey).Digest
            .AsSpan().SequenceEqual(manifest.Digest),
        "the release verifies against the key that signed it");

    using var fleet = new Updater(vendor, deviceClass, publisher.PublicKey, 2, 4096);
    fleet.Provision(0, 1);
    Assert(fleet.Begin(envelope) == 1, "the release names the spare slot");
    for (int at = 0; at < image.Length; at += 128)
    {
        fleet.Write(image.AsSpan(at, Math.Min(128, image.Length - at)));
    }

    Assert(fleet.CurrentProgress().Written == image.Length, "every byte arrived");
    Assert(fleet.Finish() == 1, "and the image matched what was promised");

    Assert(fleet.OnBoot().Action == BootAction.Trying, "a new image is on trial");
    Assert(fleet.Confirm() == 1, "and confirms once it has run");
    Assert(
        fleet.Record(1).State == SlotState.Confirmed,
        "so the slot holds the release from now on");

    using var impostor = new DeviceIdentity(Repeat(0x32, 32));
    try
    {
        fleet.Stage(Update.SignManifest(manifest with { Sequence = 3 }, impostor), image);
        Fail("a release signed by anyone else must be refused");
    }
    catch (PamojaException)
    {
    }

    // A delegated key signs day to day, so the anchor can stay offline.
    using var anchor = new DeviceIdentity(Repeat(0x41, 32));
    using var releases = new DeviceIdentity(Repeat(0x42, 32));
    byte[] statement = Update.SignDelegation(
        new Delegation(Epoch: 1, ReleaseKey: releases.PublicKey), anchor);
    Assert(
        Update.OpenDelegation(statement, anchor.PublicKey).ReleaseKey
            .AsSpan().SequenceEqual(releases.PublicKey),
        "the delegation names the release key");

    using var delegated = new Updater(vendor, deviceClass, anchor.PublicKey, 2, 4096);
    delegated.Provision(0, 1);
    delegated.Adopt(statement);
    Assert(delegated.CurrentDelegation is not null, "the device now honours it");
    Assert(
        delegated.Stage(Update.SignManifest(manifest, releases), image) == 1,
        "so a release the anchor never touched is accepted");

    // How often a node on a battery can afford to do any of the above.
    PowerPlan plan = PowerPlan.Create(60_000_000, 300_000_000, 3_600_000_000);
    Assert(plan.Mode(0.9f) == PowerMode.Active, "a healthy charge works normally");
    Assert(plan.Mode(0.1f) == PowerMode.Critical, "a flat one barely works at all");
    Assert(
        plan.ModeWhileCharging(0.1f, true) == PowerMode.Saver,
        "and sunlight eases it back one step");
    Assert(plan.IntervalUs(0.1f) == 3_600_000_000, "which is an hour between readings");

    DutyCycle duty = DutyCycle.FromFraction(1_000_000, 0.25f);
    Assert(duty.ActiveUs == 250_000, "a quarter of the period is spent awake");

    // What it says about itself on the way back, and what it drops when the link
    // costs too much to say it.
    using var reporter = new Reporter(TelemetryLevel.Trace);
    reporter.AdaptTo(LinkCost.Expensive);
    Assert(
        reporter.Record(new TelemetryEvent(TelemetryLevel.Info, "loop.tick")) is null,
        "routine detail is dropped on a costly link");

    TelemetryEvent? warned =
        reporter.Record(new TelemetryEvent(TelemetryLevel.Warn, "battery.low", 0.18f));
    Assert(warned?.Code == "battery.low", "but a warning still ships");
    Assert(warned?.Value == 0.18f, "with the measurement that triggered it");

    TelemetrySnapshot counts = reporter.Snapshot();
    Assert(counts.Dropped == 1, "the dropped event was still counted");
    Assert(counts.Emitted == 1, "alongside the one that shipped");
    Assert(
        Reporter.ThresholdFor(LinkCost.Offline) == TelemetryLevel.Error,
        "and an offline link ships only failures");
}

// Builds a buffer of one repeated byte, which is how the fixtures name keys.
static byte[] Repeat(byte value, int length)
{
    byte[] bytes = new byte[length];
    Array.Fill(bytes, value);
    return bytes;
}


// Reaching the network when no single link always works, and testing all of it
// with nothing plugged in.
static async Task AsyncTransports()
{
    // An in-process broker: publish on one link, receive on another.
    using var broker = new LoopbackBroker();
    using LoopbackTransport publisher = broker.Link();
    using LoopbackTransport subscriber = broker.Link();
    await publisher.ConnectAsync();
    await subscriber.ConnectAsync();
    Assert(await subscriber.IsConnectedAsync(), "a connected link reports it");

    await subscriber.SubscribeAsync("sensors/1");
    await publisher.SendAsync("sensors/1", "21.5"u8.ToArray());

    TransportMessage? received = await subscriber.ReceiveAsync();
    Assert(received?.Topic == "sensors/1", "the topic survives");
    Assert(
        received!.Payload.AsSpan().SequenceEqual("21.5"u8),
        "and so does the reading");

    // A buffer holds what cannot be sent yet.
    using var store = Store.Memory();
    await store.AppendAsync("one"u8.ToArray());
    await store.AppendAsync("two"u8.ToArray());
    Assert(await store.CountAsync() == 2, "both records are held");
    Assert(
        (await store.PeekAsync())!.AsSpan().SequenceEqual("one"u8),
        "peek leaves the record in place");
    Assert((await store.PopAsync())!.AsSpan().SequenceEqual("one"u8), "oldest first");
    Assert((await store.PopAsync())!.AsSpan().SequenceEqual("two"u8), "then the next");
    Assert(await store.PopAsync() is null, "an empty store yields nothing");

    using var bounded = Store.Memory(1);
    await bounded.AppendAsync("one"u8.ToArray());
    try
    {
        await bounded.AppendAsync("two"u8.ToArray());
        Fail("a full store must tell the caller rather than dropping something");
    }
    catch (PamojaException)
    {
    }

    // With no rung, a ladder buffers rather than losing the reading.
    using var offline = new Ladder(Store.Memory());
    Assert(
        await offline.SendAsync("sensors/1", "21.5"u8.ToArray()) == Delivery.Buffered,
        "buffering is a success, not a failure");
    Assert(await offline.BufferedAsync() == 1, "and the reading is waiting");

    // The link comes back, and the buffer drains over it.
    offline.Rung(broker.Rung());
    await offline.ConnectAsync();
    Assert(await offline.FlushAsync() == 1, "the buffered reading went out");
    Assert(await offline.BufferedAsync() == 0, "leaving the buffer empty");

    // A rung that refuses falls through to the next.
    using var rungs = new Ladder(Store.Memory());
    rungs.Rung(Transport.Faulty(broker.Rung(), 1));
    rungs.Rung(broker.Rung());
    await rungs.ConnectAsync();
    Assert(
        await rungs.SendAsync("sensors/1", "4.8C"u8.ToArray()) == Delivery.Sent,
        "the second rung carried what the first refused");

    // A transport handed to a ladder is spent.
    Transport spent = broker.Rung();
    Assert(spent.IsAvailable, "a fresh transport is holdable");
    rungs.Rung(spent);
    Assert(!spent.IsAvailable, "and is not once it has been added");
    try
    {
        rungs.Rung(spent);
        Fail("adding a spent transport must be refused");
    }
    catch (PamojaException)
    {
    }

    // One publisher, many subscribers, in one process.
    using var hub = new EventBus(8);
    using EventBus firstSeat = hub.Subscribe();
    using EventBus secondSeat = hub.Subscribe();
    await hub.PublishAsync("battery.low"u8.ToArray());
    Assert(
        (await firstSeat.NextAsync())!.AsSpan().SequenceEqual("battery.low"u8),
        "the first subscriber saw it");
    Assert(
        (await secondSeat.NextAsync())!.AsSpan().SequenceEqual("battery.low"u8),
        "and so did the second");

    // Devices that need no hardware.
    using var seeded = new SimulatedSensor(20.0f, 0.5f, 1.0f, 42);
    using var twin = new SimulatedSensor(20.0f, 0.5f, 1.0f, 42);
    for (int at = 0; at < 5; at++)
    {
        Assert(
            await seeded.ReadAsync() == await twin.ReadAsync(),
            "the same seed gives the same readings");
    }

    using var replay = new Replay([21.0f, 21.5f, 22.0f], repeating: true);
    for (int round = 0; round < 2; round++)
    {
        foreach (float want in new[] { 21.0f, 21.5f, 22.0f })
        {
            Assert(Math.Abs(await replay.ReadAsync() - want) < 1e-6f, "a capture reads back");
        }
    }

    using var actuator = new RecordingActuator();
    foreach (float command in new[] { 0.0f, 0.5f, 1.0f })
    {
        await actuator.ApplyAsync(command);
    }

    Assert(actuator.Count == 3, "every command was recorded");
    Assert(actuator.Commands.AsSpan().SequenceEqual([0.0f, 0.5f, 1.0f]), "in order");

    using var robot = new SimulatedRobot(1.0f);
    await robot.ApplyAsync(new Twist(1.0f));
    Assert(
        Math.Abs(robot.Pose.X - 1.0f) < 1e-5f,
        "one second at one metre a second puts it a metre ahead");
}

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


// A profile deciding what a reading calls for, and the naming and encoding rules
// a robot's graph is addressed by.
static void ProfilesAndRobotics()
{
    const string ChatterHash =
        "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18";

    using var fridge = Profile.VaccineFridgeMonitor();
    Assert(fridge.Name == "vaccine-fridge-monitor", "a preset carries its name");
    Assert(fridge.Control.Kind == ControlKind.Setpoint, "and its control policy");
    Assert(fridge.Control.Cooling == true, "a fridge cools rather than heats");

    using (Controller control = fridge.Controller())
    {
        Reaction warm = control.Evaluate(9.0f);
        Assert(warm.Actuator == true, "a warm fridge runs the cooler");
        Assert(warm.Alert?.Kind == AlertKind.OutOfRange, "and 9 C is a spoilage excursion");
        Assert(Math.Abs((warm.Alert?.Reading ?? 0f) - 9.0f) < 1e-6f, "the alert carries the reading");
    }

    using (Controller observer = Controller.Monitor())
    {
        Reaction seen = observer.Evaluate(21.5f);
        Assert(seen.Actuator is null, "a monitor drives no output");
        Assert(seen.Alert is null, "and judges nothing");
    }

    using var reloaded = Profile.FromJson(fridge.ToJson());
    Assert(reloaded.Topic == fridge.Topic, "a manifest round-trips");
    Assert(
        reloaded.PowerPlan.ActiveUs == fridge.Power.ActiveSecs * 1_000_000,
        "and its schedule assembles into a governor");

    Assert(Ros2.IsValidName("/robot1/camera_left/image_raw"), "a legal ROS 2 name is accepted");
    Assert(!Ros2.IsValidName("/2foo"), "a token may not start with a digit");
    Assert(Ros2.IsFullyQualified("/chatter"), "a leading slash is fully qualified");
    Assert(
        Ros2.DdsTopic("/robot1/cmd_vel", EntityKind.Topic) == "rt/robot1/cmd_vel",
        "a topic takes the rt prefix");
    Assert(Ros2.PrefixFor(EntityKind.ServiceRequest) == "rq", "a request takes rq");
    Assert(
        Ros2.DdsTypeName("std_msgs/msg/String") == "std_msgs::msg::dds_::String_",
        "an interface type maps onto its DDS name");
    Assert(Ros2.TypeHashDigest(ChatterHash)?.Length == 32, "a RIHS01 hash carries 32 bytes");
    Assert(Ros2.TypeHashDigest("not a hash") is null, "and a malformed one carries none");
    Assert(
        Ros2.EntityKey(0, "/chatter", "std_msgs/msg/String", ChatterHash)
            == $"0/chatter/std_msgs::msg::dds_::String_/{ChatterHash}",
        "an entity key matches the published example");

    var command = new Ros2Twist(new Vector3(1.5, 0.0, 0.0), new Vector3(0.0, 0.0, -0.25));
    Ros2Twist? decoded = Ros2.TwistFromCdr(Ros2.TwistToCdr(command));
    Assert(decoded == command, "a twist survives a CDR round trip");
    Assert(Ros2.TwistFromCdr(Array.Empty<byte>()) is null, "and empty bytes decode to nothing");

    byte[] encoded;
    using (var writer = new CdrWriter())
    {
        writer.WriteUInt32(7);
        writer.WriteDouble(2.5);
        writer.WriteInt32(-3);
        encoded = writer.ToBytes();
    }

    using (var reader = new CdrReader(encoded))
    {
        Assert(reader.ReadUInt32() == 7u, "the first word reads back");
        Assert(reader.ReadDouble() == 2.5, "an eight-byte field keeps its alignment");
        Assert(reader.ReadInt32() == -3, "and the field after it is not skewed");
        Assert(reader.ReadUInt32() is null, "reading past the end yields nothing");
    }

    Assert(KeyExpression.IsValid("fleet/*/battery"), "a wildcard expression is valid");
    Assert(
        KeyExpression.Matches("fleet/*/battery", "fleet/n7/battery"),
        "and selects a node beneath it");
    Assert(
        !KeyExpression.Matches("fleet/*/battery", "fleet/n7/rack/battery"),
        "but one wildcard spans one segment");
    Assert(
        KeyExpression.Canonize("fleet/**/**/battery") == "fleet/**/battery",
        "a redundant double wildcard canonizes away");
}


static void ConformProfile(JsonElement vector, double tolerance)
{
    JsonElement coldChain = vector.GetProperty("coldChain");
    using var fridge = Profile.VaccineFridgeMonitor();
    Assert(fridge.Name == coldChain.GetProperty("name").GetString(), "the preset name");
    Assert(fridge.Topic == coldChain.GetProperty("topic").GetString(), "the publish topic");
    AssertControl(fridge.Control, coldChain.GetProperty("control"), tolerance);

    JsonElement power = coldChain.GetProperty("power");
    Assert(
        fridge.Power.ActiveSecs == power.GetProperty("activeSecs").GetUInt64(),
        "the active cadence");
    Close(fridge.Power.SaverBelow, (float)power.GetProperty("saverBelow").GetDouble(), tolerance,
        "the saver threshold");

    using (Controller control = fridge.Controller())
    {
        AssertReactions(control, coldChain.GetProperty("reactions"), tolerance);
    }

    JsonElement draining = vector.GetProperty("draining");
    using var well = Profile.WellLevel();
    Assert(well.Name == draining.GetProperty("name").GetString(), "the preset name");
    AssertControl(well.Control, draining.GetProperty("control"), tolerance);

    using (Controller level = well.Controller())
    {
        AssertReactions(level, draining.GetProperty("reactions"), tolerance);
    }

    using (Controller observer = Controller.Monitor())
    {
        JsonElement observed = vector.GetProperty("observed");
        Reaction seen = observer.Evaluate((float)observed.GetProperty("reading").GetDouble());
        Assert(seen.Actuator is null, "a monitoring profile drives no output");
        Assert(seen.Alert is null, "and raises nothing");
        Assert(
            observed.GetProperty("alert").GetProperty("kind").GetString() == "None",
            "which is what the vectors record");
    }
}

static void AssertReactions(Controller control, JsonElement reactions, double tolerance)
{
    foreach (JsonElement want in reactions.EnumerateArray())
    {
        double reading = want.GetProperty("reading").GetDouble();
        Reaction reaction = control.Evaluate((float)reading);

        JsonElement actuator = want.GetProperty("actuator");
        bool? expected = actuator.ValueKind == JsonValueKind.Null
            ? null
            : actuator.GetBoolean();
        Assert(reaction.Actuator == expected, $"the output setting at {reading}");

        JsonElement alert = want.GetProperty("alert");
        string kind = alert.GetProperty("kind").GetString()!;
        if (kind == "None")
        {
            Assert(reaction.Alert is null, $"no alert at {reading}");
            continue;
        }

        Assert(reaction.Alert is not null, $"an alert at {reading}");
        Assert(
            reaction.Alert!.Value.Kind.ToString() == kind,
            $"the alert raised at {reading}");
        switch (kind)
        {
            case "OutOfRange":
                Close(
                    reaction.Alert.Value.Reading ?? 0f,
                    (float)alert.GetProperty("reading").GetDouble(),
                    tolerance,
                    "the offending reading");
                break;
            case "RunningOut":
                Assert(
                    reaction.Alert.Value.Samples == alert.GetProperty("samples").GetUInt32(),
                    "the samples until empty");
                break;
            case "ChangingFast":
                Close(
                    reaction.Alert.Value.Rate ?? 0f,
                    (float)alert.GetProperty("rate").GetDouble(),
                    tolerance,
                    "the rate of change");
                break;
        }
    }
}

static void AssertControl(ControlPolicy policy, JsonElement want, double tolerance)
{
    string kind = want.GetProperty("kind").GetString()!;
    Assert(policy.Kind.ToString() == kind, "the policy kind");
    switch (kind)
    {
        case "Setpoint":
            Close(policy.Setpoint ?? 0f, (float)want.GetProperty("setpoint").GetDouble(), tolerance,
                "the setpoint");
            Close(policy.Hysteresis ?? 0f, (float)want.GetProperty("hysteresis").GetDouble(), tolerance,
                "the hysteresis");
            Assert(policy.Cooling == want.GetProperty("cooling").GetBoolean(), "the direction");
            Close(policy.SafeBand ?? 0f, (float)want.GetProperty("safeBand").GetDouble(), tolerance,
                "the safe band");
            break;
        case "Level":
            Close(policy.Empty ?? 0f, (float)want.GetProperty("empty").GetDouble(), tolerance,
                "the empty level");
            Assert(
                policy.WarnWithin == want.GetProperty("warnWithin").GetUInt32(),
                "the warning horizon");
            break;
        case "Surge":
            Assert(policy.Rising == want.GetProperty("rising").GetBoolean(), "the direction");
            Close(policy.Limit ?? 0f, (float)want.GetProperty("limit").GetDouble(), tolerance,
                "the limit");
            break;
    }
}

static void ConformRos2(JsonElement vector, double tolerance)
{
    foreach (JsonElement want in vector.GetProperty("names").EnumerateArray())
    {
        string name = want.GetProperty("name").GetString()!;
        Assert(
            Ros2.IsValidName(name) == want.GetProperty("valid").GetBoolean(),
            $"whether {name} obeys the ROS 2 rules");
        Assert(
            Ros2.IsFullyQualified(name) == want.GetProperty("fullyQualified").GetBoolean(),
            $"whether {name} is fully qualified");
    }

    foreach (JsonElement want in vector.GetProperty("ddsTopics").EnumerateArray())
    {
        string fqn = want.GetProperty("fqn").GetString()!;
        EntityKind kind = Enum.Parse<EntityKind>(want.GetProperty("kind").GetString()!);
        Assert(
            Ros2.DdsTopic(fqn, kind) == want.GetProperty("topic").GetString(),
            $"the DDS topic for {fqn}");
    }

    foreach (JsonProperty prefix in vector.GetProperty("prefixes").EnumerateObject())
    {
        EntityKind kind = Enum.Parse<EntityKind>(prefix.Name);
        Assert(Ros2.PrefixFor(kind) == prefix.Value.GetString(), $"the {prefix.Name} prefix");
    }

    JsonElement mangled = vector.GetProperty("mangled");
    Assert(
        Ros2.PercentMangle(mangled.GetProperty("name").GetString()!)
            == mangled.GetProperty("mangled").GetString(),
        "the mangled name");

    foreach (JsonElement want in vector.GetProperty("typeNames").EnumerateArray())
    {
        string rosType = want.GetProperty("rosType").GetString()!;
        Assert(
            Ros2.DdsTypeName(rosType) == want.GetProperty("ddsType").GetString(),
            $"the DDS type name for {rosType}");
    }

    JsonElement typeHash = vector.GetProperty("typeHash");
    string text = typeHash.GetProperty("text").GetString()!;
    Assert(
        Convert.ToHexString(Ros2.TypeHashDigest(text)!).ToLowerInvariant()
            == typeHash.GetProperty("digest").GetString(),
        "the digest a RIHS01 string carries");

    JsonElement key = vector.GetProperty("entityKey");
    Assert(
        Ros2.EntityKey(
            key.GetProperty("domainId").GetUInt32(),
            key.GetProperty("fqn").GetString()!,
            key.GetProperty("rosType").GetString()!,
            text) == key.GetProperty("key").GetString(),
        "the Zenoh key an rmw_zenoh peer publishes on");

    JsonElement twist = vector.GetProperty("twist");
    double[] linear = Doubles(twist.GetProperty("linear"));
    double[] angular = Doubles(twist.GetProperty("angular"));
    var command = new Ros2Twist(
        new Vector3(linear[0], linear[1], linear[2]),
        new Vector3(angular[0], angular[1], angular[2]));

    byte[] encoded = Ros2.TwistToCdr(command);
    Assert(
        Convert.ToHexString(encoded).ToLowerInvariant() == twist.GetProperty("cdr").GetString(),
        "a twist encodes to the same CDR everywhere");
    Assert(Ros2.TwistFromCdr(encoded) == command, "and decodes back unchanged");

    JsonElement mixed = vector.GetProperty("mixedWidths");
    using var reader = new CdrReader(
        Convert.FromHexString(mixed.GetProperty("cdr").GetString()!));
    Assert(reader.ReadUInt32() == mixed.GetProperty("word").GetUInt32(), "the first word");
    Close(
        (float)(reader.ReadDouble() ?? 0),
        (float)mixed.GetProperty("double").GetDouble(),
        tolerance,
        "an eight-byte field keeps its alignment");
    Assert(
        reader.ReadInt32() == mixed.GetProperty("signed").GetInt32(),
        "and the field after it is not skewed");
}

static double[] Doubles(JsonElement array)
{
    var values = new List<double>();
    foreach (JsonElement entry in array.EnumerateArray())
    {
        values.Add(entry.GetDouble());
    }

    return values.ToArray();
}

static void ConformZenoh(JsonElement vector)
{
    foreach (JsonElement want in vector.GetProperty("expressions").EnumerateArray())
    {
        string key = want.GetProperty("key").GetString()!;
        Assert(
            KeyExpression.IsValid(key) == want.GetProperty("valid").GetBoolean(),
            $"whether {key} is well formed");
        Assert(
            KeyExpression.IsCanon(key) == want.GetProperty("canon").GetBoolean(),
            $"whether {key} is already canonical");
    }

    foreach (JsonElement want in vector.GetProperty("canonized").EnumerateArray())
    {
        string key = want.GetProperty("key").GetString()!;
        Assert(
            KeyExpression.Canonize(key) == want.GetProperty("canonical").GetString(),
            $"the canonical form of {key}");
    }

    foreach (JsonElement want in vector.GetProperty("matches").EnumerateArray())
    {
        string pattern = want.GetProperty("pattern").GetString()!;
        string key = want.GetProperty("key").GetString()!;
        Assert(
            KeyExpression.Matches(pattern, key) == want.GetProperty("matches").GetBoolean(),
            $"whether {pattern} selects {key}");
    }
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
    ConformLoraRegions(vectors.GetProperty("loraRegions"));
    ConformMavlink(vectors.GetProperty("mavlink"));
    ConformMesh(vectors.GetProperty("mesh"));
    ConformRouting(vectors.GetProperty("routing"));
    ConformLorawan(vectors.GetProperty("lorawan"));
    ConformHeader(vectors.GetProperty("header"));
    ConformNetwork(vectors.GetProperty("network"));
    ConformAudit(vectors.GetProperty("audit"));
    ConformSession(vectors.GetProperty("session"));
    ConformUpdate(vectors.GetProperty("update"));
    ConformPower(vectors.GetProperty("power"));
    ConformTelemetry(vectors.GetProperty("telemetry"));
    ConformLadder(vectors.GetProperty("ladder")).GetAwaiter().GetResult();
    ConformSimulation(vectors.GetProperty("simulation")).GetAwaiter().GetResult();
    ConformProfile(vectors.GetProperty("profile"), tolerance);
    ConformRos2(vectors.GetProperty("ros2"), tolerance);
    ConformZenoh(vectors.GetProperty("zenoh"));

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

    RegionalPlans();
    MavlinkWire();

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


// What a band allows, and what a deployment on its own spectrum allows instead.
// The plan reports; it never refuses a transmission.
static void RegionalPlans()
{
    using var eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
    Assert(eu868.Name == "EU863-870", "the plan names its band");
    Assert(eu868.LinkSettings(0)!.SpreadingFactor == 12, "EU868 DR0 is the slowest LoRa rate");
    Assert(eu868.DutyCyclePermille(868_100_000) == 10, "the 868.1 MHz sub-band is 1%");
    Assert(eu868.MaxEirpDbm(868_100_000) == 16, "and is capped at 16 dBm");
    Assert(eu868.MaxPayload(5)!.Value.Application == 242, "DR5 carries the largest payload");
    Assert(eu868.Rx1DataRate(5, 0) == 5, "RX1 at offset 0 mirrors the uplink rate");
    Assert(eu868.Rx2().FrequencyHz == 869_525_000, "RX2 listens on 869.525 MHz");
    Assert(eu868.NextBackoffDataRate(0) is null, "DR0 has nothing slower to fall back to");
    Assert(eu868.SubBands().Count == 2, "EU868 describes two sub-bands");

    // EU868 defines every number it has, including the LR-FHSS rates.
    LoraDataRate fhss = eu868.DataRate(9)!.Value;
    Assert(fhss.Kind == LoraModulation.LrFhss, "EU868 DR9 is LR-FHSS");
    Assert(fhss.CodingRateNumerator == 2, "at coding rate 2/3");
    Assert(eu868.DataRate(200) is null, "a number past the end of the table is absent");

    // A number the region reserves is told from one it never defines.
    using var us915 = LoraChannelPlan.ForRegion(LoraRegion.Us915);
    Assert(
        us915.DataRate(2, LoraDirection.Downlink)!.Value.Kind == LoraModulation.Reserved,
        "US915 reserves downlink DR2");
    Assert(
        us915.DataRate(8, LoraDirection.Downlink)!.Value.Kind == LoraModulation.Lora,
        "and starts its downlink rates at DR8");
    Assert(
        us915.DutyCyclePermille(903_000_000) is null,
        "the FCC caps dwell time rather than duty cycle, so US915 describes no sub-band");

    using var au915 = LoraChannelPlan.ForRegion(LoraRegion.Au915);
    Assert(au915.Info().HasDwellTimeLimit, "AU915 does limit dwell time");

    // Every published region resolves in this build.
    foreach (LoraRegion region in Enum.GetValues<LoraRegion>())
    {
        Assert(LoraChannelPlan.IsAvailable(region), $"{region} is compiled into this build");
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(region);
        Assert(plan.Name.Length > 0, $"{region} names its band");
    }

    // A private deployment on licensed spectrum answers the same questions.
    using LoraChannelPlan licensed = new LoraPlanBuilder("private-915")
        .DataRate(LoraDataRate.ForLora(12, 125_000, 250))
        .DataRate(LoraDataRate.ForLora(7, 125_000, 5_470))
        .MaxPayload(new LoraMaxPayload(59, 51))
        .MaxPayload(new LoraMaxPayload(230, 222))
        .ChannelBlock(new LoraChannelBlock(915_000_000, 500_000, 4, 0, 1))
        .SubBand(new LoraSubBand(915_000_000, 917_000_000, 1000, 30))
        .Rx(915_000_000)
        .Rx1Row([0])
        .Rx1Row([1])
        .Build();

    Assert(licensed.Name == "private-915", "a private plan keeps its name");
    Assert(licensed.ChannelFrequencyHz(3) == 916_500_000, "four channels, 500 kHz apart");
    Assert(
        licensed.DutyCyclePermille(915_500_000) == 1000,
        "licensed spectrum is reported as unrestricted, not refused");
    Assert(licensed.MaxEirpDbm(915_500_000) == 30, "and carries the power its licence allows");
    Assert(
        licensed.MaxPayload(1, LoraPayloadTable.DownlinkDirect)!.Value.Application == 222,
        "an empty downlink table mirrors the uplink one");
    Assert(
        licensed.NextBackoffDataRate(1) == 0,
        "an unset back-off chain steps down one rate at a time");
    Assert(licensed.ChannelBlocks()[0].Count == 4, "and lists the channels it was given");

    // A plan that would answer a question wrongly is refused where it is built.
    bool refused = false;
    try
    {
        using var narrow = new LoraPlanBuilder("too-narrow");
        narrow
            .DataRate(LoraDataRate.ForLora(12, 125_000, 250))
            .Rx(915_000_000, 0, 5)
            .Rx1Row([0])
            .Build();
    }
    catch (PamojaException)
    {
        refused = true;
    }

    Assert(refused, "offsets up to 5 need six entries in every RX1 row");

    // And a spent builder cannot be built twice.
    var spent = new LoraPlanBuilder("spent");
    spent.DataRate(LoraDataRate.ForLora(12, 125_000, 250)).Rx(915_000_000).Rx1Row([0]);
    spent.Build().Dispose();
    bool spentRefused = false;
    try
    {
        spent.Build();
    }
    catch (PamojaException)
    {
        spentRefused = true;
    }

    Assert(spentRefused, "a builder is spent once built");
}


// Regional channel plans: every binding must report the same facts about each
// band, and must assemble a private plan that answers the same questions.
static void ConformLoraRegions(JsonElement vector)
{
    JsonElement published = vector.GetProperty("published");
    LoraRegion[] regions = Enum.GetValues<LoraRegion>();
    Assert(
        published.GetArrayLength() == regions.Length,
        "every published region is described");

    int index = 0;
    foreach (JsonElement want in published.EnumerateArray())
    {
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(regions[index]);
        ConformPlan(plan, want);
        index++;
    }

    using LoraChannelPlan custom = new LoraPlanBuilder("private-915")
        .DataRate(LoraDataRate.ForLora(12, 125_000, 250))
        .DataRate(LoraDataRate.ForLora(7, 125_000, 5_470))
        .MaxPayload(new LoraMaxPayload(59, 51), LoraPayloadTable.UplinkRepeater)
        .MaxPayload(new LoraMaxPayload(230, 222), LoraPayloadTable.UplinkRepeater)
        .MaxPayload(new LoraMaxPayload(59, 51), LoraPayloadTable.UplinkDirect)
        .MaxPayload(new LoraMaxPayload(230, 222), LoraPayloadTable.UplinkDirect)
        .ChannelBlock(new LoraChannelBlock(915_000_000, 500_000, 4, 0, 1))
        .SubBand(new LoraSubBand(915_000_000, 917_000_000, 1000, 30))
        .Power(30, 2, 7)
        .Rx(915_000_000, 0, 0)
        .Rx1Row([0])
        .Rx1Row([1])
        .Build();

    ConformPlan(custom, vector.GetProperty("custom"));
}

// Holds one channel plan to the answers every binding must give.
static void ConformPlan(LoraChannelPlan plan, JsonElement want)
{
    string where = want.GetProperty("name").GetString()!;
    LoraPlanInfo info = plan.Info();
    Assert(plan.Name == where, $"the name of {where}");
    Assert(
        info.UplinkDataRateCount == want.GetProperty("uplinkDataRateCount").GetUInt16(),
        $"uplink rates of {where}");
    Assert(
        info.DownlinkDataRateCount == want.GetProperty("downlinkDataRateCount").GetUInt16(),
        $"downlink rates of {where}");
    Assert(
        info.DefaultChannelCount == want.GetProperty("defaultChannelCount").GetUInt16(),
        $"default channels of {where}");
    Assert(
        info.MaxRx1DataRateOffset == want.GetProperty("maxRx1DataRateOffset").GetByte(),
        $"RX1 offsets of {where}");
    Assert(
        info.HasDwellTimeLimit == want.GetProperty("hasDwellTimeLimit").GetBoolean(),
        $"dwell limit of {where}");

    JsonElement rx2 = want.GetProperty("rx2");
    Assert(
        plan.Rx2().FrequencyHz == rx2.GetProperty("frequencyHz").GetUInt32(),
        $"RX2 frequency of {where}");
    Assert(
        plan.Rx2().DataRate == rx2.GetProperty("dataRate").GetByte(),
        $"RX2 data rate of {where}");

    byte fastest = (byte)(info.UplinkDataRateCount - 1);
    ConformDataRate(plan.DataRate(0), want.GetProperty("slowestUplink"), where);
    ConformDataRate(plan.DataRate(fastest), want.GetProperty("fastestUplink"), where);
    ConformDataRate(
        plan.DataRate(0, LoraDirection.Downlink),
        want.GetProperty("slowestDownlink"),
        where);

    JsonElement atSlowest = want.GetProperty("payloadAtSlowest");
    ConformPayload(
        plan.MaxPayload(0, LoraPayloadTable.UplinkRepeater),
        atSlowest.GetProperty("repeater"),
        where);
    ConformPayload(
        plan.MaxPayload(0, LoraPayloadTable.UplinkDirect),
        atSlowest.GetProperty("direct"),
        where);
    ConformPayload(
        plan.MaxPayload(0, LoraPayloadTable.DwellLimited),
        want.GetProperty("dwellLimitedAtSlowest"),
        where);

    uint probe = want.GetProperty("probeFrequencyHz").GetUInt32();
    JsonElement duty = want.GetProperty("dutyCyclePermilleAtProbe");
    uint? permille = plan.DutyCyclePermille(probe);
    if (duty.ValueKind == JsonValueKind.Null)
    {
        Assert(permille is null, $"the duty cycle of {where}");
    }
    else
    {
        Assert(permille == duty.GetUInt32(), $"the duty cycle of {where}");
    }

    Assert(
        plan.MaxEirpDbm(probe) == want.GetProperty("maxEirpDbmAtProbe").GetSByte(),
        $"the EIRP ceiling of {where}");

    byte offset = 0;
    foreach (JsonElement entry in want.GetProperty("rx1RowForSlowest").EnumerateArray())
    {
        byte? got = plan.Rx1DataRate(0, offset);
        Assert(
            entry.ValueKind == JsonValueKind.Null ? got is null : got == entry.GetByte(),
            $"RX1 offset {offset} of {where}");
        offset++;
    }

    ConformOptionalByte(
        plan.NextBackoffDataRate(fastest),
        want.GetProperty("backoffFromFastest"),
        $"back-off from the fastest rate of {where}");
    ConformOptionalByte(
        plan.NextBackoffDataRate(0),
        want.GetProperty("backoffFromSlowest"),
        $"back-off from the slowest rate of {where}");

    ushort channel = 0;
    foreach (JsonElement entry in want.GetProperty("channelFrequencies").EnumerateArray())
    {
        Assert(
            plan.ChannelFrequencyHz(channel) == entry.GetUInt32(),
            $"channel {channel} of {where}");
        channel++;
    }

    JsonElement bands = want.GetProperty("subBands");
    IReadOnlyList<LoraSubBand> got_bands = plan.SubBands();
    Assert(got_bands.Count == bands.GetArrayLength(), $"sub-bands of {where}");
    int band = 0;
    foreach (JsonElement entry in bands.EnumerateArray())
    {
        Assert(got_bands[band].StartHz == entry.GetProperty("startHz").GetUInt32(), where);
        Assert(got_bands[band].EndHz == entry.GetProperty("endHz").GetUInt32(), where);
        Assert(
            got_bands[band].DutyCyclePermille
                == entry.GetProperty("dutyCyclePermille").GetUInt32(),
            where);
        Assert(
            got_bands[band].MaxEirpDbm == entry.GetProperty("maxEirpDbm").GetSByte(),
            where);
        band++;
    }
}

// Checks a data rate against the vector describing it.
static void ConformDataRate(LoraDataRate? rate, JsonElement want, string where)
{
    string kind = want.GetProperty("kind").GetString()!;
    if (rate is null)
    {
        Fail($"a data rate is missing in {where}");
        return;
    }

    LoraDataRate got = rate.Value;
    string gotKind = got.Kind switch
    {
        LoraModulation.Lora => "lora",
        LoraModulation.Fsk => "fsk",
        LoraModulation.LrFhss => "lr_fhss",
        _ => "reserved",
    };
    Assert(gotKind == kind, $"the modulation in {where}");
    Assert(
        got.BitrateBps == want.GetProperty("bitrateBps").GetUInt32(),
        $"the bitrate in {where}");
    ConformOptionalUint(got.BandwidthHz, want.GetProperty("bandwidthHz"), $"bandwidth in {where}");
    ConformOptionalByte(
        got.SpreadingFactor,
        want.GetProperty("spreadingFactor"),
        $"spreading factor in {where}");
    ConformOptionalByte(
        got.CodingRateNumerator,
        want.GetProperty("codingRateNumerator"),
        $"coding-rate numerator in {where}");
    ConformOptionalByte(
        got.CodingRateDenominator,
        want.GetProperty("codingRateDenominator"),
        $"coding-rate denominator in {where}");
}

// Checks a payload limit against the vector describing it.
static void ConformPayload(LoraMaxPayload? payload, JsonElement want, string where)
{
    if (want.ValueKind == JsonValueKind.Null)
    {
        Assert(payload is null, $"an absent payload limit in {where}");
        return;
    }

    Assert(payload is not null, $"a payload limit in {where}");
    Assert(
        payload!.Value.MacPayload == want.GetProperty("macPayload").GetUInt16(),
        $"the MAC payload in {where}");
    Assert(
        payload.Value.Application == want.GetProperty("application").GetUInt16(),
        $"the application payload in {where}");
}

// Checks a value the vectors may report as null.
static void ConformOptionalByte(byte? got, JsonElement want, string message)
{
    Assert(
        want.ValueKind == JsonValueKind.Null ? got is null : got == want.GetByte(),
        message);
}

// Checks a wider value the vectors may report as null.
static void ConformOptionalUint(uint? got, JsonElement want, string message)
{
    Assert(
        want.ValueKind == JsonValueKind.Null ? got is null : got == want.GetUInt32(),
        message);
}


// Talking to an autopilot: framing a message, reading it back off a link that
// splits and garbles it, and proving a signed frame came from who it claims.
static void MavlinkWire()
{
    MavlinkHeader header = new(1, 1, 7);
    // HEARTBEAT announcing an onboard controller in an active state.
    ReadOnlySpan<byte> heartbeat = [0, 0, 0, 0, 18, 0, 0, 4, 3];

    Assert(Mavlink.KnownCrcExtra(0) == 50, "HEARTBEAT's published CRC_EXTRA");
    Assert(Mavlink.KnownCrcExtra(9999) is null, "an id outside the common dialect");

    using MavlinkFrame frame = Mavlink.Frame(header, 0, heartbeat);
    Assert(frame.Version == MavlinkVersion.V2, "v2 is the current wire format");
    Assert(frame.MessageId == 0, "and the id survives");
    Assert(!frame.Signed, "an ordinary frame carries no signature");
    Assert(frame.Signature is null, "so there is none to read");
    Assert(frame.Header == header, "the addressing fields survive");

    byte[] wire = frame.Bytes;
    using (MavlinkFrame received = MavlinkFrame.ParseKnown(wire))
    {
        Assert(received.MessageId == 0, "the frame reads back");
        Assert(received.Payload.AsSpan().SequenceEqual(heartbeat), "with its payload intact");
    }

    // A frame mangled in transit is refused rather than acted on.
    byte[] mangled = (byte[])wire.Clone();
    mangled[12] ^= 0xFF;
    AssertThrows(() => MavlinkFrame.ParseKnown(mangled).Dispose(), "a corrupt frame is refused");

    // A parser joins a stream already in progress and survives arbitrary splits.
    using (MavlinkParser parser = new())
    {
        Assert(
            parser.Push(new byte[] { 0x11, 0x22, 0x33 }).Count == 0,
            "noise between frames is skipped, not reported");
        Assert(parser.Push(wire.AsSpan(0, 5)).Count == 0, "half a frame is not a frame");

        IReadOnlyList<MavlinkFrame> found = parser.Push(wire.AsSpan(5));
        Assert(found.Count == 1, "the rest of it completes one");
        Assert(found[0].MessageId == 0, "and it is the frame that was sent");
        foreach (MavlinkFrame each in found)
        {
            each.Dispose();
        }

        Assert(parser.Pending == 0, "a drained parser holds nothing");
    }

    // A private dialect: describe the message once, and it checks from then on.
    using MavlinkDialect dialect = new();
    MavlinkField[] fields = [new MavlinkField("uint32_t", "uptime")];
    byte seed = dialect.AddMessage(50_000, "PRIVATE_STATUS", fields);
    Assert(
        seed == Mavlink.MessageCrcExtra("PRIVATE_STATUS", fields),
        "the seed is derived, not invented");
    Assert(dialect.CrcExtra(50_000) == seed, "the dialect keeps it");
    Assert(dialect.CrcExtra(0) == 50, "and the common dialect still answers");

    using MavlinkFrame priv = MavlinkFrame.Raw(header, 50_000, seed, BitConverter.GetBytes(42u));
    byte[] privWire = priv.Bytes;
    AssertThrows(
        () => MavlinkFrame.ParseKnown(privWire).Dispose(),
        "the common registry alone cannot check a private message");

    using (MavlinkFrame back = MavlinkFrame.ParseKnown(privWire, dialect))
    {
        Assert(back.MessageId == 50_000, "but the dialect can");
        // MAVLink 2 drops trailing zero bytes, so a four-byte field holding 42
        // arrives as one byte; a decoder zero-extends it.
        Assert(back.Payload.AsSpan().SequenceEqual(new byte[] { 42 }), "and the payload is truncated");
    }

    // Signing: a ground station trusts a command came from the vehicle it expects.
    byte[] key = new byte[Mavlink.KeyLength];
    key.AsSpan().Fill(7);
    using MavlinkSigner signer = new(key, linkId: 1, timestamp: Mavlink.TimestampNow());
    Assert(signer.LinkId == 1, "the signer knows its link");

    using MavlinkFrame signed = signer.Sign(header, 0, heartbeat, 50);
    Assert(signed.Signed, "a signed frame says so");
    Assert(signed.Signature!.Length == Mavlink.SignatureLength, "and carries a full block");
    Assert(signed.Signature![0] == 1, "the link id leads the signature block");

    using MavlinkVerifier verifier = new(key);
    verifier.Verify(signed);
    AssertThrows(() => verifier.Verify(signed), "the same frame a second time is a replay");

    byte[] otherKey = new byte[Mavlink.KeyLength];
    otherKey.AsSpan().Fill(9);
    using MavlinkVerifier stranger = new(otherKey);
    AssertThrows(() => stranger.Verify(signed), "a different key is a different sender");

    using MavlinkVerifier strict = new(key);
    AssertThrows(
        () => strict.Verify(frame),
        "an unsigned frame is never silently treated as authentic");
}

// Runs an action that must throw, and fails the suite if it does not.
static void AssertThrows(Action action, string message)
{
    try
    {
        action();
    }
    catch (Exception)
    {
        return;
    }

    Fail(message);
}


// The MAVLink wire layer: the bytes a sender puts on the wire are pinned
// exactly, because a protocol that is self-consistent but wrong is what this
// suite exists to catch.
static void ConformMavlink(JsonElement vector)
{
    foreach (JsonElement entry in vector.GetProperty("crc16").EnumerateArray())
    {
        byte[] input = Convert.FromHexString(entry.GetProperty("input").GetString()!);
        Assert(
            Mavlink.Crc16(input) == entry.GetProperty("checksum").GetUInt16(),
            "a published checksum");
    }

    foreach (JsonElement entry in vector.GetProperty("knownCrcExtra").EnumerateArray())
    {
        uint msgid = entry.GetProperty("msgid").GetUInt32();
        Assert(
            Mavlink.KnownCrcExtra(msgid) == entry.GetProperty("crcExtra").GetByte(),
            $"the published CRC_EXTRA of message {msgid}");
    }

    Assert(
        Mavlink.KnownCrcExtra(vector.GetProperty("unknownCrcExtra").GetUInt32()) is null,
        "an id outside the common dialect has no seed here");

    // A seed derived from a definition must equal the published one.
    foreach (JsonElement described in vector.GetProperty("derivedCrcExtra").EnumerateArray())
    {
        List<MavlinkField> fields = [];
        foreach (JsonElement field in described.GetProperty("fields").EnumerateArray())
        {
            fields.Add(new MavlinkField(
                field.GetProperty("type").GetString()!,
                field.GetProperty("name").GetString()!,
                field.GetProperty("arrayLen").GetByte()));
        }

        string name = described.GetProperty("name").GetString()!;
        Assert(
            Mavlink.MessageCrcExtra(name, fields) == described.GetProperty("crcExtra").GetByte(),
            $"the derived CRC_EXTRA of {name}");
    }

    JsonElement head = vector.GetProperty("header");
    MavlinkHeader header = new(
        head.GetProperty("systemId").GetByte(),
        head.GetProperty("componentId").GetByte(),
        head.GetProperty("sequence").GetByte());
    byte[] payload = Convert.FromHexString(vector.GetProperty("payload").GetString()!);

    foreach (JsonElement described in vector.GetProperty("frames").EnumerateArray())
    {
        string name = described.GetProperty("name").GetString()!;
        uint msgid = described.GetProperty("msgid").GetUInt32();
        byte crcExtra = described.GetProperty("crcExtra").GetByte();
        byte[] want = Convert.FromHexString(described.GetProperty("bytes").GetString()!);

        using MavlinkFrame built = described.GetProperty("version").GetByte() == 1
            ? MavlinkFrame.EncodeV1(header, msgid, payload, crcExtra)
            : msgid == 50_000
                ? MavlinkFrame.EncodeV2(
                    new MavlinkHeader(9, 1),
                    msgid,
                    BitConverter.GetBytes(42u),
                    crcExtra)
                : MavlinkFrame.EncodeV2(header, msgid, payload, crcExtra);
        Assert(built.Bytes.AsSpan().SequenceEqual(want), $"the wire bytes of {name}");

        using MavlinkFrame parsed = MavlinkFrame.Parse(want, crcExtra);
        Assert(parsed.MessageId == msgid, $"the id of {name}");
        Assert(
            Convert.ToHexString(parsed.Payload).ToLowerInvariant()
                == described.GetProperty("payload").GetString(),
            $"the payload of {name}");
        Assert(
            parsed.Signed == described.GetProperty("signed").GetBoolean(),
            $"whether {name} is signed");
        Assert(
            parsed.IncompatFlags == described.GetProperty("incompatFlags").GetByte(),
            $"the flags of {name}");

        // A parser fed the same bytes must find the same frame.
        using MavlinkDialect dialect = new();
        dialect.Add(msgid, crcExtra);
        using MavlinkParser parser = new();
        IReadOnlyList<MavlinkFrame> found = parser.Push(want, dialect);
        Assert(found.Count == 1, $"the parser finds {name}");
        Assert(found[0].Bytes.AsSpan().SequenceEqual(want), $"and recovers {name} whole");
        foreach (MavlinkFrame each in found)
        {
            each.Dispose();
        }
    }

    // Signing is deterministic given the key, link and timestamp.
    JsonElement signed = vector.GetProperty("signed");
    byte[] key = Convert.FromHexString(signed.GetProperty("key").GetString()!);
    using MavlinkSigner signer = new(
        key,
        signed.GetProperty("linkId").GetByte(),
        signed.GetProperty("timestamp").GetUInt64());
    using MavlinkFrame frame = signer.Sign(
        header,
        signed.GetProperty("msgid").GetUInt32(),
        payload,
        signed.GetProperty("crcExtra").GetByte());
    Assert(
        Convert.ToHexString(frame.Bytes).ToLowerInvariant()
            == signed.GetProperty("bytes").GetString(),
        "the bytes of a signed frame");
    Assert(
        Convert.ToHexString(frame.Signature!).ToLowerInvariant()
            == signed.GetProperty("signature").GetString(),
        "and its signature block");

    using MavlinkVerifier verifier = new(key);
    verifier.Verify(frame);
    AssertThrows(() => verifier.Verify(frame), "the same timestamp again is a replay");

    foreach (JsonElement entry in vector.GetProperty("timestamps").EnumerateArray())
    {
        Assert(
            Mavlink.TimestampFromUnixMicros(entry.GetProperty("unixMicros").GetUInt64())
                == entry.GetProperty("timestamp").GetUInt64(),
            "a signing timestamp");
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
            "a spreading factor outside 5 to 12 is clamped");
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

    using var seen = new SeenPackets(vector.GetProperty("seenCapacity").GetInt32());
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

    JsonElement sized = vector.GetProperty("sizedSeen");
    using var small = new SeenPackets(sized.GetProperty("capacity").GetInt32());
    Assert(
        small.Capacity == sized.GetProperty("capacity").GetInt32(),
        "the size it was given");
    foreach (JsonElement key in sized.GetProperty("keys").EnumerateArray())
    {
        small.Record(key[0].GetUInt32(), key[1].GetUInt16());
    }

    JsonElement evicted = sized.GetProperty("evicted");
    Assert(
        !small.Contains(evicted[0].GetUInt32(), evicted[1].GetUInt16()),
        "a cache sized by the caller evicts at that size");
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
    using var router = new Router(
        vector.GetProperty("address").GetUInt32(),
        vector.GetProperty("capacity").GetInt32());
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

    JsonElement sized = vector.GetProperty("sized");
    using var small = new Router(0x01, sized.GetProperty("capacity").GetInt32());
    Assert(
        small.Capacity == sized.GetProperty("capacity").GetInt32(),
        "the size it was given");
    for (uint node = 0; node < sized.GetProperty("offered").GetUInt32(); node++)
    {
        small.Observe(node + 0x100, 0x05, 4);
    }

    Assert(
        small.Count == sized.GetProperty("learned").GetInt32(),
        "a table sized by the caller holds exactly what it was asked for");
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

static void ConformHeader(JsonElement vector)
{
    foreach (JsonElement want in vector.GetProperty("frames").EnumerateArray())
    {
        LorawanHeader header = Lorawan.ParseHeader(
            Convert.FromHexString(want.GetProperty("frame").GetString()!));

        Assert(
            header.MessageType.ToString() == want.GetProperty("messageType").GetString(),
            "the message type");
        Assert(header.IsData == want.GetProperty("isData").GetBoolean(), "data or join");
        AssertOptional(header.DevAddr, want.GetProperty("devAddr"), "the address a receiver routes by");
        AssertOptional(header.Fcnt, want.GetProperty("fcnt"), "the counter");
        AssertOptional(header.Fport, want.GetProperty("fport"), "the port");
        Assert(header.Confirmed == want.GetProperty("confirmed").GetBoolean(), "the confirmed bit");
        Assert(header.Adr == want.GetProperty("adr").GetBoolean(), "the ADR bit");
        Assert(header.Ack == want.GetProperty("ack").GetBoolean(), "the ACK bit");
        Assert(header.FPending == want.GetProperty("fpending").GetBoolean(), "the pending bit");
        Assert(
            header.FoptsLength == want.GetProperty("foptsLen").GetInt32(),
            "the options length");
        Assert(
            header.PayloadLength == want.GetProperty("payloadLen").GetInt32(),
            "the payload length");
    }

    foreach (string name in new[] { "unsupported", "truncated" })
    {
        try
        {
            Lorawan.ParseHeader(Convert.FromHexString(vector.GetProperty(name).GetString()!));
            Fail($"a {name} frame must be refused");
        }
        catch (PamojaException)
        {
        }
    }
}

static void AssertOptional<T>(T? got, JsonElement want, string message)
    where T : struct
{
    if (want.ValueKind == JsonValueKind.Null)
    {
        Assert(got is null, message);
    }
    else
    {
        Assert(got is not null && got.Value.ToString() == want.ToString(), message);
    }
}

static void AssertGrant(JsonElement vector, byte[] appKey, ushort devNonce)
{
    byte[]? cflist = vector.TryGetProperty("cflist", out JsonElement list)
        ? Convert.FromHexString(list.GetString()!)
        : null;
    var grant = new LorawanGrant(
        vector.GetProperty("appNonce").GetUInt32(),
        vector.GetProperty("netId").GetUInt32(),
        vector.GetProperty("devAddr").GetUInt32(),
        vector.GetProperty("dlSettings").GetByte(),
        vector.GetProperty("rxDelay").GetByte(),
        cflist);

    Assert(
        Convert.ToHexString(grant.Accept(appKey, devNonce)).ToLowerInvariant()
            == vector.GetProperty("accept").GetString(),
        "the signed join-accept matches byte for byte");

    // Neither side sent a key, so the proof they agree is that one reads what the
    // other wrote.
    JsonElement probe = vector.GetProperty("probe");
    using LorawanSession session = grant.Session(appKey, devNonce);
    Assert(
        Convert.ToHexString(session.EncodeUplink(
            probe.GetProperty("fcnt").GetUInt32(),
            probe.GetProperty("fport").GetByte(),
            Convert.FromHexString(probe.GetProperty("payload").GetString()!)))
            .ToLowerInvariant() == probe.GetProperty("frame").GetString(),
        "the session this network derived is the one the device holds");
}

static void ConformNetwork(JsonElement vector)
{
    byte[] appKey = Convert.FromHexString(vector.GetProperty("appKey").GetString()!);

    JsonElement want = vector.GetProperty("joinRequest");
    LorawanJoinRequest request = Lorawan.ParseJoinRequest(
        Convert.FromHexString(want.GetProperty("frame").GetString()!), appKey);
    Assert(
        Convert.ToHexString(request.DevEui).ToLowerInvariant()
            == want.GetProperty("devEui").GetString(),
        "the device identifier");
    Assert(
        Convert.ToHexString(request.AppEui).ToLowerInvariant()
            == want.GetProperty("appEui").GetString(),
        "the application identifier");
    Assert(
        request.DevNonce == want.GetProperty("devNonce").GetUInt16(),
        "the nonce a network must not accept twice");

    try
    {
        Lorawan.ParseJoinRequest(
            Convert.FromHexString(vector.GetProperty("forgedRequest").GetString()!), appKey);
        Fail("a request signed with another root key must not be trusted");
    }
    catch (PamojaException)
    {
    }

    AssertGrant(vector.GetProperty("grant"), appKey, vector.GetProperty("devNonce").GetUInt16());

    // The captured join: a third party's numbers, so agreement here is not just
    // this implementation agreeing with itself.
    JsonElement published = vector.GetProperty("published");
    byte[] publishedKey = Convert.FromHexString(published.GetProperty("appKey").GetString()!);
    ushort publishedNonce = published.GetProperty("devNonce").GetUInt16();
    AssertGrant(published, publishedKey, publishedNonce);

    using var device = new LorawanDevice(new byte[8], new byte[8], publishedKey);
    using LorawanJoinAccept accepted = device.AcceptJoin(
        Convert.FromHexString(published.GetProperty("accept").GetString()!), publishedNonce);
    Assert(
        accepted.DevAddr == published.GetProperty("devAddr").GetUInt32(),
        "the captured accept activates");

    JsonElement probe = published.GetProperty("probe");
    using LorawanSession activated = accepted.Session();
    Assert(
        Convert.ToHexString(activated.EncodeUplink(
            probe.GetProperty("fcnt").GetUInt32(),
            probe.GetProperty("fport").GetByte(),
            Convert.FromHexString(probe.GetProperty("payload").GetString()!)))
            .ToLowerInvariant() == probe.GetProperty("frame").GetString(),
        "the session the device derived matches the published keys");
}

static void ConformAudit(JsonElement vector)
{
    using var keeper = new DeviceIdentity(
        Convert.FromHexString(vector.GetProperty("seed").GetString()!));
    Assert(
        Convert.ToHexString(keeper.PublicKey).ToLowerInvariant()
            == vector.GetProperty("publicKey").GetString(),
        "the key a chain is checked against");

    using var log = new AuditLog(keeper);
    List<AuditEntry> entries = [];
    foreach (JsonElement want in vector.GetProperty("entries").EnumerateArray())
    {
        AuditEntry entry = log.Append(
            System.Text.Encoding.UTF8.GetBytes(want.GetProperty("payload").GetString()!));
        Assert(entry.Index == want.GetProperty("index").GetUInt64(), "the index");
        Assert(
            Convert.ToHexString(entry.Previous).ToLowerInvariant()
                == want.GetProperty("previous").GetString(),
            "each record carries the hash of the one before it");
        Assert(
            Convert.ToHexString(entry.Digest).ToLowerInvariant()
                == want.GetProperty("digest").GetString(),
            "the digest");
        Assert(
            Convert.ToHexString(entry.Signature).ToLowerInvariant()
                == want.GetProperty("signature").GetString(),
            "the signature");
        Assert(
            Convert.ToHexString(entry.ToBytes()).ToLowerInvariant()
                == want.GetProperty("bytes").GetString(),
            "a record encodes the same in every language");
        entries.Add(entry);
    }

    Assert(Audit.VerifyChain(keeper.PublicKey, entries), "an untouched chain verifies");

    using AuditEntry tampered = AuditEntry.FromBytes(
        Convert.FromHexString(vector.GetProperty("tampered").GetString()!));
    Assert(
        !Audit.VerifyChain(keeper.PublicKey, [entries[0], entries[1], tampered]),
        "and an altered record breaks it");

    JsonElement resumedWant = vector.GetProperty("resumed");
    using AuditLog resumed = AuditLog.Resume(keeper, entries[2]);
    using AuditEntry afterReboot = resumed.Append(
        System.Text.Encoding.UTF8.GetBytes(resumedWant.GetProperty("payload").GetString()!));
    Assert(
        afterReboot.Index == resumedWant.GetProperty("index").GetUInt64(),
        "a reboot leaves no gap");
    Assert(
        Convert.ToHexString(afterReboot.ToBytes()).ToLowerInvariant()
            == resumedWant.GetProperty("bytes").GetString(),
        "and the resumed record encodes the same");

    foreach (AuditEntry entry in entries)
    {
        entry.Dispose();
    }
}

static void ConformSession(JsonElement vector)
{
    using var node = new AgreementKey(
        Convert.FromHexString(vector.GetProperty("nodeSeed").GetString()!));
    using var gateway = new AgreementKey(
        Convert.FromHexString(vector.GetProperty("gatewaySeed").GetString()!));

    Assert(
        Convert.ToHexString(node.PublicKey).ToLowerInvariant()
            == vector.GetProperty("nodePublicKey").GetString(),
        "the node key");
    Assert(
        Convert.ToHexString(gateway.PublicKey).ToLowerInvariant()
            == vector.GetProperty("gatewayPublicKey").GetString(),
        "the gateway key");

    byte[] salt = Convert.FromHexString(vector.GetProperty("salt").GetString()!);
    byte[] aad = System.Text.Encoding.UTF8.GetBytes(vector.GetProperty("aad").GetString()!);
    using var uplink = new Session(node, gateway.PublicKey, salt, SessionRole.Initiator);
    using var downlink = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);

    foreach (JsonElement want in vector.GetProperty("messages").EnumerateArray())
    {
        string plaintext = want.GetProperty("plaintext").GetString()!;
        SealedMessage message =
            uplink.Seal(System.Text.Encoding.UTF8.GetBytes(plaintext), aad);

        Assert(message.Counter == want.GetProperty("counter").GetUInt64(), "the counter");
        Assert(
            Convert.ToHexString(message.Tag).ToLowerInvariant()
                == want.GetProperty("tag").GetString(),
            "the tag");
        Assert(
            Convert.ToHexString(message.Ciphertext).ToLowerInvariant()
                == want.GetProperty("ciphertext").GetString(),
            "the same key and counter produce the same bytes everywhere");
        Assert(
            System.Text.Encoding.UTF8.GetString(downlink.Open(message, aad)) == plaintext,
            "the peer recovers the reading");
    }

    JsonElement first = vector.GetProperty("messages")[0];
    var replayed = new SealedMessage(
        first.GetProperty("counter").GetUInt64(),
        Convert.FromHexString(first.GetProperty("tag").GetString()!),
        Convert.FromHexString(first.GetProperty("ciphertext").GetString()!));

    try
    {
        downlink.Open(replayed, aad);
        Fail("a repeated counter must be refused");
    }
    catch (PamojaException)
    {
    }

    using var fresh = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);
    try
    {
        fresh.Open(
            replayed,
            System.Text.Encoding.UTF8.GetBytes(vector.GetProperty("wrongAad").GetString()!));
        Fail("associated data that does not match must fail authentication");
    }
    catch (PamojaException)
    {
    }

    JsonElement hmac = vector.GetProperty("hmac");
    Assert(
        Convert.ToHexString(Session.HmacSha256(
            System.Text.Encoding.UTF8.GetBytes(hmac.GetProperty("key").GetString()!),
            System.Text.Encoding.UTF8.GetBytes(hmac.GetProperty("message").GetString()!)))
            .ToLowerInvariant() == hmac.GetProperty("digest").GetString(),
        "the keyed hash");

    JsonElement hkdf = vector.GetProperty("hkdf");
    Assert(
        Convert.ToHexString(Session.HkdfSha256(
            System.Text.Encoding.UTF8.GetBytes(hkdf.GetProperty("salt").GetString()!),
            System.Text.Encoding.UTF8.GetBytes(hkdf.GetProperty("ikm").GetString()!),
            System.Text.Encoding.UTF8.GetBytes(hkdf.GetProperty("info").GetString()!),
            hkdf.GetProperty("length").GetInt32()))
            .ToLowerInvariant() == hkdf.GetProperty("output").GetString(),
        "the expansion");
}

static void ConformUpdate(JsonElement vector)
{
    using var publisher = new DeviceIdentity(
        Convert.FromHexString(vector.GetProperty("publisherSeed").GetString()!));
    Assert(
        Convert.ToHexString(publisher.PublicKey).ToLowerInvariant()
            == vector.GetProperty("publisherPublicKey").GetString(),
        "the key a device trusts");

    JsonElement want = vector.GetProperty("manifest");
    byte[] vendor = Convert.FromHexString(vector.GetProperty("vendorId").GetString()!);
    byte[] deviceClass = Convert.FromHexString(vector.GetProperty("classId").GetString()!);
    var manifest = new Manifest(
        Sequence: want.GetProperty("sequence").GetUInt64(),
        VendorId: vendor,
        ClassId: deviceClass,
        Storage: want.GetProperty("storage").GetByte(),
        Digest: Convert.FromHexString(want.GetProperty("digest").GetString()!),
        Size: want.GetProperty("size").GetUInt32(),
        Expires: want.GetProperty("expires").GetUInt64(),
        Format: want.GetProperty("format").GetByte(),
        StructureVersion: want.GetProperty("structureVersion").GetByte());

    byte[] image = new byte[vector.GetProperty("imageLen").GetInt32()];
    Array.Fill(image, vector.GetProperty("imageByte").GetByte());

    Assert(
        Convert.ToHexString(Update.EncodeManifest(manifest)).ToLowerInvariant()
            == vector.GetProperty("body").GetString(),
        "a manifest encodes the same in every language");

    byte[] envelope = Update.SignManifest(manifest, publisher);
    Assert(
        Convert.ToHexString(envelope).ToLowerInvariant()
            == vector.GetProperty("envelope").GetString(),
        "the signed envelope");
    Assert(
        Convert.ToHexString(Update.VerifyEnvelope(envelope, publisher.PublicKey).Digest)
            .ToLowerInvariant() == want.GetProperty("digest").GetString(),
        "which verifies against the key that signed it");

    try
    {
        Update.VerifyEnvelope(
            Convert.FromHexString(vector.GetProperty("forgedEnvelope").GetString()!),
            publisher.PublicKey);
        Fail("a release signed by another key must be refused");
    }
    catch (PamojaException)
    {
    }

    JsonElement delegationWant = vector.GetProperty("delegation");
    using var anchor = new DeviceIdentity(
        Convert.FromHexString(vector.GetProperty("anchorSeed").GetString()!));
    Assert(
        Convert.ToHexString(Update.SignDelegation(
            new Delegation(
                delegationWant.GetProperty("epoch").GetUInt64(),
                Convert.FromHexString(delegationWant.GetProperty("releaseKey").GetString()!),
                delegationWant.GetProperty("expires").GetUInt64()),
            anchor)).ToLowerInvariant() == delegationWant.GetProperty("envelope").GetString(),
        "the signed delegation");

    JsonElement life = vector.GetProperty("lifecycle");
    using var fleet = new Updater(vendor, deviceClass, publisher.PublicKey, 2, 4096);
    fleet.Provision(0, 1);
    Assert(
        fleet.Begin(envelope) == life.GetProperty("staged").GetByte(),
        "the release names the same slot");

    int chunk = life.GetProperty("chunk").GetInt32();
    for (int at = 0; at < image.Length; at += chunk)
    {
        fleet.Write(image.AsSpan(at, Math.Min(chunk, image.Length - at)));
    }

    Assert(
        fleet.Finish() == life.GetProperty("staged").GetByte(),
        "and the image matched what was promised");

    BootDecision boot = fleet.OnBoot();
    Assert(boot.Action.ToString() == life.GetProperty("boot").GetString(), "the boot decision");
    Assert(boot.Slot == life.GetProperty("bootSlot").GetByte(), "the slot it is about");
    Assert(
        fleet.Confirm() == life.GetProperty("confirmed").GetByte(),
        "the confirmed slot");

    SlotRecord record = fleet.Record(life.GetProperty("confirmed").GetByte());
    Assert(record.State.ToString() == life.GetProperty("state").GetString(), "the slot state");
    Assert(record.Written == life.GetProperty("written").GetUInt32(), "the bytes written");
}

static void ConformPower(JsonElement vector)
{
    JsonElement want = vector.GetProperty("plan");
    PowerPlan plan = PowerPlan.Create(
        want.GetProperty("activeUs").GetUInt64(),
        want.GetProperty("saverUs").GetUInt64(),
        want.GetProperty("criticalUs").GetUInt64());

    Close(plan.SaverBelow, want.GetProperty("saverBelow").GetSingle(), 1e-6, "the saver bar");
    Close(
        plan.CriticalBelow,
        want.GetProperty("criticalBelow").GetSingle(),
        1e-6,
        "the critical bar");

    JsonElement charges = vector.GetProperty("charges");
    JsonElement modes = vector.GetProperty("modes");
    JsonElement charging = vector.GetProperty("charging");
    JsonElement intervals = vector.GetProperty("intervalsUs");
    for (int at = 0; at < charges.GetArrayLength(); at++)
    {
        float soc = charges[at].GetSingle();
        Assert(plan.Mode(soc).ToString() == modes[at].GetString(), $"the mode at {soc}");
        Assert(
            plan.ModeWhileCharging(soc, true).ToString() == charging[at].GetString(),
            $"the mode while charging at {soc}");
        Assert(
            plan.IntervalUs(soc) == intervals[at].GetUInt64(),
            $"the interval at {soc}");
    }

    JsonElement dutyWant = vector.GetProperty("duty");
    DutyCycle duty = DutyCycle.FromFraction(
        dutyWant.GetProperty("periodUs").GetUInt64(),
        dutyWant.GetProperty("fraction").GetSingle());
    Assert(duty.ActiveUs == dutyWant.GetProperty("activeUs").GetUInt64(), "the time awake");
    Assert(duty.SleepUs == dutyWant.GetProperty("sleepUs").GetUInt64(), "the time asleep");
}

static void ConformTelemetry(JsonElement vector)
{
    JsonElement costs = vector.GetProperty("costs");
    JsonElement thresholds = vector.GetProperty("thresholds");
    for (int at = 0; at < costs.GetArrayLength(); at++)
    {
        LinkCost cost = Enum.Parse<LinkCost>(costs[at].GetString()!);
        Assert(
            Reporter.ThresholdFor(cost).ToString() == thresholds[at].GetString(),
            $"the bar {cost} sets");
    }

    using var reporter = new Reporter(TelemetryLevel.Trace);
    reporter.AdaptTo(Enum.Parse<LinkCost>(vector.GetProperty("adaptedTo").GetString()!));

    JsonElement levels = vector.GetProperty("levels");
    JsonElement shipped = vector.GetProperty("shipped");
    for (int at = 0; at < levels.GetArrayLength(); at++)
    {
        TelemetryLevel level = Enum.Parse<TelemetryLevel>(levels[at].GetString()!);
        TelemetryEvent? outcome = reporter.Record(new TelemetryEvent(level, "vector"));
        Assert(
            (outcome is not null) == shipped[at].GetBoolean(),
            $"whether event {at} is worth its bytes");
    }

    JsonElement want = vector.GetProperty("snapshot");
    TelemetrySnapshot snapshot = reporter.Snapshot();
    Assert(snapshot.Trace == want.GetProperty("trace").GetUInt32(), "the trace count");
    Assert(snapshot.Debug == want.GetProperty("debug").GetUInt32(), "the debug count");
    Assert(snapshot.Info == want.GetProperty("info").GetUInt32(), "the info count");
    Assert(snapshot.Warn == want.GetProperty("warn").GetUInt32(), "the warn count");
    Assert(snapshot.Error == want.GetProperty("error").GetUInt32(), "the error count");
    Assert(snapshot.Emitted == want.GetProperty("emitted").GetUInt32(), "the shipped count");
    Assert(
        snapshot.Dropped == want.GetProperty("dropped").GetUInt32(),
        "what was dropped is still counted");
}

static async Task ConformLadder(JsonElement vector)
{
    string topic = vector.GetProperty("topic").GetString()!;
    using var broker = new LoopbackBroker();
    using LoopbackTransport listener = broker.Link();
    await listener.ConnectAsync();
    await listener.SubscribeAsync(topic);

    JsonElement offlineWant = vector.GetProperty("withNoRung");
    JsonElement deliveries = offlineWant.GetProperty("deliveries");
    using var offline = new Ladder(Store.Memory());

    int at = 0;
    foreach (JsonElement payload in vector.GetProperty("payloads").EnumerateArray())
    {
        Delivery delivery = await offline.SendAsync(
            topic, System.Text.Encoding.UTF8.GetBytes(payload.GetString()!));
        Assert(
            delivery.ToString() == deliveries[at].GetString(),
            "a message no rung takes is buffered rather than lost");
        at++;
    }

    Assert(
        await offline.BufferedAsync() == offlineWant.GetProperty("buffered").GetInt32(),
        "the buffer holds them");

    JsonElement restoredWant = vector.GetProperty("afterTheLinkReturns");
    offline.Rung(broker.Rung());
    await offline.ConnectAsync();
    Assert(
        await offline.FlushAsync() == restoredWant.GetProperty("flushed").GetInt32(),
        "the buffer replays once a link returns");
    Assert(
        await offline.BufferedAsync() == restoredWant.GetProperty("buffered").GetInt32(),
        "leaving it empty");

    JsonElement fallthrough = vector.GetProperty("fallthrough");
    using var rungs = new Ladder(Store.Memory());
    rungs.Rung(Transport.Faulty(
        broker.Rung(),
        fallthrough.GetProperty("failuresOnFirstRung").GetInt32()));
    rungs.Rung(broker.Rung());
    await rungs.ConnectAsync();
    Delivery fell = await rungs.SendAsync(
        topic,
        System.Text.Encoding.UTF8.GetBytes(fallthrough.GetProperty("payload").GetString()!));
    Assert(
        fell.ToString() == fallthrough.GetProperty("delivery").GetString(),
        "a rung that refuses falls through to the next");
}

static async Task ConformSimulation(JsonElement vector)
{
    JsonElement want = vector.GetProperty("sensor");
    using var sensor = new SimulatedSensor(
        want.GetProperty("baseline").GetSingle(),
        want.GetProperty("driftPerRead").GetSingle(),
        want.GetProperty("noise").GetSingle(),
        want.GetProperty("seed").GetUInt32());
    foreach (JsonElement reading in want.GetProperty("readings").EnumerateArray())
    {
        Assert(
            await sensor.ReadAsync() == reading.GetSingle(),
            "a seeded sensor invents the same run everywhere");
    }

    want = vector.GetProperty("replay");
    float[] capture = want.GetProperty("capture")
        .EnumerateArray()
        .Select(value => value.GetSingle())
        .ToArray();
    using var replay = new Replay(capture, want.GetProperty("repeating").GetBoolean());
    foreach (JsonElement reading in want.GetProperty("readings").EnumerateArray())
    {
        Assert(
            await replay.ReadAsync() == reading.GetSingle(),
            "a capture reads back the same");
    }

    want = vector.GetProperty("robot");
    using var robot = new SimulatedRobot(want.GetProperty("dt").GetSingle());
    var twist = new Twist(
        want.GetProperty("vx").GetSingle(),
        0.0f,
        want.GetProperty("omega").GetSingle());
    foreach (JsonElement pose in want.GetProperty("poses").EnumerateArray())
    {
        await robot.ApplyAsync(twist);
        Close(robot.Pose.X, pose.GetProperty("x").GetSingle(), 1e-6, "the x it reached");
        Close(robot.Pose.Y, pose.GetProperty("y").GetSingle(), 1e-6, "the y it reached");
        Close(
            robot.Pose.Theta,
            pose.GetProperty("theta").GetSingle(),
            1e-6,
            "the heading it holds");
    }
}
