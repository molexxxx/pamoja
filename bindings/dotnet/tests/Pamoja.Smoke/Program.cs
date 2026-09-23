// Smoke test: confirms the facade loads, the native core is reachable, and each
// capability behaves through it (no broker or hardware required).
using System.Text;
using System.Text.Json;

using Pamoja;
using Pamoja.Core;
using Pamoja.Security;
using Pamoja.Codec;
using Pamoja.Kit;
using Pamoja.Serial;
using Pamoja.Modbus;
using Pamoja.Can;
using Pamoja.Gpio;
using Pamoja.Hal;
using Pamoja.Sensors;
using Pamoja.Actuators;
using Pamoja.Lora;
using Pamoja.Lorawan;
using Pamoja.Radios;
using Pamoja.Gateway;
using Pamoja.Mesh;
using Pamoja.Routing;
using Pamoja.Mavlink;
using Pamoja.Audit;
using Pamoja.Session;
using Pamoja.Update;
using Pamoja.Power;
using Pamoja.Telemetry;
using Pamoja.Mqtt;
using Pamoja.Coap;
using Pamoja.Loopback;
using Pamoja.Sync;
using Pamoja.Ladder;
using Pamoja.Bus;
using Pamoja.Sim;
using Pamoja.Profile;
using Pamoja.Ros2;
using Pamoja.Zenoh;
using Pamoja.Native.Interop;

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
LaterSensors();
Buses();
SensorDrivers();
ActuatorDrivers();
StepperDrivers();
RadioAndReach();
Gateways();
GatewayNetworks();
RelayedReach();
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
    Audit.VerifyChain(keeper.PublicKey, [opened, shut]);

    byte[] edited = shut.ToBytes();
    edited[^1] ^= 0xFF;
    using AuditEntry tampered = AuditEntry.FromBytes(edited);
    Refuses(
        () => Audit.VerifyChain(keeper.PublicKey, [opened, tampered]),
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
    Assert(delegated.CurrentDelegation is not null, "the device now honors it");
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

    // A subscription placed on the ladder reaches its rungs, and a command published
    // upstream comes back through the ladder.
    await rungs.SubscribeAsync("commands/1");
    using var upstream = broker.Link();
    await upstream.ConnectAsync();
    await upstream.SendAsync("commands/1", "open"u8.ToArray());
    TransportMessage? inbound = await rungs.ReceiveAsync();
    Assert(
        inbound is not null && inbound.Payload.AsSpan().SequenceEqual("open"u8),
        "the command came back through the ladder");

    // A link written in .NET is a rung like any other: what the ladder sends reaches
    // it, and what it delivers comes back through the ladder.
    var hostLink = new QueueLink();
    using var hosted = new Ladder(Store.Memory());
    hosted.Rung(Transport.FromHandlers(hostLink));
    await hosted.ConnectAsync();
    await hosted.SubscribeAsync("commands/1");
    Assert(
        await hosted.SendAsync("sensors/1", "21.5"u8.ToArray()) == Delivery.Sent,
        "the host link carried the reading");
    Assert(hostLink.Sent.Count == 1 && hostLink.Sent[0].Topic == "sensors/1", "and saw it");
    Assert(hostLink.Filters.Contains("commands/1"), "the subscription reached the host link");
    hostLink.Deliver(new TransportMessage("commands/1", "open"u8.ToArray()));
    TransportMessage? fromHost = await hosted.ReceiveAsync();
    Assert(
        fromHost is not null && fromHost.Payload.AsSpan().SequenceEqual("open"u8),
        "what the host link delivered came back through the ladder");

    // A send-only host link is an uplink: sends go out, nothing is listened on.
    var uplink = new SendOnlyLink();
    using var oneWay = new Ladder(Store.Memory());
    oneWay.Rung(Transport.FromHandlers(uplink));
    await oneWay.ConnectAsync();
    Assert(
        await oneWay.SendAsync("sensors/1", "21.6"u8.ToArray()) == Delivery.Sent,
        "an uplink carries sends");
    try
    {
        await oneWay.ReceiveAsync();
        Fail("a ladder with only an uplink has nothing to receive from");
    }
    catch (PamojaException)
    {
    }

    // A handler that throws reports its reason to the caller.
    using var failing = Transport.FromHandlers(new RefusingLink());
    await failing.ConnectAsync();
    try
    {
        await failing.SendAsync("sensors/1", "x"u8.ToArray());
        Fail("a refusing link must fail the send");
    }
    catch (PamojaException error)
    {
        Assert(error.Message.Contains("out of range"), $"the reason is the handler's: {error.Message}");
    }

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
        "one second at one meter a second puts it a meter ahead");
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

    using var dry = Trigger.Below(30.0f, 5.0f);
    Assert(dry.Update(42.0f) is null, "above the line nothing fires");
    Assert(dry.Update(28.0f) == Edge.Set, "crossing it fires once");
    Assert(dry.IsSet, "and the trigger reports the condition holds");
    Assert(dry.Update(36.0f) == Edge.Cleared, "coming back past the band clears it");

    using var tank = new Depletion(10.0f);
    Assert(tank.Update(100.0f) is null, "the first reading sets no rate");
    Assert(tank.Update(90.0f) > 0, "a falling level projects a countdown");

    using var probe = Calibration.TwoPoint(0.0f, 0.0f, 1024.0f, 100.0f);
    Assert(Math.Abs(probe.Apply(512.0f) - 50.0f) < 0.01f, "a two-point fit maps its midpoint");

    Assert(Kit.Deadband(0.2f, 0.0f, 0.5f) == 0.0f, "noise inside the band does not act");

    var center = new Coordinate(-1.2921, 36.8219);
    var away = new Coordinate(-1.2930, 36.8219);
    using var pen = new Geofence(center, 50.0);
    Assert(pen.Update(center) == Boundary.Inside, "the first fix is inside");
    Assert(pen.Update(away) == Boundary.Exited, "the crossing fix reports once");
    Assert(pen.Update(away) == Boundary.Outside, "later fixes stay outside");
    Assert(!pen.Contains(away), "the fix is outside the fence");
    Assert(Kit.DistanceBetween(center, away) > 50.0, "the fix is beyond the radius");
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
        "an active-low relay is energized by a low level");

    var relay = Switch.ActiveLow(new PinScript());
    Assert(!relay.IsAsserted, "a switch starts off and drives nothing");
    relay.Set(true);
    relay.Set(false);
    Assert(
        relay.Release().Driven.SequenceEqual(new[] { PinLevel.Low, PinLevel.High }),
        "an active-low switch drives low to turn on and high to turn off");
    var button = Contact.ActiveLow(new PinScript(PinLevel.High, PinLevel.Low));
    Assert(
        !button.IsAsserted() && button.IsAsserted(),
        "an active-low contact reads closed on a low level");
    var idle = new PinScript();
    Assert(idle.Read() == PinLevel.High, "a script starts released and high");
    idle.Drive(PinLevel.Low);
    Assert(idle.Read() == PinLevel.Low, "past its reads a script answers its driven level");

    // A line opens on Linux alone, and a missing chip is refused naming the chip and line.
    try
    {
        using GpioLine absent = GpioLine.OpenInput("/dev/gpiochip-pamoja-absent", 27);
        Assert(false, "a missing chip does not open");
    }
    catch (PlatformNotSupportedException refused)
    {
        Assert(!OperatingSystem.IsLinux(), "only a platform that is not Linux refuses outright");
        Assert(refused.Message.Contains("only Linux"), "the refusal says where lines open");
    }
    catch (PamojaException refused)
    {
        Assert(OperatingSystem.IsLinux(), "only Linux gets as far as the chip");
        Assert(
            refused.Message.StartsWith("/dev/gpiochip-pamoja-absent line 27: "),
            "the error names the chip and the line");
    }
}

// The parts wired to a board: a compensated environment reading, a thermometer
// The seven parts added after the first four: a datasheet figure each, and the
// input each one is meant to refuse.
static void LaterSensors()
{
    PamojaSht3xMeasurement air = Sht3x.ParseMeasurement(Sht3x.MeasurementBytes(0x6666, 0x9999));
    Assert(air.MilliCelsius == 25_000, "0x6666 is two fifths of full scale");
    Assert(air.MilliPercent == 60_000, "and 0x9999 is three fifths");
    Assert(Sht3x.Crc([0xBE, 0xEF]) == 0x92, "Sensirion's check value");

    PamojaScd4xMeasurement measured = Scd4x.MeasurementFromPhysical(500, 25_000, 37_000);
    byte[] frame = Scd4x.MeasurementBytes(
        measured.Co2Ppm, measured.TemperatureRaw, measured.HumidityRaw);
    Assert(
        Scd4x.ParseMeasurement(frame).Co2Ppm == 500,
        "the carbon dioxide word survives the frame");

    byte[] corrupt = (byte[])frame.Clone();
    corrupt[2] ^= 0xFF;
    try
    {
        Scd4x.ParseMeasurement(corrupt);
        Fail("a flipped checksum byte should throw");
    }
    catch (PamojaException)
    {
    }

    Assert(Tmp117.MicroCelsius(0x0C80) == 25_000_000, "the datasheet's 25 C row");
    Assert(Tmp117.MicroCelsius(-1) == -7_812, "and one count below zero");

    Assert(Hdc1080.MilliCelsius(0x8000) == 42_500, "mid-scale on the HDC1080");
    try
    {
        Hdc1080.ConfigFromRegister(0x1300);
        Fail("the undefined humidity-resolution code should throw");
    }
    catch (PamojaException)
    {
    }

    Assert(Opt3001.MilliLux(0xBFFF) == 83_865_600, "the OPT3001 full scale");
    Assert(Opt3001.FullScaleMilliLux(12) is null, "a reserved range number has no full scale");

    Assert(Ina226.Calibration(1_000, 2) == 2_560, "the INA226 design example");
    Assert(
        Ina226.PowerMicrowatts(4_792, 1_000) == 119_800_000,
        "which reads 119.8 W at the example's load");
    try
    {
        Ina226.Identify(0x5449, 0x2270);
        Fail("a die that is not an INA226 should throw");
    }
    catch (PamojaException)
    {
    }

    Assert(Ina226.ConfigToRegister(Ina226Config.PowerOn) == 0x4127, "the named settings spell the power-on register");
    Assert(
        Ina226Config.From(Ina226.ConfigFromRegister(0x4527)).Averaging == Ina226.Averaging.Samples16,
        "and read back by name");

    using var coefficients = new Bmp280Calibration(
        Convert.FromHexString("706b436718fc7d8e43d6d00b270b8c00f9ff8c3cf8c67017"));
    Bmp280Reading reading = coefficients.Compensate(Convert.FromHexString("655ac07eed00"));
    Assert(
        reading.Pascals is > 90_000 and < 110_000,
        "the BMP280 reads a sane pressure");
}

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

    Assert(Pwm.FullOff()[3] == 0x10, "fully off is its own flag in LEDn_OFF_H");
    Assert(Pwm.Duty(0).SequenceEqual(Pwm.FullOff()), "the datasheet rules out the same count in on and off");
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
    Assert(fridge.Description?.Contains("safe range") == true, "a preset says what it is for");
    Assert(
        fridge.Presentation?.Elements.Any(element => element.Key == "fridge_temp") == true,
        "and says how it should be drawn");

    // A presentation is typed on the way in and on the way out, and travels in the manifest.
    using var described = fridge.WithDescription("Holds the clinic fridge at 5 C.");
    using var drawn = described.WithPresentation(new Presentation(
    [
        new ElementSpec("door_open", "state", "Door", Viz.Switch)
        {
            Labels = new Dictionary<string, string> { ["sw"] = "Mlango" },
            State = "state.closed",
        },
        new ElementSpec("compressor_amps", "amps", "Compressor current", Viz.Dial)
        {
            Band = [0.5f, 3.0f],
            Scope = ["mesh"],
        },
    ])
    {
        Theme = new Theme { Accent = "#3fb1c8" },
        Messages = new Dictionary<string, LocalizedText>
        {
            ["event.door_ajar"] = new Dictionary<string, string>
            {
                ["en"] = "Door left open",
                ["sw"] = "Mlango umeachwa wazi",
            },
            ["state.priming"] = "Priming",
        },
    });
    Assert(drawn.Description == "Holds the clinic fridge at 5 C.", "the description travels");
    using var shown = Profile.FromJson(drawn.ToJson());
    Presentation? presentation = shown.Presentation;
    Assert(presentation?.Elements.Count == 2, "both elements survive the manifest");
    Assert(presentation!.Elements[0].Viz == Viz.Switch, "the graphic comes back typed");
    Assert(presentation.Elements[0].Labels?["sw"] == "Mlango", "and so does a locale label");
    Assert(presentation.Elements[0].Scope is null, "an unscoped element is offered everywhere");
    Assert(presentation.Elements[1].Band is [0.5f, 3.0f], "a band is two numbers");
    Assert(presentation.Elements[1].Scope is ["mesh"], "a scoped element names its links");
    Assert(presentation.Theme?.Accent == "#3fb1c8", "the theme survives");
    Assert(
        presentation.Messages?["event.door_ajar"].PerLocale?["sw"] == "Mlango umeachwa wazi",
        "a per-locale message survives");
    Assert(presentation.Messages?["state.priming"].Text == "Priming", "and so does a plain one");
    Assert(drawn.ToJson().Contains("\"viz\": \"switch\""), "the manifest names the graphic as a manifest does");
    try
    {
        using var refused = fridge.WithPresentation(Presentation.FromJson("{ \"elements\": 3 }"));
        Assert(false, "a malformed presentation should be refused");
    }
    catch (JsonException)
    {
    }

    // A kind the library never shipped loads with its parameters beside it.
    using var orchard = Profile.FromJson("""
        {
            "name": "orchard-frost",
            "topic": "orchard/air/temperature",
            "control": { "kind": "frost_guard", "warn_below": 2.0, "latching": true, "zone": "north" },
            "power": { "active_secs": 60, "saver_secs": 300, "critical_secs": 900 }
        }
        """);
    ControlPolicy custom = orchard.Control;
    Assert(custom.Kind == ControlKind.Custom, "an unknown kind is a custom policy");
    Assert(custom.CustomKind == "frost_guard", "named as the manifest names it");
    Assert(custom.Params?["warn_below"] is double warnBelow && Math.Abs(warnBelow - 2.0) < 1e-9, "with its numbers");
    Assert(custom.Params?["latching"] is true, "its flags");
    Assert(custom.Params?["zone"] is "north", "and its text");
    Assert(fridge.Control.Params is null, "a built-in kind carries no parameter object");
    using (Controller inert = orchard.Controller())
    {
        Assert(inert.Evaluate(-4.0f).Actuator is null, "the built-in controller for a custom kind observes only");
    }

    Assert(orchard.ToJson().Contains("\"kind\": \"frost_guard\""), "and it writes back under its own name");

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

    JsonElement customVector = vector.GetProperty("custom");
    using var orchard = Profile.FromJson(customVector.GetProperty("manifest").GetString()!);
    Assert(orchard.Name == customVector.GetProperty("name").GetString(), "a custom kind's profile name");
    AssertControl(orchard.Control, customVector.GetProperty("control"), tolerance);
    using (Controller inert = orchard.Controller())
    {
        AssertReactions(inert, customVector.GetProperty("reactions"), tolerance);
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
            case "Custom":
                Assert(
                    reaction.Alert!.Value.Code == alert.GetProperty("code").GetString(),
                    $"the custom code at {reading}");
                Close(
                    reaction.Alert!.Value.Value ?? 0f,
                    (float)alert.GetProperty("value").GetDouble(),
                    tolerance,
                    "the custom value");
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
        case "Custom":
            Assert(policy.CustomKind == want.GetProperty("customKind").GetString(), "the custom kind");
            JsonElement wantParams = want.GetProperty("params");
            Assert(policy.Params is not null, "a custom kind carries its parameters");
            foreach (JsonProperty parameter in wantParams.EnumerateObject())
            {
                object got = policy.Params![parameter.Name];
                bool same = parameter.Value.ValueKind switch
                {
                    JsonValueKind.Number => got is double number
                        && Math.Abs(number - parameter.Value.GetDouble()) < tolerance,
                    JsonValueKind.True => got is true,
                    JsonValueKind.False => got is false,
                    _ => got is string text && text == parameter.Value.GetString(),
                };
                Assert(same, $"the parameter {parameter.Name}");
            }

            Assert(policy.Params!.Count == wantParams.EnumerateObject().Count(), "and no other");
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

// One I2C bus shared by the program and a driver: a simulated part read through the BME280
// driver, a script that refuses what it did not expect, and an adapter that opens on Linux
// alone.
static void Buses()
{
    const byte Address = Bme280.AddressPrimary;

    using I2cPart simulated = Bme280.Sim.Part(Address);
    using I2cBus bus = I2cBus.Simulated(simulated);
    Assert(bus.Kind == I2cBusKind.Simulated, "a bus of parts is simulated");
    using (var sensor = new Bme280(bus, Address))
    {
        Bme280Measurement reading = sensor.Measure();
        Assert(Math.Abs(reading.Celsius - 20.44f) < 0.005f, "the shipped part reads 20.44 C");
    }

    Assert(bus.Transfers == 11, "initializing and one measurement is eleven transfers");
    Assert(bus.WaitedMicros == 2_000 + 9_300, "the start-up and one measurement's wait");
    using (I2cPart held = bus.Part<I2cPart>(Address)!)
    {
        Assert(
            Bme280.CtrlMeasFromBits(held.Register(Bme280.Register.CtrlMeas)).Mode == Bme280.Mode.Forced,
            "the part keeps what the driver last wrote");
    }

    Assert(bus.Part(0x10) is null, "no part at an empty address");
    Assert(bus.Remaining is null, "only a script has steps left");

    using (I2cBus empty = I2cBus.Simulated())
    using (var absent = new Bme280(empty, Address))
    {
        try
        {
            absent.Init();
            Fail("an empty bus has nothing to initialize");
        }
        catch (PamojaException refused)
        {
            Assert(refused.Message == "nothing answered at 0x76", "the refusal names the address");
        }
    }

    using (I2cPart bmp280 = new I2cPart(Address).Holding(Bme280.Register.ChipId, [0x58]))
    using (I2cBus other = I2cBus.Simulated(bmp280))
    using (var wrong = new Bme280(other, Address))
    {
        Refuses(wrong.Init, "a part that is not a BME280 is refused");
    }

    using I2cBus script = I2cBus.Scripted(
        I2cStep.WriteRead(Address, [Bme280.Register.ChipId], [Bme280.ChipId]),
        I2cStep.Fault(Address, I2cFault.Bus));
    Assert(script.Remaining == 2, "a script starts with every step");
    Refuses(() => script.Write(Address, [0x00]), "a transfer the script does not expect is refused");
    Assert(script.WriteRead(Address, [Bme280.Register.ChipId], 1)[0] == Bme280.ChipId, "a matching transfer gets the reply");
    Refuses(() => script.Read(Address, 1), "a fault step fails the transfer");
    Assert(script.Remaining == 0, "and the script is spent");
    using (var extra = new I2cPart(Address))
    {
        Refuses(() => script.Attach(extra), "a script takes no parts");
    }

    try
    {
        using I2cBus missing = I2cBus.Open("/dev/i2c-pamoja-absent");
        Fail("a missing adapter does not open");
    }
    catch (PlatformNotSupportedException refused)
    {
        Assert(!OperatingSystem.IsLinux(), "only a platform that is not Linux refuses outright");
        Assert(refused.Message.Contains("only Linux"), "the refusal says where adapters open");
    }
    catch (PamojaException refused)
    {
        Assert(OperatingSystem.IsLinux(), "only Linux gets as far as the file");
        Assert(refused.Message.StartsWith("/dev/i2c-pamoja-absent: ", StringComparison.Ordinal), "the refusal names the file");
    }
}

// The PCA9685 driven over a simulated part that keeps its datasheet's rules: the prescale for
// 50 Hz only lands while the part sleeps, a servo channel reads back as loaded, one ALL_LED
// write reaches every channel, and a channel the part does not have is refused.
static void ActuatorDrivers()
{
    const byte Address = Pca9685.DefaultAddress;
    using (I2cPart fresh = Pca9685.Sim.Part(Address))
    {
        Assert(fresh.Register(Pca9685.Register.Mode1) == Pca9685.Mode1Reset, "MODE1 at power-up");
        Assert(fresh.Register(Pca9685.Register.PreScale) == Pca9685.PreScaleReset, "200 Hz at power-up");
    }

    using I2cPart part = Pca9685.Sim.Part(Address);
    using I2cBus bus = I2cBus.Simulated(part);
    using var board = new Pca9685(bus, Address, frequencyHz: 50);
    Assert(board.Prescale == 121, "round(25 MHz / 4096 / 50) - 1");
    byte[] center = Pwm.Servo(1_500);
    board.SetChannel(0, center);
    Assert(bus.WaitedMicros == Pca9685.OscillatorStartupMicros, "the oscillator's start-up");

    using (I2cPart held = bus.Part<I2cPart>(Address)!)
    {
        Assert(held.Register(Pca9685.Register.PreScale) == 121, "the prescale landed while asleep");
        Assert(held.Register(Pca9685.Register.Mode1) == Pca9685.Mode1.AutoIncrement, "awake, RESTART cleared");
        byte first = Pca9685.ChannelRegister(0);
        byte[] loaded = [held.Register(first), held.Register((byte)(first + 1)), held.Register((byte)(first + 2)), held.Register((byte)(first + 3))];
        Assert(loaded.SequenceEqual(center), "the servo channel reads back");
    }

    Assert(board.Channel(0).SequenceEqual(center), "the driver reads it back too");
    Refuses(() => board.Channel(16), "a channel the part does not have");

    board.SetAll(Pwm.FullOff());
    using (I2cPart held = bus.Part<I2cPart>(Address)!)
    {
        Assert(held.Register((byte)(Pca9685.ChannelRegister(15) + 3)) == 0x10, "every channel off");
    }

    Refuses(() => board.SetChannel(16, Pwm.FullOn()), "a channel the part does not have");
    Refuses(() => board.SoftwareReset(), "nothing on a simulated bus answers the general call");
}

// The stepper drivers walk the same coil pairs and pulse the same lines as the Rust
// drivers' own tests, with every wait counted rather than slept.
static void StepperDrivers()
{
    const PinLevel High = PinLevel.High;
    const PinLevel Low = PinLevel.Low;
    static (PinScript, PinScript, PinScript, PinScript) Lines() =>
        (new PinScript(), new PinScript(), new PinScript(), new PinScript());

    var delay = new DelayLog();
    using (var motor = new FourWire<PinScript>(Lines(), StepDrive.FullStep, stepMicros: 1_500, delay: delay))
    {
        motor.Steps(4);
        Assert(motor.Position == 4, "four full steps forward");
        Assert(motor.Drive == StepDrive.FullStep, "the drive pattern");
        var (a, b, c, d) = motor.Release();
        Assert(a.Driven.SequenceEqual([Low, Low, High, High]), "coil A walks the datasheet pairs");
        Assert(b.Driven.SequenceEqual([High, Low, Low, High]), "coil B walks the datasheet pairs");
        Assert(c.Driven.SequenceEqual([High, High, Low, Low]), "coil C walks the datasheet pairs");
        Assert(d.Driven.SequenceEqual([Low, High, High, Low]), "coil D walks the datasheet pairs");
        Assert(delay.WaitsMicros.SequenceEqual([1_500u, 1_500u, 1_500u, 1_500u]), "a wait after every step");
        Assert(delay.TotalMillis == 6, "six milliseconds of steps");
    }

    using (var wave = new FourWire<PinScript>(Lines(), StepDrive.Wave, delay: new DelayLog()))
    {
        wave.Steps(-2);
        Assert(wave.Position == -2, "two wave steps backward");
        wave.Idle();
        var (a, _, _, d) = wave.Release();
        Assert(a.Driven.SequenceEqual([Low, Low, Low]), "coil A stays off");
        Assert(d.Driven.SequenceEqual([High, Low, Low]), "wave drive backward starts at coil D");
    }

    var pulses = new DelayLog();
    var carriage = new StepDir<PinScript>(new PinScript(), new PinScript(), pulseMicros: 5, stepMicros: 1_000, delay: pulses);
    carriage.Steps(2);
    carriage.Steps(-1);
    Assert(carriage.Position == 1, "two forward and one back");
    var (step, direction) = carriage.Release();
    Assert(direction.Driven.SequenceEqual([High, High, Low]), "the direction is set before each pulse");
    Assert(step.Driven.SequenceEqual([High, Low, High, Low, High, Low]), "one pulse a step");
    Assert(pulses.WaitsMicros.Take(3).SequenceEqual([5u, 5u, 1_000u]), "pulse, pulse, then the step wait");

    var defaults = new StepDir<PinScript>(new PinScript(), new PinScript(), delay: new DelayLog());
    Assert(defaults.PulseMicros == Stepper.DefaultPulseMicros && Stepper.DefaultPulseMicros == 10, "the default pulse");
    Assert(defaults.StepMicros == Stepper.DefaultStepMicros && Stepper.DefaultStepMicros == 2_000, "the default step wait");

    using var stuck = new FourWire<Unplugged>((new(), new(), new(), new()), StepDrive.Wave, delay: new DelayLog());
    AssertThrows(() => stuck.Step(StepDirection.Forward), "a coil line that cannot be driven");
    Assert(stuck.Position == 0, "a step that could not be driven is not counted");
}

// Every I2C part's driver against its simulated twin, all on one bus at the addresses a
// board would give them, and the three kinds of simulated part the bus hands back.
static void SensorDrivers()
{
    const byte Tmp117At = Tmp117.AddressAdd0Vplus;
    const byte Ads1115At = Ads1115.AddressSda;
    const byte Opt3001At = Opt3001.AddressScl;
    const byte Ina219At = Ina219.BaseAddress + 1;
    const byte Ina226At = 0x45;

    SimulatedPart[] parts =
    [
        Bmp280.Sim.Reporting(Bmp280.AddressSecondary, -7.5f, 1003.0f),
        Tmp117.Sim.Part(Tmp117At),
        Opt3001.Sim.Reporting(Opt3001At, 1200.0f),
        Hdc1080.Sim.Reporting(-20.0f, 12.5f),
        Ina219.Sim.Part(Ina219At),
        Ina226.Sim.Reporting(Ina226At, 2, 20_000_000, 3_300_000, -10_000_000),
        Ads1115.Sim.Reporting(Ads1115At, Ads1115.Pga.Fsr4_096, 3.0f),
        Sht3x.Sim.Reporting(Sht3x.AddressA, 30.0f, 70.0f),
        Scd4x.Sim.Part(),
    ];
    using I2cBus bus = I2cBus.Simulated(parts);
    foreach (SimulatedPart part in parts)
    {
        part.Dispose();
    }

    using (var pressure = new Bmp280(bus, Bmp280.AddressSecondary))
    {
        Assert(pressure.Coefficients is null, "no trimming before initialization");
        Bmp280Reading reading = pressure.Measure();
        Assert(Math.Abs(reading.Celsius + 7.5f) < 0.01f, "the BMP280 reads what its twin reports");
        Assert(Math.Abs(reading.Hectopascals - 1003.0f) < 0.01f, "and the pressure");
        Assert(pressure.Coefficients is not null, "the trimming is read at initialization");
    }

    using (var thermometer = new Tmp117(bus, Tmp117At, Tmp117.Averaging.X8))
    {
        Assert(thermometer.SiliconRevision is null, "no revision before initialization");
        Assert(thermometer.Measure().Celsius == Tmp117.Sim.Celsius, "21.25 C");
        Assert(thermometer.SiliconRevision is not null, "the revision is read at initialization");
        thermometer.SetAlertLimits(30.0f, 10.0f);
        Assert(thermometer.Alerts() == new Tmp117Alerts(false, false), "no alerts");
    }

    using (var light = new Opt3001(bus, Opt3001At, Opt3001.ConversionTime.Ms100))
    {
        Assert(light.Measure().Lux == 1200.0f, "the OPT3001 reads 1200 lux");
        Assert(light.Configuration.LongConversion == 0, "at the short conversion");
    }

    using (var climate = new Hdc1080(bus))
    {
        PamojaHdc1080Measurement air = climate.Measure();
        Assert(Math.Abs(air.Celsius + 20.0f) < 0.003f, "the HDC1080 reads -20 C");
        Assert(Math.Abs(air.RelativeHumidity - 12.5f) < 0.002f, "and 12.5 %");
    }
    Refuses(() => new Hdc1080(bus, (Hdc1080.TemperatureResolution)12).Dispose(), "a resolution the part does not have is refused");

    using (var solar = new Ina219(bus, Ina219At))
    {
        Ina219Reading panel = solar.Measure();
        Assert(panel.BusMillivolts == Ina219.Sim.BusMillivolts, "12 V on the bus");
        Assert(Math.Abs(panel.CurrentMicroamps - Ina219.Sim.Microamps) < panel.CurrentLsbMicroamps, "half an amp through the shunt");
        Assert(solar.CurrentLsbMicroamps == Ina219.MinimumCurrentLsbMicroamps(3_200_000), "the finest step for 3.2 A");
    }

    using (var battery = new Ina226(bus, Ina226At, shuntMilliohms: 2, maxMicroamps: 20_000_000))
    {
        Assert(battery.Identity is null, "no identity before initialization");
        Assert(Math.Abs(battery.Measure().CurrentAmps + 10.0f) < 0.001f, "10 A flowing out of the battery");
        Assert(battery.Identity?.Device == 0x226, "an INA226 answered");
    }

    using (var adc = new Ads1115(bus, Ads1115At, gain: Ads1115.Pga.Fsr4_096))
    {
        Ads1115Sample probe = adc.Sample();
        Assert(Math.Abs(probe.Volts - 3.0f) < 0.001f, "the ADS1115 reads 3 V");
        Assert(probe.Gain == Ads1115.Pga.Fsr4_096, "at the range it was built for");
    }

    using (var humidity = new Sht3x(bus, Sht3x.AddressA, Sht3x.Repeatability.Low))
    {
        Assert(Math.Abs(humidity.Measure().Celsius - 30.0f) < 0.003f, "the SHT3x reads 30 C");
        Assert(humidity.LastStatus?.Bits == 0x8010, "the status after a reset");
        humidity.HeaterOn();
    }

    using (var co2 = new Scd4x(bus))
    {
        Assert(co2.Measure().Co2Ppm == Scd4x.Sim.Co2Ppm, "the SCD4x reads 800 ppm");
        Assert(co2.Serial == Scd4x.Sim.Serial, "and its serial");
        Assert(co2.DataReady(), "a result is always waiting");
    }

    using (CommandPart? sht = bus.Part<CommandPart>(Sht3x.AddressA))
    {
        Assert(sht is not null, "a command part comes back as one");
        Assert(sht!.Received[0].SequenceEqual(new byte[] { 0x30, 0xA2 }), "the reset went first");
    }
    using (WordPart? tmp = bus.Part<WordPart>(Tmp117At))
    {
        Assert(tmp?.Word(0x02) == unchecked((ushort)Tmp117.RawFromCelsius(30.0f)), "the high limit the driver wrote");
    }
    Assert(bus.Part<WordPart>(Bmp280.AddressSecondary) is null, "a byte part is not a word part");
    using (SimulatedPart? any = bus.Part(Bmp280.AddressSecondary))
    {
        Assert(any is I2cPart, "and comes back as a byte part");
    }

    using var words = new WordPart(0x48).Holding(0x01, 0x2000).ReadOnly(0x01, 0xF000);
    using var commands = new CommandPart(0x44).Answering([0xF3, 0x2D], [0x80, 0x10, 0xE1]);
    using I2cBus handmade = I2cBus.Simulated(words, commands);
    handmade.Write(0x48, [0x01, 0x00, 0x20]);
    Assert(handmade.WriteRead(0x48, [0x01], 2).SequenceEqual(new byte[] { 0x20, 0x20 }), "the flag the part keeps survives a write");
    handmade.Write(0x44, [0xF3, 0x2D]);
    Assert(handmade.Read(0x44, 3).SequenceEqual(new byte[] { 0x80, 0x10, 0xE1 }), "the command's reply");
    Refuses(() => handmade.Read(0x44, 3), "the reply was taken");

    byte[] scratchpad = Ds18b20.BuildScratchpad(21.5f, 12, 75, -10);
    string text = $"{Convert.ToHexString(scratchpad).ToLowerInvariant()} : crc=00 YES\n";
    string spaced = string.Join(' ', Enumerable.Range(0, scratchpad.Length).Select(index => scratchpad[index].ToString("x2")));
    Assert(Ds18b20.ParseW1Slave($"{spaced} : crc={scratchpad[8]:x2} YES\n").MicroCelsius == 21_500_000, "the kernel's text decodes");
    Refuses(() => Ds18b20.ParseW1Slave(text), "one run-together hex word is not the kernel's format");
    string rendered = Ds18b20.W1SlaveText(scratchpad);
    Assert(rendered.StartsWith($"{spaced} : crc={scratchpad[8]:x2} YES\n", StringComparison.Ordinal), "the kernel's first line");
    Assert(rendered.EndsWith(" t=21500\n", StringComparison.Ordinal), "and the millidegrees on the second");
    string devices = Path.Combine(Path.GetTempPath(), $"pamoja-dotnet-w1-{Environment.ProcessId}");
    string device = Path.Combine(devices, "28-000005e2fdc3");
    Directory.CreateDirectory(device);
    File.WriteAllText(Path.Combine(device, "w1_slave"), $"{spaced} : crc={scratchpad[8]:x2} YES\n");
    try
    {
        IReadOnlyList<Ds18b20Thermometer> found = Ds18b20Thermometer.Discover(devices);
        Assert(found.Count == 1, "one probe under the directory");
        Assert(found[0].Read().MicroCelsius == 21_500_000, "and it reads 21.5 C");
        Assert(found[0].Path.EndsWith("w1_slave", StringComparison.Ordinal), "from its w1_slave file");
        Assert(found[0].Serial == "000005e2fdc3", "named by the serial in its directory");
        using (Ds18b20Thermometer bare = Ds18b20Thermometer.At(Path.Combine(devices, "w1_slave")))
        {
            Assert(bare.Serial is null, "a file outside a probe's directory has no serial");
        }
        foreach (Ds18b20Thermometer probe in found)
        {
            probe.Dispose();
        }
    }
    finally
    {
        Directory.Delete(devices, recursive: true);
    }
    using Ds18b20Thermometer gone = Ds18b20Thermometer.At(Path.Combine(devices, "28-gone", "w1_slave"));
    Refuses(() => gone.Read(), "a missing file is refused");
}

static void Assert(bool condition, string message)
{
    if (!condition)
    {
        Fail(message);
    }
}

static void Refuses(Action call, string message)
{
    try
    {
        call();
    }
    catch (PamojaException)
    {
        return;
    }

    Fail(message);
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
    ConformPerturbation(vectors.GetProperty("modbus"));
    ConformCan(vectors.GetProperty("can"));
    ConformGpio(vectors.GetProperty("gpio"));
    ConformSensors(vectors.GetProperty("sensors"));
    ConformLaterSensors(vectors.GetProperty("sensors"));
    ConformActuators(vectors.GetProperty("actuators"));
    ConformWindows(vectors.GetProperty("windows"), tolerance);
    ConformLora(vectors.GetProperty("lora"));
    ConformLoraBudget(vectors.GetProperty("lora"));
    ConformLoraRegions(vectors.GetProperty("loraRegions"));
    ConformRadios(vectors.GetProperty("radios"), vectors.GetProperty("lora"));
    ConformSx127x(vectors.GetProperty("radios"), vectors.GetProperty("lora"));
    ConformGateway(vectors.GetProperty("gateway"));
    ConformGatewayNetwork(vectors.GetProperty("gatewayNetwork"));
    ConformStation(vectors.GetProperty("station"));
    ConformChirpstack(vectors.GetProperty("chirpstack"));
    ConformMavlink(vectors.GetProperty("mavlink"));
    ConformMavlinkSchema(vectors.GetProperty("mavlinkSchema"));
    ConformMavlinkProtocol(vectors.GetProperty("mavlinkProtocol"));
    ConformMesh(vectors.GetProperty("mesh"));
    ConformRouting(vectors.GetProperty("routing"));
    ConformLorawan(vectors.GetProperty("lorawan"));
    ConformHeader(vectors.GetProperty("header"));
    ConformLorawanLink(vectors.GetProperty("lorawanLink"), vectors);
    ConformLorawanDevice(vectors.GetProperty("lorawanDevice"));
    ConformLorawanRelay(vectors.GetProperty("lorawanRelay"));
    ConformLorawanPackages(vectors.GetProperty("lorawanPackages"));
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

    vector = vectors.GetProperty("trigger");
    using var trigger = Trigger.Below(
        vector.GetProperty("threshold").GetSingle(),
        vector.GetProperty("hysteresis").GetSingle());
    float[] crossings = Floats(vector, "readings");
    JsonElement[] edges = vector.GetProperty("outputs").EnumerateArray().ToArray();
    for (int i = 0; i < crossings.Length; i++)
    {
        Edge? edge = trigger.Update(crossings[i]);
        string? want = edges[i].ValueKind == JsonValueKind.Null ? null : edges[i].GetString();
        Assert(edge?.ToString().ToLowerInvariant() == want, "trigger edge");
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
    JsonElement center = vector.GetProperty("center");
    using var fence = new Geofence(
        new Coordinate(
            center.GetProperty("latitude").GetDouble(),
            center.GetProperty("longitude").GetDouble()),
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

// A suite that stopped comparing would pass every vector in the file, so this asserts the
// comparison itself: the committed frame matches what the library builds, and a frame with
// one bit moved does not. Every binding's runner carries the same case.
static void ConformPerturbation(JsonElement vector)
{
    JsonElement read = vector.GetProperty("readHoldingRegisters");
    byte[] built = Modbus.ReadHoldingRegisters(
        read.GetProperty("address").GetByte(),
        read.GetProperty("start").GetUInt16(),
        read.GetProperty("count").GetUInt16());

    byte[] committed = Convert.FromHexString(read.GetProperty("frame").GetString()!);
    Assert(built.SequenceEqual(committed), "the committed vector still matches");

    byte[] perturbed = (byte[])committed.Clone();
    perturbed[^1] ^= 0x01;
    Assert(
        !built.SequenceEqual(perturbed),
        "a vector with one bit moved must not compare equal");
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

// The parts added to pamoja-sensors after the first four, against the same vectors
// the Rust, Node, and Python runners assert.
static void ConformLaterSensors(JsonElement vector)
{
    JsonElement bmp = vector.GetProperty("bmp280");
    using var calibration = new Bmp280Calibration(
        Convert.FromHexString(bmp.GetProperty("calibration").GetString()!));
    Bmp280Reading reading = calibration.Compensate(
        Convert.FromHexString(bmp.GetProperty("measurement").GetString()!));
    Assert(
        Math.Abs(reading.Celsius - bmp.GetProperty("celsius").GetSingle()) < 1e-3f,
        "BMP280 temperature matches");
    Assert(reading.Pascals == bmp.GetProperty("pascals").GetUInt32(), "BMP280 pressure matches");
    Assert(
        Convert.ToHexString(calibration.ToBytes()).ToLowerInvariant()
            == bmp.GetProperty("calibrationRoundTrip").GetString(),
        "the BMP280 coefficients round-trip through their registers");

    JsonElement sht = vector.GetProperty("sht3x");
    PamojaSht3xMeasurement air = Sht3x.ParseMeasurement(
        Convert.FromHexString(sht.GetProperty("measurement").GetString()!));
    Assert(
        air.TemperatureRaw == sht.GetProperty("temperatureRaw").GetUInt16(),
        "SHT3x temperature register matches");
    Assert(
        air.MilliCelsius == sht.GetProperty("milliCelsius").GetInt32(),
        "SHT3x temperature matches");
    Assert(
        air.MilliFahrenheit == sht.GetProperty("milliFahrenheit").GetInt32(),
        "SHT3x temperature in Fahrenheit matches");
    Assert(
        air.MilliPercent == sht.GetProperty("milliPercent").GetUInt32(),
        "SHT3x humidity matches");
    Assert(
        Sht3x.Crc(Convert.FromHexString(sht.GetProperty("crcInput").GetString()!))
            == sht.GetProperty("crc").GetByte(),
        "SHT3x checksum matches");
    try
    {
        Sht3x.ParseMeasurement(
            Convert.FromHexString(sht.GetProperty("corruptMeasurement").GetString()!));
        Fail("a flipped bit must fail the SHT3x checksum");
    }
    catch (PamojaException)
    {
    }

    PamojaSht3xStatus status = Sht3x.ParseStatus(
        Convert.FromHexString(sht.GetProperty("status").GetString()!));
    Assert(status.Bits == sht.GetProperty("statusBits").GetUInt16(), "SHT3x status word matches");
    Assert(
        (status.AlertPending != 0) == sht.GetProperty("alertPending").GetBoolean(),
        "SHT3x alert flag matches");
    Assert(
        (status.HeaterOn != 0) == sht.GetProperty("heaterOn").GetBoolean(),
        "SHT3x heater flag matches");

    JsonElement scd = vector.GetProperty("scd4x");
    PamojaScd4xMeasurement gas = Scd4x.ParseMeasurement(
        Convert.FromHexString(scd.GetProperty("measurement").GetString()!));
    Assert(gas.Co2Ppm == scd.GetProperty("co2Ppm").GetUInt16(), "SCD4x carbon dioxide matches");
    Assert(
        gas.MilliCelsius == scd.GetProperty("milliCelsius").GetInt32(),
        "SCD4x temperature matches");
    Assert(
        gas.HumidityMilliPercent == scd.GetProperty("humidityMilliPercent").GetUInt32(),
        "SCD4x humidity matches");
    try
    {
        Scd4x.ParseMeasurement(
            Convert.FromHexString(scd.GetProperty("corruptMeasurement").GetString()!));
        Fail("a flipped byte must fail the SCD4x checksum");
    }
    catch (PamojaException)
    {
    }

    // The offset scales by 2^16 where the measurement scales by 2^16 - 1, in the same
    // datasheet, so it is pinned separately.
    Assert(
        Scd4x.TemperatureOffsetWord(scd.GetProperty("temperatureOffsetMilliCelsius").GetUInt32())
            == scd.GetProperty("temperatureOffsetWord").GetUInt16(),
        "the SCD4x temperature offset matches");
    Assert(
        Scd4x.SerialNumber(Convert.FromHexString(scd.GetProperty("serialFrame").GetString()!))
            == scd.GetProperty("serialNumber").GetUInt64(),
        "SCD4x serial number matches");

    JsonElement tmp = vector.GetProperty("tmp117");
    foreach (JsonElement entry in tmp.GetProperty("readings").EnumerateArray())
    {
        short raw = unchecked((short)entry.GetProperty("register").GetUInt16());
        Assert(
            Tmp117.MicroCelsius(raw) == entry.GetProperty("microCelsius").GetInt32(),
            "TMP117 temperature matches");
        Assert(
            Tmp117.NanoCelsius(raw) == entry.GetProperty("nanoCelsius").GetInt64(),
            "TMP117 temperature in nanodegrees matches");
    }

    ushort flags = tmp.GetProperty("flagConfig").GetUInt16();
    Assert(
        Tmp117.HighAlert(flags) == tmp.GetProperty("highAlert").GetBoolean(),
        "TMP117 high alert matches");
    Assert(
        Tmp117.DataReady(flags) == tmp.GetProperty("dataReady").GetBoolean(),
        "TMP117 data ready matches");

    JsonElement hdc = vector.GetProperty("hdc1080");
    PamojaHdc1080Measurement room = Hdc1080.ParseMeasurement(
        Convert.FromHexString(hdc.GetProperty("measurement").GetString()!));
    Assert(
        room.TemperatureRaw == hdc.GetProperty("temperatureRaw").GetUInt16(),
        "HDC1080 temperature register matches");
    Assert(
        room.MilliCelsius == hdc.GetProperty("milliCelsius").GetInt32(),
        "HDC1080 temperature matches");
    Assert(
        room.MilliPercent == hdc.GetProperty("milliPercent").GetUInt32(),
        "HDC1080 humidity matches");
    try
    {
        Hdc1080.ConfigFromRegister(hdc.GetProperty("invalidConfiguration").GetUInt16());
        Fail("the undefined humidity-resolution code must be refused");
    }
    catch (PamojaException)
    {
    }

    JsonElement opt = vector.GetProperty("opt3001");
    foreach (JsonElement entry in opt.GetProperty("results").EnumerateArray())
    {
        Assert(
            Opt3001.MilliLux(entry.GetProperty("register").GetUInt16())
                == entry.GetProperty("milliLux").GetUInt32(),
            "OPT3001 illuminance matches");
    }

    foreach (JsonElement entry in opt.GetProperty("ranges").EnumerateArray())
    {
        Assert(
            Opt3001.FullScaleMilliLux(entry.GetProperty("range").GetByte())
                == entry.GetProperty("fullScaleMilliLux").GetUInt32(),
            "OPT3001 full scale matches");
    }

    Assert(
        Opt3001.FullScaleMilliLux(opt.GetProperty("invalidRange").GetByte()) is null,
        "a reserved range number has no full scale");

    JsonElement ina = vector.GetProperty("ina226");
    uint lsb = ina.GetProperty("currentLsbMicroamps").GetUInt32();
    Assert(
        Ina226.Calibration(lsb, ina.GetProperty("shuntMilliohms").GetUInt32())
            == ina.GetProperty("calibration").GetUInt16(),
        "the INA226 calibration its datasheet works out matches");
    Assert(
        Ina226.ShuntNanovolts(ina.GetProperty("rawShunt").GetInt16())
            == ina.GetProperty("shuntNanovolts").GetInt32(),
        "INA226 shunt voltage matches");
    Assert(
        Ina226.BusMicrovolts(ina.GetProperty("rawBus").GetUInt16())
            == ina.GetProperty("busMicrovolts").GetUInt32(),
        "INA226 bus voltage matches");
    Assert(
        Ina226.CurrentMicroamps(ina.GetProperty("rawCurrent").GetInt16(), lsb)
            == ina.GetProperty("currentMicroamps").GetInt32(),
        "INA226 current matches");
    Assert(
        Ina226.PowerMicrowatts(ina.GetProperty("rawPower").GetUInt16(), lsb)
            == ina.GetProperty("powerMicrowatts").GetUInt32(),
        "INA226 power matches");
    try
    {
        Ina226.Identify(
            ina.GetProperty("manufacturerId").GetUInt16(),
            ina.GetProperty("badDieId").GetUInt16());
        Fail("a die that is not an INA226 must be refused");
    }
    catch (PamojaException)
    {
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
    foreach (JsonElement ctrl in bme.GetProperty("ctrlMeas").EnumerateArray())
    {
        var fields = new Bme280CtrlMeas(
            (Bme280.Oversampling)ctrl.GetProperty("temperature").GetByte(),
            (Bme280.Oversampling)ctrl.GetProperty("pressure").GetByte(),
            (Bme280.Mode)ctrl.GetProperty("mode").GetByte());
        byte bits = ctrl.GetProperty("bits").GetByte();
        Assert(Bme280.CtrlMeasBits(fields) == bits, "BME280 ctrl_meas bits match");
        Assert(Bme280.CtrlMeasFromBits(bits) == fields, "BME280 ctrl_meas fields match");
        Assert(
            Bme280.MaxMeasurementMicros(fields.Temperature, fields.Pressure, Bme280.Oversampling.X1)
                == ctrl.GetProperty("maxMeasurementMicros").GetUInt32(),
            "BME280 measurement time matches");
    }

    foreach (JsonElement hum in bme.GetProperty("ctrlHum").EnumerateArray())
    {
        var humidity = (Bme280.Oversampling)hum.GetProperty("humidity").GetByte();
        byte bits = hum.GetProperty("bits").GetByte();
        Assert(Bme280.CtrlHumBits(humidity) == bits, "BME280 ctrl_hum bits match");
        Assert(Bme280.CtrlHumFromBits(bits) == humidity, "BME280 humidity code matches");
    }

    foreach (JsonElement config in bme.GetProperty("config").EnumerateArray())
    {
        var fields = new Bme280Config(
            (Bme280.Standby)config.GetProperty("standby").GetByte(),
            (Bme280.Filter)config.GetProperty("filter").GetByte(),
            config.GetProperty("spi3Wire").GetBoolean());
        byte bits = config.GetProperty("bits").GetByte();
        Assert(Bme280.ConfigBits(fields) == bits, "BME280 config bits match");
        Assert(Bme280.ConfigFromBits(bits) == fields, "BME280 config fields match");
    }

    JsonElement simulated = bme.GetProperty("simulated");
    byte[] burst = Bme280.Sim.BurstFor(
        simulated.GetProperty("celsius").GetSingle(),
        simulated.GetProperty("hectopascals").GetSingle(),
        simulated.GetProperty("relativeHumidity").GetSingle());
    Assert(
        Convert.ToHexString(burst).ToLowerInvariant() == simulated.GetProperty("burst").GetString(),
        "BME280 simulated burst matches");

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
    JsonElement servo = pwm.GetProperty("servoCenter");
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
static void LoraRadios()
{
    // A radio is reached over spidev and the GPIO character device, so opening one says
    // either that this platform has neither or which device it could not open.
    var wiring = new LoraRadioWiring("/dev/spidev-pamoja-absent", "/dev/gpiochip0", 25);
    try
    {
        using LoraRadio radio = LoraRadio.OpenSx127x(wiring, new Sx127xBoard(Sx127xPaOutput.PaBoost));
        Fail("no radio is wired to the machine running these tests");
    }
    catch (PlatformNotSupportedException error)
    {
        Assert(error.Message.Contains("Linux"), "a radio opens only on Linux");
    }
    catch (PamojaException error)
    {
        Assert(
            error.Message.Contains("/dev/spidev-pamoja-absent"),
            "the failure names the device that could not be opened");
    }

    // Settings the chip has no value for are refused before any device is opened.
    try
    {
        LoraRadio.OpenSx126x(
            wiring with { BusyLine = 24 },
            new Sx126xBoard(Sx126xAmplifier.HighPower) { TcxoVolts = 1.9 });
        Fail("DIO3 cannot supply a TCXO with 1.9 V");
    }
    catch (ArgumentOutOfRangeException)
    {
    }
}

static void RadioAndReach()
{
    LoraRadios();

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
    MavlinkShapes();
    MavlinkProtocols();

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
    Assert(router.Forward(0x09).NextHop == 0x07, "a cheaper neighbor wins");
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

    var radioWhip = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
    Sx126xTxPower radioPower = Sx126x.TxPowerUnderCeiling(Sx126xAmplifier.HighPower, radioWhip, 16);
    Assert(radioPower.SettingDbm == 14, "a whip and pigtail leave 14 dBm under a 16 dBm ceiling");
    Assert(
        Sx126x.SetRfFrequency(868_100_000).AsSpan().SequenceEqual(new byte[] { 0x86, 0x36, 0x41, 0x99, 0x9A }),
        "SetRfFrequency carries the frequency word");
    Refuses(
        () => Sx126x.SetLoraModulationParams(new LoraLink(7, 203_125)),
        "the SX126x has no 203.125 kHz bandwidth");
    Assert(Sx126x.Status(0x2C).ChipMode == Sx126xChipMode.StandbyRc, "0x2C is RC standby");
    using var radioGuard = new RadioDutyCycle(10);
    ulong radioAirtime = radioGuard.Transmitted(0, new LoraLink(12, 125_000), 10);
    Assert(radioGuard.WaitMicros(0) == radioAirtime * 100, "a 1% limit owes a hundred airtimes from a frame's start");
    Assert(Sx126x.Llcc68Supports(new LoraLink(9, 125_000)), "an LLCC68 has SF9 at 125 kHz");
    Assert(!Sx126x.Llcc68Supports(new LoraLink(10, 125_000)), "but not SF10");
    Assert(Sx127x.FrequencyWord(868_100_000) == 0xD9_0666, "the SX1276 carrier word");
    Assert(Sx127x.LoraOpMode(Sx127xMode.Tx) == 0x8B, "TX mode on the LoRa page");
    Assert(
        Sx127x.TxPower(Sx127xPaOutput.PaBoost, 20).PaDac == Sx127x.PaDacHighPower,
        "+20 dBm on PA_BOOST needs the high power setting");
    Refuses(() => Sx127x.Modem(new LoraLink(5, 125_000), 868_100_000), "the SX1276 has no SF5");
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
    Assert(licensed.MaxEirpDbm(915_500_000) == 30, "and carries the power its license allows");
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

    LoraCn470Plan[] cn470 = Enum.GetValues<LoraCn470Plan>();
    JsonElement cn470Vectors = vector.GetProperty("cn470");
    Assert(cn470Vectors.GetArrayLength() == cn470.Length, "every CN470 plan is described");
    index = 0;
    foreach (JsonElement want in cn470Vectors.EnumerateArray())
    {
        Assert(
            want.GetProperty("code").GetString() == Cn470Name(cn470[index]),
            $"CN470 plan {index} is {cn470[index]}");
        using LoraChannelPlan plan = LoraChannelPlan.ForCn470(cn470[index]);
        ConformPlan(plan, want);
        index++;
    }

    using LoraPlanBuilder fixedBuilder = new LoraPlanBuilder("private-fixed")
        .DataRate(LoraDataRate.ForLora(10, 125_000, 980))
        .DataRate(LoraDataRate.ForLora(8, 500_000, 12_500))
        .MaxPayload(new LoraMaxPayload(19, 11), LoraPayloadTable.UplinkRepeater)
        .MaxPayload(new LoraMaxPayload(230, 222), LoraPayloadTable.UplinkRepeater)
        .MaxPayload(new LoraMaxPayload(19, 11), LoraPayloadTable.UplinkDirect)
        .MaxPayload(new LoraMaxPayload(230, 222), LoraPayloadTable.UplinkDirect)
        .ChannelBlock(new LoraChannelBlock(902_300_000, 200_000, 16, 0, 0))
        .ChannelBlock(new LoraChannelBlock(903_000_000, 1_600_000, 2, 1, 1))
        .ChannelBlock(new LoraChannelBlock(923_300_000, 600_000, 4, 1, 1), LoraChannelSet.Downlink)
        .SubBand(new LoraSubBand(902_000_000, 928_000_000, 1000, 30))
        .Power(30, 2, 10)
        .Rx(923_300_000, 1, 0)
        .Rx1Row([1])
        .Rx1Row([1])
        .Kind(LoraPlanKind.Fixed)
        .TxParamSetup(false)
        .MaskControls(
        [
            LoraMaskControl.OneGroup(0),
            LoraMaskControl.OneGroup(1),
            LoraMaskControl.Reserved(),
            LoraMaskControl.Reserved(),
            LoraMaskControl.Reserved(),
            LoraMaskControl.Reserved(),
            LoraMaskControl.AllChannels(true, 1),
            LoraMaskControl.AllChannels(false, 1),
        ]);
    try
    {
        fixedBuilder.MaskControls([LoraMaskControl.Reserved()]);
        Fail("a plan takes exactly eight mask controls");
    }
    catch (PamojaException)
    {
    }

    using LoraChannelPlan customFixed = fixedBuilder
        .JoinSequence(LoraJoinSequence.OctetPasses)
        .PowerReference(LoraPowerReference.Conducted, 6)
        .RelayChannel(new LoraRelayChannel(916_700_000, 918_300_000, 1))
        .Build();
    ConformPlan(customFixed, vector.GetProperty("customFixed"));
}

// The name a CN470-510 plan goes by in the vectors.
static string Cn470Name(LoraCn470Plan plan) => plan switch
{
    LoraCn470Plan.Antenna20MhzA => "antenna_20mhz_a",
    LoraCn470Plan.Antenna20MhzB => "antenna_20mhz_b",
    LoraCn470Plan.Antenna26MhzA => "antenna_26mhz_a",
    LoraCn470Plan.Antenna26MhzB => "antenna_26mhz_b",
    _ => "channels_96",
};

// Checks a channel block against the vector describing it.
static void ConformBlock(LoraChannelBlock block, JsonElement want, string where)
{
    Assert(
        block == new LoraChannelBlock(
            want.GetProperty("startHz").GetUInt32(),
            want.GetProperty("stepHz").GetUInt32(),
            want.GetProperty("count").GetUInt16(),
            want.GetProperty("minDataRate").GetByte(),
            want.GetProperty("maxDataRate").GetByte()),
        where);
}

// Holds a plan's channel rules to the answers every binding must give.
static void ConformRules(LoraChannelPlan plan, JsonElement want, string where)
{
    LoraPlanRules rules = plan.Rules();
    string kind = rules.Kind == LoraPlanKind.Fixed ? "fixed" : "dynamic";
    Assert(kind == want.GetProperty("kind").GetString(), $"the kind of {where}");
    string? channelList = rules.ChannelList switch
    {
        LoraChannelList.Mhz800 => "mhz800",
        LoraChannelList.Mhz900 => "mhz900",
        _ => null,
    };
    JsonElement wantList = want.GetProperty("channelList");
    Assert(
        channelList == (wantList.ValueKind == JsonValueKind.Null ? null : wantList.GetString()),
        $"the channel list of {where}");
    Assert(
        rules.TxParamSetup == want.GetProperty("txParamSetup").GetBoolean(),
        $"TXParamSetupReq on {where}");
    string sequence = rules.JoinSequence == LoraJoinSequence.OctetPasses ? "octet_passes" : "random";
    Assert(sequence == want.GetProperty("joinSequence").GetString(), $"the join sequence of {where}");
    string reference = rules.PowerReference == LoraPowerReference.Conducted ? "conducted" : "eirp";
    Assert(
        reference == want.GetProperty("powerReference").GetString(),
        $"the power reference of {where}");
    ConformOptionalByte(
        rules.GainAllowanceDb,
        want.GetProperty("gainAllowanceDb"),
        $"the gain allowance of {where}");

    byte value = 0;
    foreach (JsonElement control in want.GetProperty("maskControls").EnumerateArray())
    {
        LoraMaskControl? got = plan.MaskControl(value);
        if (got is null)
        {
            Fail($"ChMaskCntl {value} of {where} is missing");
            return;
        }

        string gotKind = got.Value.Kind switch
        {
            LoraMaskControlKind.Group => "group",
            LoraMaskControlKind.Banks => "banks",
            LoraMaskControlKind.PairedBanks => "paired_banks",
            LoraMaskControlKind.All => "all",
            _ => "reserved",
        };
        Assert(gotKind == control.GetProperty("kind").GetString(), $"ChMaskCntl {value} of {where}");
        ConformOptionalByte(got.Value.Group, control.GetProperty("group"), $"ChMaskCntl {value} group of {where}");
        JsonElement on = control.GetProperty("on");
        Assert(
            on.ValueKind == JsonValueKind.Null ? got.Value.On is null : got.Value.On == on.GetBoolean(),
            $"ChMaskCntl {value} on of {where}");
        ConformOptionalByte(
            got.Value.ThenGroup,
            control.GetProperty("thenGroup"),
            $"ChMaskCntl {value} then group of {where}");
        value++;
    }

    Assert(plan.MaskControl(8) is null, $"ChMaskCntl past 7 of {where}");

    JsonElement downlinkBlocks = want.GetProperty("downlinkChannelBlocks");
    IReadOnlyList<LoraChannelBlock> gotBlocks = plan.ChannelBlocks(LoraChannelSet.Downlink);
    Assert(
        rules.DownlinkChannelBlockCount == downlinkBlocks.GetArrayLength()
            && gotBlocks.Count == downlinkBlocks.GetArrayLength(),
        $"downlink channel blocks of {where}");
    int blockIndex = 0;
    int downlinkCount = 0;
    foreach (JsonElement block in downlinkBlocks.EnumerateArray())
    {
        ConformBlock(gotBlocks[blockIndex], block, $"downlink channel block {blockIndex} of {where}");
        downlinkCount += block.GetProperty("count").GetUInt16();
        blockIndex++;
    }

    ushort downlink = 0;
    foreach (JsonElement frequency in want.GetProperty("downlinkChannelFrequencies").EnumerateArray())
    {
        Assert(
            plan.DownlinkChannelFrequencyHz(downlink) == frequency.GetUInt32(),
            $"downlink channel {downlink} of {where}");
        downlink++;
    }

    ConformOptionalUint(
        plan.DownlinkChannelFrequencyHz((ushort)downlinkCount),
        want.GetProperty("downlinkChannelPastEnd"),
        $"a downlink channel past the end of {where}");

    foreach (JsonElement probe in want.GetProperty("rx1Frequencies").EnumerateArray())
    {
        ushort uplinkChannel = probe.GetProperty("uplinkChannel").GetUInt16();
        ConformOptionalUint(
            plan.Rx1FrequencyHz(uplinkChannel, probe.GetProperty("uplinkHz").GetUInt32()),
            probe.GetProperty("rx1Hz"),
            $"RX1 after uplink channel {uplinkChannel} of {where}");
    }

    JsonElement joinPlans = want.GetProperty("joinPlans");
    IReadOnlyList<LoraJoinPlan> runs = plan.JoinPlans();
    Assert(
        rules.JoinPlanCount == joinPlans.GetArrayLength() && runs.Count == joinPlans.GetArrayLength(),
        $"join plans of {where}");
    int run = 0;
    foreach (JsonElement entry in joinPlans.EnumerateArray())
    {
        string label = $"join plan {run} of {where}";
        ConformBlock(runs[run].Channels, entry.GetProperty("channels"), label);
        Assert(runs[run].AcceptStartHz == entry.GetProperty("acceptStartHz").GetUInt32(), label);
        Assert(runs[run].AcceptStepHz == entry.GetProperty("acceptStepHz").GetUInt32(), label);
        Assert(runs[run].Rx2StartHz == entry.GetProperty("rx2StartHz").GetUInt32(), label);
        Assert(runs[run].Rx2StepHz == entry.GetProperty("rx2StepHz").GetUInt32(), label);
        JsonElement selects = entry.GetProperty("plan");
        Assert(
            (runs[run].Plan is { } named ? Cn470Name(named) : null)
                == (selects.ValueKind == JsonValueKind.Null ? null : selects.GetString()),
            label);
        run++;
    }

    foreach (JsonElement entry in want.GetProperty("joinPlaces").EnumerateArray())
    {
        ushort joinChannel = entry.GetProperty("joinChannel").GetUInt16();
        LoraJoinPlanPlace? place = plan.JoinPlanForChannel(joinChannel);
        JsonElement wantPlace = entry.GetProperty("place");
        string label = $"the join plan holding join channel {joinChannel} of {where}";
        if (wantPlace.ValueKind == JsonValueKind.Null)
        {
            Assert(place is null, label);
            continue;
        }

        Assert(
            place == new LoraJoinPlanPlace(
                wantPlace.GetProperty("index").GetUInt16(),
                wantPlace.GetProperty("offset").GetUInt16(),
                wantPlace.GetProperty("acceptHz").GetUInt32(),
                wantPlace.GetProperty("rx2Hz").GetUInt32()),
            label);
    }
}

// Holds one channel plan to the answers every binding must give.
static void ConformPlan(LoraChannelPlan plan, JsonElement want)
{
    IReadOnlyList<LoraRelayChannel> relayChannels = plan.RelayChannels();
    JsonElement wantRelay = want.GetProperty("relayChannels");
    Assert(relayChannels.Count == wantRelay.GetArrayLength(), "relay channel count");
    int relayIndex = 0;
    foreach (JsonElement entry in wantRelay.EnumerateArray())
    {
        Assert(
            relayChannels[relayIndex] == new LoraRelayChannel(
                entry.GetProperty("worFrequencyHz").GetUInt32(),
                entry.GetProperty("ackFrequencyHz").GetUInt32(),
                entry.GetProperty("dataRate").GetByte()),
            $"relay channel {relayIndex}");
        relayIndex++;
    }

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

    ConformRules(plan, want.GetProperty("rules"), where);
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
// Message shapes: filling a message in by name, and describing one this build has
// never heard of so a vendor dialect needs no code here.
static void MavlinkShapes()
{
    using MavlinkSchema heartbeat = MavlinkSchema.ForName("HEARTBEAT");
    Assert(heartbeat.MessageId == 0, "HEARTBEAT's id");
    Assert(heartbeat.CrcExtra == Mavlink.KnownCrcExtra(0), "and its published seed");
    Assert(heartbeat.WireLength == 9, "and its length on the wire");
    Assert(
        heartbeat.Fields[0].Name == "custom_mode",
        "wire order puts the 32-bit field first");
    Assert(
        MavlinkSchema.KnownMessages().Contains("GLOBAL_POSITION_INT"),
        "the registry lists it");
    AssertThrows(() => MavlinkSchema.ForName("NOT_A_MESSAGE"), "an unknown name is refused");

    using MavlinkMessage built = heartbeat.CreateMessage();
    built.Set("type", 18); // MAV_TYPE_ONBOARD_CONTROLLER
    built.Set("system_status", 4); // MAV_STATE_ACTIVE
    built.Set("mavlink_version", 3);

    using MavlinkFrame sent = built.ToFrame(new MavlinkHeader(1, 1));
    Assert(sent.MessageId == 0, "the frame carries HEARTBEAT");

    using MavlinkMessage received = heartbeat.Decode(sent.Payload);
    Assert(received.Get("system_status") == 4, "the status survives the wire");
    Assert(received.GetInt64("type") == 18, "and so does the vehicle type");

    AssertThrows(() => received.Get("throttle"), "an unknown field is refused");
    AssertThrows(() => built.Set("type", 300), "a value past a uint8_t is refused");
    AssertThrows(() => built.Set("type", 1.5), "and so is a fractional one");

    // Text lives in a fixed-length char array, padded with zeros.
    using MavlinkSchema status = MavlinkSchema.ForName("STATUSTEXT");
    int textLength = status.Fields.First(field => field.Name == "text").ArrayLen;
    using MavlinkMessage spoken = status.CreateMessage();
    spoken.SetText("text", "preflight checks passed");
    Assert(
        spoken.GetText("text", textLength) == "preflight checks passed",
        "the text reads back");

    // A private message: described once, then carried and checked like any other.
    MavlinkSchemaBuilder builder = new(50_001, "BATTERY_CELLS");
    using MavlinkSchema cells = builder
        .Field("cell_mv", MavlinkFieldType.UInt16, 6)
        .Field("pack_id", MavlinkFieldType.UInt8)
        .Field("uptime_ms", MavlinkFieldType.UInt32)
        .Build();
    Assert(
        cells.Fields[0].Name == "uptime_ms",
        "the builder puts the widest field first");

    using MavlinkMessage pack = cells.CreateMessage();
    pack.Set("pack_id", 2);
    pack.Set("cell_mv", 4151, 2);

    using MavlinkDialect dialect = new();
    dialect.AddSchema(cells);
    using MavlinkFrame carried = pack.ToFrame(new MavlinkHeader(9, 1));
    using MavlinkFrame back = MavlinkFrame.ParseKnown(carried.Bytes, dialect);
    using MavlinkMessage read = cells.Decode(back.Payload);
    Assert(read.Get("cell_mv", 2) == 4151, "a private message survives the wire whole");
}


// The service protocols: a plan crosses from a station to a vehicle one frame at a time,
// a command is matched to its acknowledgment, and a setpoint goes out as the right message.
static void MavlinkProtocols()
{
    MavlinkHeader vehicle = new(1, 1);
    MavlinkHeader station = new(255, 190);

    // Two waypoints, described by field name; the sender numbers them itself.
    using MavlinkSchema itemShape = MavlinkSchema.ForName("MISSION_ITEM_INT");
    using MavlinkMissionSender upload = new(1, 1);
    using (MavlinkMessage takeoff = itemShape.CreateMessage())
    {
        takeoff.Set("command", 22);
        takeoff.Set("z", 20);
        upload.AddItem(takeoff);
    }

    using (MavlinkMessage waypoint = itemShape.CreateMessage())
    {
        waypoint.Set("command", 16);
        waypoint.Set("x", -338_567_800);
        waypoint.Set("y", 1_512_153_000);
        waypoint.Set("z", 50);
        upload.AddItem(waypoint);
    }

    Assert(upload.Count == 2, "the plan holds both items");

    // The station opens the download and the two sides take turns until it is acknowledged.
    using MavlinkMissionReceiver download = new(255, 190);
    using MavlinkFrame requestList = download.RequestList(station);
    MavlinkFrame fromVehicle = upload.OnFrame(requestList, vehicle)!.Value.Reply!;
    List<long> accepted = [];
    while (true)
    {
        MavlinkReceiverStep? step = download.OnFrame(fromVehicle, station);
        fromVehicle.Dispose();
        Assert(step is not null, "the vehicle only sends what the receiver handles");
        if (step!.Value.Accepted is not null)
        {
            accepted.Add(step.Value.Accepted.GetInt64("command"));
            step.Value.Accepted.Dispose();
        }

        MavlinkSenderStep answer = upload.OnFrame(step.Value.Reply, vehicle)!.Value;
        step.Value.Reply.Dispose();
        if (answer.Kind == MavlinkSenderKind.Finished)
        {
            Assert(answer.Result == 0, "the transfer is accepted");
            break;
        }

        fromVehicle = answer.Reply!;
    }

    Assert(accepted.SequenceEqual([22L, 16L]), "both items arrive in order");
    Assert(download.Complete, "and the download is complete");

    // A command is matched to its acknowledgment, and a stray frame is passed over.
    using MavlinkCommand arm = new(400);
    Assert(arm.Confirmation == 0, "the first send carries confirmation 0");
    using MavlinkSchema ackShape = MavlinkSchema.ForName("COMMAND_ACK");
    using MavlinkMessage ackMessage = ackShape.CreateMessage();
    ackMessage.Set("command", 400);
    using MavlinkFrame ack = ackMessage.ToFrame(vehicle);
    MavlinkAckOutcome outcome = arm.OnFrame(ack)!.Value;
    Assert(outcome.Kind == MavlinkAckKind.Final && outcome.Value == 0, "an accepted arm");
    Assert(arm.OnFrame(requestList) is null, "a mission frame is not an ack");
    Assert(arm.OnTimeout() == 1, "a timeout allows a resend with confirmation 1");

    // A setpoint goes out as the right message with the right mask.
    using MavlinkFrame setpoint = MavlinkOffboard.LocalVelocity(station, 1000, 1, 1, 1, 0.5f, 0f, 0f);
    Assert(setpoint.MessageId == 84, "SET_POSITION_TARGET_LOCAL_NED");
    using MavlinkSchema setpointShape = MavlinkSchema.ForName("SET_POSITION_TARGET_LOCAL_NED");
    using MavlinkMessage read = setpointShape.Decode(setpoint.Payload);
    Assert(
        read.GetInt64("type_mask") == MavlinkOffboard.TypeMask(MavlinkTypeMask.Velocity),
        "only the velocity fields are active");
}

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


// The message-shape layer: field order, offsets, and the seed they imply are what a
// peer checks against, so a binding that reorders a field fails here rather than
// against a vehicle.
static void ConformMavlinkSchema(JsonElement vector)
{
    foreach (JsonElement entry in vector.GetProperty("fieldTypes").EnumerateArray())
    {
        uint code = entry.GetProperty("code").GetUInt32();
        Assert(
            Enum.IsDefined(typeof(MavlinkFieldType), code),
            $"the code {code} for {entry.GetProperty("name").GetString()} is a field type");
    }

    Assert((uint)MavlinkFieldType.UInt32 == 6, "the uint32_t code");
    Assert((uint)MavlinkFieldType.Char == 3, "the char code");

    Assert(
        MavlinkSchema.KnownMessages().Count == vector.GetProperty("messageCount").GetInt32(),
        "how many messages this build types");
    AssertThrows(
        () => MavlinkSchema.ForId(vector.GetProperty("unknownMessage").GetProperty("msgid").GetUInt32()),
        "an id outside the typed set has no shape here");
    AssertThrows(
        () => MavlinkSchema.ForName(vector.GetProperty("unknownMessage").GetProperty("name").GetString()!),
        "a name outside the typed set has no shape here");

    foreach (JsonElement described in vector.GetProperty("shapes").EnumerateArray())
    {
        string name = described.GetProperty("name").GetString()!;
        using MavlinkSchema shape = MavlinkSchema.ForName(name);
        Assert(shape.MessageId == described.GetProperty("msgid").GetUInt32(), $"the id of {name}");
        Assert(
            shape.CrcExtra == described.GetProperty("crcExtra").GetByte(),
            $"the seed of {name}");
        Assert(
            shape.WireLength == described.GetProperty("wireLen").GetInt32(),
            $"the wire length of {name}");

        IReadOnlyList<MavlinkFieldInfo> fields = shape.Fields;
        JsonElement wanted = described.GetProperty("fields");
        Assert(fields.Count == wanted.GetArrayLength(), $"the field count of {name}");

        int index = 0;
        foreach (JsonElement want in wanted.EnumerateArray())
        {
            MavlinkFieldInfo field = fields[index];
            string where = $"{name}.{want.GetProperty("name").GetString()}";
            Assert(field.Name == want.GetProperty("name").GetString(), $"the field order of {where}");
            Assert(field.TypeName == want.GetProperty("typeName").GetString(), $"the type of {where}");
            Assert(
                (uint)field.FieldType == want.GetProperty("fieldType").GetUInt32(),
                $"the type code of {where}");
            Assert(
                field.ArrayLen == want.GetProperty("arrayLen").GetByte(),
                $"the array length of {where}");
            Assert(
                field.Extension == want.GetProperty("extension").GetBoolean(),
                $"whether {where} is an extension");
            Assert(field.Offset == want.GetProperty("offset").GetInt32(), $"the offset of {where}");
            index += 1;
        }
    }

    // A definition written in declaration order lands in wire order and on the published
    // seed, which is what lets a caller transcribe a dialect as it reads.
    foreach (string key in new[] { "declared", "private" })
    {
        JsonElement described = vector.GetProperty(key);
        MavlinkSchemaBuilder builder = new(
            described.GetProperty("msgid").GetUInt32(),
            described.GetProperty("name").GetString()!);
        foreach (JsonElement field in described.GetProperty("fields").EnumerateArray())
        {
            builder.Field(
                field.GetProperty("name").GetString()!,
                (MavlinkFieldType)field.GetProperty("fieldType").GetUInt32(),
                field.GetProperty("arrayLen").GetByte());
        }

        using MavlinkSchema built = builder.Build();
        List<string> order = [.. built.Fields.Select(field => field.Name)];
        List<string> expected =
            [.. described.GetProperty("wireOrder").EnumerateArray().Select(name => name.GetString()!)];
        Assert(order.SequenceEqual(expected), $"the wire order the builder produces for {key}");
        Assert(
            built.CrcExtra == described.GetProperty("crcExtra").GetByte(),
            $"the seed the builder derives for {key}");
    }

    // A message filled in by name puts exactly these bytes on the wire.
    JsonElement filled = vector.GetProperty("filled");
    using MavlinkSchema position = MavlinkSchema.ForName(filled.GetProperty("name").GetString()!);
    using MavlinkMessage built2 = position.CreateMessage();
    foreach (JsonElement entry in filled.GetProperty("values").EnumerateArray())
    {
        built2.Set(entry.GetProperty("field").GetString()!, entry.GetProperty("value").GetDouble());
    }

    Assert(
        Convert.ToHexString(built2.Payload).ToLowerInvariant() == filled.GetProperty("payload").GetString(),
        "the payload of a filled message");

    using MavlinkFrame frame = built2.ToFrame(new MavlinkHeader(1, 1, 7));
    Assert(
        Convert.ToHexString(frame.Bytes).ToLowerInvariant() == filled.GetProperty("frame").GetString(),
        "the frame of a filled message");

    using MavlinkMessage read = position.Decode(frame.Payload);
    foreach (JsonElement entry in filled.GetProperty("values").EnumerateArray())
    {
        string field = entry.GetProperty("field").GetString()!;
        Assert(
            read.GetInt64(field) == entry.GetProperty("value").GetInt64(),
            $"the value read back from {field}");
    }

    // A char array carries text padded with zeros.
    JsonElement text = vector.GetProperty("text");
    using MavlinkSchema status = MavlinkSchema.ForName(text.GetProperty("name").GetString()!);
    string textField = text.GetProperty("field").GetString()!;
    int textLength = status.Fields.First(field => field.Name == textField).ArrayLen;

    using MavlinkMessage written = status.CreateMessage();
    written.Set("severity", text.GetProperty("severity").GetDouble());
    written.SetText(textField, text.GetProperty("value").GetString()!);
    Assert(
        Convert.ToHexString(written.Payload).ToLowerInvariant() == text.GetProperty("payload").GetString(),
        "the payload of a text message");
    Assert(
        written.GetText(textField, textLength) == text.GetProperty("value").GetString(),
        "the text read back");

    // A value an integer field cannot hold exactly is refused rather than truncated.
    using MavlinkMessage report = position.CreateMessage();
    foreach (JsonElement refused in vector.GetProperty("refused").EnumerateArray())
    {
        string field = refused.GetProperty("field").GetString()!;
        double value = refused.GetProperty("value").GetDouble();
        AssertThrows(() => report.Set(field, value), $"{value} is refused for {field}");
    }
}


// The service protocols: a whole mission upload frame by frame, command acknowledgments,
// and setpoints, each pinned to the exact bytes the engine puts on the wire.
static void ConformMavlinkProtocol(JsonElement vector)
{
    MavlinkHeader vehicle = HeaderOf(vector.GetProperty("vehicle"));
    MavlinkHeader station = HeaderOf(vector.GetProperty("station"));
    using MavlinkDialect common = new();
    MavlinkFrame Parse(string hex) => MavlinkFrame.ParseKnown(Convert.FromHexString(hex), common);
    string BytesOf(MavlinkFrame frame) => Convert.ToHexString(frame.Bytes).ToLowerInvariant();

    // The plan's items are built by field name and must match the pinned payloads.
    using MavlinkSchema itemShape = MavlinkSchema.ForName("MISSION_ITEM_INT");
    using MavlinkMissionSender upload = new(1, 1);
    int index = 0;
    foreach (JsonElement fields in vector.GetProperty("plan").EnumerateArray())
    {
        using MavlinkMessage item = itemShape.CreateMessage();
        foreach (JsonElement entry in fields.EnumerateArray())
        {
            item.Set(entry.GetProperty("field").GetString()!, entry.GetProperty("value").GetDouble());
        }

        Assert(
            Convert.ToHexString(item.Payload).ToLowerInvariant()
                == vector.GetProperty("planPayloads")[index].GetString(),
            $"plan item {index}");
        upload.AddItem(item);
        index += 1;
    }

    Assert(upload.Count == index, "the plan's length");

    using MavlinkMissionReceiver download = new(255, 190);
    using MavlinkFrame requestList = download.RequestList(station);
    Assert(BytesOf(requestList) == vector.GetProperty("requestList").GetString(), "the request list frame");
    MavlinkSenderStep opened = upload.OnFrame(requestList, vehicle)!.Value;
    Assert(opened.Kind == MavlinkSenderKind.Reply, "a request list is answered");
    using (opened.Reply)
    {
        Assert(BytesOf(opened.Reply!) == vector.GetProperty("count").GetString(), "the count frame");
    }

    index = 0;
    foreach (JsonElement step in vector.GetProperty("exchange").EnumerateArray())
    {
        using MavlinkFrame feed = Parse(step.GetProperty("feed").GetString()!);
        MavlinkReceiverStep received = download.OnFrame(feed, station)!.Value;
        string receiverKind = received.Kind == MavlinkReceiverKind.Request ? "request" : "ack";
        Assert(receiverKind == step.GetProperty("receiverKind").GetString(), $"receiver kind at {index}");
        Assert(
            (received.Accepted is not null) == step.GetProperty("accepted").GetBoolean(),
            $"whether an item was accepted at {index}");
        if (received.Accepted is not null)
        {
            Assert(
                received.Accepted.GetInt64("seq") == step.GetProperty("acceptedSeq").GetInt64(),
                $"the accepted item's sequence at {index}");
            received.Accepted.Dispose();
        }

        Assert(BytesOf(received.Reply) == step.GetProperty("reply").GetString(), $"the receiver's reply at {index}");

        MavlinkSenderStep answered = upload.OnFrame(received.Reply, vehicle)!.Value;
        received.Reply.Dispose();
        string senderKind = answered.Kind == MavlinkSenderKind.Reply ? "reply" : "finished";
        Assert(senderKind == step.GetProperty("senderKind").GetString(), $"sender kind at {index}");
        if (answered.Kind == MavlinkSenderKind.Reply)
        {
            Assert(BytesOf(answered.Reply!) == step.GetProperty("senderReply").GetString(), $"the sender's reply at {index}");
            answered.Reply!.Dispose();
        }
        else
        {
            Assert(answered.Result == step.GetProperty("senderResult").GetByte(), $"the transfer's result at {index}");
        }

        index += 1;
    }

    Assert(download.Complete, "the download finished");

    // A request past the end of the plan is refused with the published result.
    JsonElement overrun = vector.GetProperty("overrun");
    using MavlinkFrame overrunRequest = Parse(overrun.GetProperty("request").GetString()!);
    MavlinkSenderStep refusal = upload.OnFrame(overrunRequest, station)!.Value;
    using (refusal.Reply)
    {
        Assert(BytesOf(refusal.Reply!) == overrun.GetProperty("reply").GetString(), "the refusal");
        using MavlinkSchema ackShape = MavlinkSchema.ForName("MISSION_ACK");
        using MavlinkMessage ack = ackShape.Decode(refusal.Reply!.Payload);
        Assert(ack.GetInt64("type") == overrun.GetProperty("result").GetInt64(), "the refusal's result");
    }

    // The command protocol classifies acknowledgments and counts retries.
    JsonElement command = vector.GetProperty("command");
    using MavlinkCommand arm = new(
        command.GetProperty("command").GetUInt16(),
        command.GetProperty("maxRetries").GetByte());
    foreach (JsonElement described in command.GetProperty("acks").EnumerateArray())
    {
        using MavlinkFrame ackFrame = Parse(described.GetProperty("frame").GetString()!);
        MavlinkAckOutcome outcome = arm.OnFrame(ackFrame)!.Value;
        string kind = outcome.Kind switch
        {
            MavlinkAckKind.Unrelated => "unrelated",
            MavlinkAckKind.InProgress => "inProgress",
            _ => "final",
        };
        Assert(kind == described.GetProperty("kind").GetString(), "an ack's kind");
        byte? value = described.GetProperty("value").ValueKind == JsonValueKind.Null
            ? null
            : described.GetProperty("value").GetByte();
        Assert(outcome.Value == value, "an ack's value");
    }

    foreach (JsonElement want in command.GetProperty("timeouts").EnumerateArray())
    {
        byte? expected = want.ValueKind == JsonValueKind.Null ? null : want.GetByte();
        Assert(arm.OnTimeout() == expected, "a timeout's verdict");
    }

    // A frame none of the machines handle is passed over by all of them.
    using MavlinkFrame ignored = Parse(vector.GetProperty("ignored").GetString()!);
    Assert(upload.OnFrame(ignored, station) is null, "the sender ignores it");
    Assert(download.OnFrame(ignored, station) is null, "the receiver ignores it");
    Assert(arm.OnFrame(ignored) is null, "the command ignores it");

    // Setpoints put exactly these bytes on the wire.
    JsonElement offboard = vector.GetProperty("offboard");
    foreach (JsonElement entry in offboard.GetProperty("typeMasks").EnumerateArray())
    {
        uint flags = entry.GetProperty("flags").GetUInt32();
        Assert(
            MavlinkOffboard.TypeMask((MavlinkTypeMask)flags) == entry.GetProperty("mask").GetUInt16(),
            $"the mask for {flags}");
    }

    JsonElement local = offboard.GetProperty("localPosition");
    using MavlinkFrame position = MavlinkOffboard.LocalPosition(
        station,
        local.GetProperty("timeBootMs").GetUInt32(),
        local.GetProperty("coordinateFrame").GetByte(),
        local.GetProperty("targetSystem").GetByte(),
        local.GetProperty("targetComponent").GetByte(),
        local.GetProperty("x").GetSingle(),
        local.GetProperty("y").GetSingle(),
        local.GetProperty("z").GetSingle());
    Assert(BytesOf(position) == local.GetProperty("frame").GetString(), "a position setpoint");

    JsonElement velocity = offboard.GetProperty("localVelocity");
    using MavlinkFrame speed = MavlinkOffboard.LocalVelocity(
        station,
        velocity.GetProperty("timeBootMs").GetUInt32(),
        velocity.GetProperty("coordinateFrame").GetByte(),
        velocity.GetProperty("targetSystem").GetByte(),
        velocity.GetProperty("targetComponent").GetByte(),
        velocity.GetProperty("vx").GetSingle(),
        velocity.GetProperty("vy").GetSingle(),
        velocity.GetProperty("vz").GetSingle());
    Assert(BytesOf(speed) == velocity.GetProperty("frame").GetString(), "a velocity setpoint");

    JsonElement global = offboard.GetProperty("globalPosition");
    using MavlinkFrame fix = MavlinkOffboard.GlobalPosition(
        station,
        global.GetProperty("timeBootMs").GetUInt32(),
        global.GetProperty("coordinateFrame").GetByte(),
        global.GetProperty("targetSystem").GetByte(),
        global.GetProperty("targetComponent").GetByte(),
        global.GetProperty("latInt").GetInt32(),
        global.GetProperty("lonInt").GetInt32(),
        global.GetProperty("alt").GetSingle());
    Assert(BytesOf(fix) == global.GetProperty("frame").GetString(), "a global setpoint");
}

static MavlinkHeader HeaderOf(JsonElement described) =>
    new(
        described.GetProperty("systemId").GetByte(),
        described.GetProperty("componentId").GetByte(),
        described.GetProperty("sequence").GetByte());

static void ConformLoraBudget(JsonElement lora)
{
    JsonElement vector = lora.GetProperty("budget");
    static int Hundredths(double value) => (int)Math.Round(value * 100, MidpointRounding.AwayFromZero);
    static double Level(JsonElement value) => value.GetInt32() / 100.0;

    Assert(
        Hundredths(LoraLinkBudget.RadioNoiseFigureDb)
            == vector.GetProperty("radioNoiseFigureHundredths").GetInt32(),
        "the radio noise figure");
    Assert(
        Hundredths(LoraLinkBudget.GatewayNoiseFigureDb)
            == vector.GetProperty("gatewayNoiseFigureHundredths").GetInt32(),
        "the gateway noise figure");
    Assert(
        Hundredths(new LoraLinkBudget().NoiseFigureDb)
            == vector.GetProperty("radioNoiseFigureHundredths").GetInt32(),
        "a default budget hears with the radio noise figure");

    foreach (JsonElement floor in vector.GetProperty("noiseFloors").EnumerateArray())
    {
        Assert(
            Hundredths(LoraLinkBudget.NoiseFloorDbm(floor.GetProperty("bandwidthHz").GetUInt32()))
                == floor.GetProperty("hundredths").GetInt32(),
            "the noise floor of a channel");
    }

    foreach (JsonElement snr in vector.GetProperty("demodulatorSnrs").EnumerateArray())
    {
        Assert(
            Hundredths(LoraLinkBudget.DemodulatorSnrDb(snr.GetProperty("spreadingFactor").GetByte()))
                == snr.GetProperty("hundredths").GetInt32(),
            "the demodulator SNR at a spreading factor");
    }

    foreach (JsonElement loss in vector.GetProperty("freeSpaceLosses").EnumerateArray())
    {
        Assert(
            Hundredths(LoraLinkBudget.FreeSpaceLossDb(
                loss.GetProperty("distanceM").GetUInt32(),
                loss.GetProperty("frequencyHz").GetUInt32()))
                == loss.GetProperty("hundredths").GetInt32(),
            "the free-space loss of a path");
    }

    foreach (JsonElement radius in vector.GetProperty("fresnelRadii").EnumerateArray())
    {
        Assert(
            LoraLinkBudget.FresnelRadiusMillimeters(
                radius.GetProperty("nearM").GetUInt32(),
                radius.GetProperty("farM").GetUInt32(),
                radius.GetProperty("frequencyHz").GetUInt32())
                == radius.GetProperty("radiusMm").GetUInt32(),
            "the first Fresnel radius");
    }

    foreach (JsonElement described in vector.GetProperty("budgets").EnumerateArray())
    {
        string linkName = described.GetProperty("link").GetString()!;
        LoraLink link = LinkOf(lora.GetProperty("links").EnumerateArray()
            .First(entry => entry.GetProperty("name").GetString() == linkName));
        var budget = new LoraLinkBudget
        {
            TransmitPowerDbm = Level(described.GetProperty("transmitPowerHundredths")),
            TransmitAntennaGainDbi = Level(described.GetProperty("transmitAntennaGainHundredths")),
            TransmitCableLossDb = Level(described.GetProperty("transmitCableLossHundredths")),
            ReceiveAntennaGainDbi = Level(described.GetProperty("receiveAntennaGainHundredths")),
            ReceiveCableLossDb = Level(described.GetProperty("receiveCableLossHundredths")),
            NoiseFigureDb = Level(described.GetProperty("noiseFigureHundredths")),
        };
        double path = Level(described.GetProperty("pathLossHundredths"));
        double ceiling = Level(described.GetProperty("ceilingHundredths"));
        Assert(
            Hundredths(budget.EirpDbm) == described.GetProperty("eirpHundredths").GetInt32(),
            "the EIRP of a budget");
        Assert(
            Hundredths(budget.ReceivedDbm(path))
                == described.GetProperty("receivedHundredths").GetInt32(),
            "the power a path delivers");
        Assert(
            Hundredths(budget.SensitivityDbm(link))
                == described.GetProperty("sensitivityHundredths").GetInt32(),
            "the sensitivity of a receiver");
        Assert(
            Hundredths(budget.MaxPathLossDb(link))
                == described.GetProperty("maxPathLossHundredths").GetInt32(),
            "the most path loss a link survives");
        Assert(
            Hundredths(budget.MarginDb(link, path))
                == described.GetProperty("marginHundredths").GetInt32(),
            "the margin a path leaves");
        Assert(
            Hundredths(budget.MaxTransmitPowerDbm(ceiling))
                == described.GetProperty("maxTransmitPowerHundredths").GetInt32(),
            "the most transmit power under a ceiling");
    }

    foreach (JsonElement rule in vector.GetProperty("fcc").EnumerateArray())
    {
        JsonElement channels = rule.GetProperty("hoppingChannels");
        double? limit = LoraLinkBudget.FccMaxConductedDbm(
            Level(rule.GetProperty("antennaGainHundredths")),
            channels.ValueKind == JsonValueKind.Null ? null : channels.GetUInt16());
        JsonElement want = rule.GetProperty("maxConductedHundredths");
        int? expected = want.ValueKind == JsonValueKind.Null ? null : want.GetInt32();
        Assert(
            (limit is null ? null : Hundredths(limit.Value)) == expected,
            "the 47 CFR 15.247 conducted power limit");
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

static void ConformRadios(JsonElement vector, JsonElement lora)
{
    var links = new Dictionary<string, LoraLink>(StringComparer.Ordinal);
    foreach (JsonElement described in lora.GetProperty("links").EnumerateArray())
    {
        links[described.GetProperty("name").GetString()!] = LinkOf(described);
    }

    static string Hex(ReadOnlySpan<byte> bytes) => Convert.ToHexString(bytes).ToLowerInvariant();
    static byte[] Unhex(JsonElement value) => Convert.FromHexString(value.GetString()!);
    static string Text(JsonElement value) => value.GetString()!;
    static string Pascal(string name) => char.ToUpperInvariant(name[0]) + name[1..];
    static int Hundredths(double value) => (int)Math.Round(value * 100, MidpointRounding.AwayFromZero);
    static Sx126xAmplifier AmplifierOf(JsonElement value) =>
        value.GetString() == "low" ? Sx126xAmplifier.LowPower : Sx126xAmplifier.HighPower;
    static string PaConfigOf(Sx126xTxPower power) =>
        Hex([power.PaDutyCycle, power.HpMax, power.DeviceSel, power.PaLut]);

    foreach (JsonElement entry in vector.GetProperty("frequencyWords").EnumerateArray())
    {
        Assert(
            Sx126x.FrequencyWord(entry.GetProperty("frequencyHz").GetUInt32())
                == entry.GetProperty("word").GetUInt32(),
            "the SX126x frequency word");
    }

    foreach (JsonElement entry in vector.GetProperty("timeouts").EnumerateArray())
    {
        Assert(
            Sx126x.TimeoutSteps(entry.GetProperty("timeoutUs").GetUInt64())
                == entry.GetProperty("steps").GetUInt32(),
            "the SX126x timeout steps");
    }

    Assert(
        Sx126x.RxContinuous == vector.GetProperty("rxContinuous").GetUInt32(),
        "the continuous receive timeout");

    foreach (JsonElement entry in vector.GetProperty("imageCalibrations").EnumerateArray())
    {
        byte[] codes = Sx126x.ImageCalibration(
            entry.GetProperty("lowHz").GetUInt32(),
            entry.GetProperty("highHz").GetUInt32());
        Assert(Hex(codes) == Text(entry.GetProperty("codes")), "the image calibration codes for a band");
    }

    foreach (JsonElement entry in vector.GetProperty("txPowers").EnumerateArray())
    {
        Sx126xTxPower power = Sx126x.TxPower(
            AmplifierOf(entry.GetProperty("amplifier")),
            entry.GetProperty("outputDbm").GetSByte());
        Assert(PaConfigOf(power) == Text(entry.GetProperty("paConfig")), "the SetPaConfig bytes for an output power");
        Assert(power.SettingDbm == entry.GetProperty("settingDbm").GetSByte(), "the SetTxParams power for an output power");
    }

    foreach (JsonElement entry in vector.GetProperty("underCeilings").EnumerateArray())
    {
        var budget = new LoraLinkBudget
        {
            TransmitAntennaGainDbi = entry.GetProperty("transmitAntennaGainHundredths").GetInt32() / 100.0,
            TransmitCableLossDb = entry.GetProperty("transmitCableLossHundredths").GetInt32() / 100.0,
        };
        Sx126xTxPower power = Sx126x.TxPowerUnderCeiling(
            AmplifierOf(entry.GetProperty("amplifier")),
            budget,
            entry.GetProperty("ceilingHundredths").GetInt32() / 100.0);
        Assert(PaConfigOf(power) == Text(entry.GetProperty("paConfig")), "the SetPaConfig bytes under a ceiling");
        Assert(power.SettingDbm == entry.GetProperty("settingDbm").GetSByte(), "the SetTxParams power under a ceiling");
    }

    JsonElement syncWords = vector.GetProperty("syncWords");
    Assert(Sx126x.SyncWordPublic.ToString("x4") == Text(syncWords.GetProperty("public")), "the public sync word");
    Assert(Sx126x.SyncWordPrivate.ToString("x4") == Text(syncWords.GetProperty("private")), "the private sync word");

    JsonElement commands = vector.GetProperty("commands");
    Assert(Hex(Sx126x.SetStandby()) == Text(commands.GetProperty("setStandby")), "SetStandby");
    Assert(Hex(Sx126x.SetPacketTypeLora()) == Text(commands.GetProperty("setPacketTypeLora")), "SetPacketType");

    JsonElement frequency = commands.GetProperty("setRfFrequency");
    Assert(
        Hex(Sx126x.SetRfFrequency(frequency.GetProperty("frequencyHz").GetUInt32()))
            == Text(frequency.GetProperty("bytes")),
        "SetRfFrequency");

    JsonElement calibration = commands.GetProperty("calibrateImage");
    Assert(
        Hex(Sx126x.CalibrateImage(
            calibration.GetProperty("lowHz").GetUInt32(),
            calibration.GetProperty("highHz").GetUInt32()))
            == Text(calibration.GetProperty("bytes")),
        "CalibrateImage");

    JsonElement paConfig = commands.GetProperty("setPaConfig");
    Sx126xTxPower high14 = Sx126x.TxPower(Sx126xAmplifier.HighPower, paConfig.GetProperty("outputDbm").GetSByte());
    Assert(Hex(Sx126x.SetPaConfig(high14)) == Text(paConfig.GetProperty("bytes")), "SetPaConfig");

    JsonElement txParams = commands.GetProperty("setTxParams");
    Assert(
        Hex(Sx126x.SetTxParams(high14, txParams.GetProperty("rampUs").GetUInt32()))
            == Text(txParams.GetProperty("bytes")),
        "SetTxParams");

    foreach (JsonElement entry in commands.GetProperty("setLoraModulationParams").EnumerateArray())
    {
        Assert(
            Hex(Sx126x.SetLoraModulationParams(links[Text(entry.GetProperty("link"))]))
                == Text(entry.GetProperty("bytes")),
            "SetModulationParams for a link");
    }

    foreach (JsonElement entry in commands.GetProperty("setLoraPacketParams").EnumerateArray())
    {
        byte[] packet = Sx126x.SetLoraPacketParams(
            links[Text(entry.GetProperty("link"))],
            entry.GetProperty("payloadLen").GetByte(),
            entry.GetProperty("invertIq").GetBoolean());
        Assert(Hex(packet) == Text(entry.GetProperty("bytes")), "SetPacketParams for a link");
    }

    foreach (JsonElement entry in commands.GetProperty("setDioIrqParams").EnumerateArray())
    {
        byte[] routing = Sx126x.SetDioIrqParams(
            (Sx126xIrq)entry.GetProperty("irq").GetUInt16(),
            (Sx126xIrq)entry.GetProperty("dio1").GetUInt16());
        Assert(Hex(routing) == Text(entry.GetProperty("bytes")), "SetDioIrqParams");
    }

    JsonElement clear = commands.GetProperty("clearIrqStatus");
    Assert(
        Hex(Sx126x.ClearIrqStatus((Sx126xIrq)clear.GetProperty("irq").GetUInt16()))
            == Text(clear.GetProperty("bytes")),
        "ClearIrqStatus");

    JsonElement transmit = commands.GetProperty("setTx");
    Assert(
        Hex(Sx126x.SetTx(transmit.GetProperty("timeoutUs").GetUInt64())) == Text(transmit.GetProperty("bytes")),
        "SetTx");

    JsonElement receive = commands.GetProperty("setRx");
    Assert(
        Hex(Sx126x.SetRx(receive.GetProperty("timeoutUs").GetUInt64())) == Text(receive.GetProperty("bytes")),
        "SetRx");
    Assert(Hex(Sx126x.SetRxContinuous()) == Text(commands.GetProperty("setRxContinuous")), "SetRx continuous");

    JsonElement sleep = commands.GetProperty("setSleep");
    Assert(
        Hex(Sx126x.SetSleep(sleep.GetProperty("warmStart").GetBoolean())) == Text(sleep.GetProperty("bytes")),
        "SetSleep");

    Assert(
        Hex(Sx126x.WriteRegister(Sx126x.RegisterLoraSyncWord, Unhex(syncWords.GetProperty("public"))))
            == Text(commands.GetProperty("setSyncWord").GetProperty("bytes")),
        "the sync word write");

    JsonElement register = commands.GetProperty("writeRegister");
    Assert(
        Hex(Sx126x.WriteRegister(
            register.GetProperty("address").GetUInt16(),
            Unhex(register.GetProperty("values"))))
            == Text(register.GetProperty("bytes")),
        "WriteRegister");

    JsonElement buffer = commands.GetProperty("writeBuffer");
    Assert(
        Hex(Sx126x.WriteBuffer(buffer.GetProperty("offset").GetByte(), Unhex(buffer.GetProperty("payload"))))
            == Text(buffer.GetProperty("bytes")),
        "WriteBuffer");

    void SameQuery(Sx126xQuery got, JsonElement want, string name)
    {
        Assert(Hex(got.Bytes) == Text(want.GetProperty("bytes")), name);
        Assert(got.AnswerLength == want.GetProperty("answerLen").GetInt32(), name);
    }

    JsonElement queries = vector.GetProperty("queries");
    SameQuery(Sx126x.GetStatus(), queries.GetProperty("getStatus"), "GetStatus");
    SameQuery(Sx126x.GetIrqStatus(), queries.GetProperty("getIrqStatus"), "GetIrqStatus");
    SameQuery(Sx126x.GetRxBufferStatus(), queries.GetProperty("getRxBufferStatus"), "GetRxBufferStatus");
    SameQuery(Sx126x.GetPacketStatus(), queries.GetProperty("getPacketStatus"), "GetPacketStatus");
    SameQuery(Sx126x.GetRssiInst(), queries.GetProperty("getRssiInst"), "GetRssiInst");
    SameQuery(Sx126x.GetDeviceErrors(), queries.GetProperty("getDeviceErrors"), "GetDeviceErrors");

    JsonElement readRegister = queries.GetProperty("readRegister");
    SameQuery(
        Sx126x.ReadRegister(
            readRegister.GetProperty("address").GetUInt16(),
            readRegister.GetProperty("length").GetByte()),
        readRegister.GetProperty("query"),
        "ReadRegister");

    JsonElement readBuffer = queries.GetProperty("readBuffer");
    SameQuery(
        Sx126x.ReadBuffer(readBuffer.GetProperty("offset").GetByte(), readBuffer.GetProperty("length").GetByte()),
        readBuffer.GetProperty("query"),
        "ReadBuffer");

    var irqFlags = new Dictionary<string, Sx126xIrq>(StringComparer.Ordinal);
    foreach (JsonProperty named in vector.GetProperty("irqFlags").EnumerateObject())
    {
        Sx126xIrq flag = Enum.Parse<Sx126xIrq>(Pascal(named.Name));
        Assert((ushort)flag == named.Value.GetUInt16(), $"the {named.Name} interrupt bit");
        irqFlags[named.Name] = flag;
    }

    foreach (JsonElement entry in vector.GetProperty("irqs").EnumerateArray())
    {
        Sx126xIrq bits = Sx126x.Irq(Unhex(entry.GetProperty("bytes")));
        Assert((ushort)bits == entry.GetProperty("bits").GetUInt16(), "the interrupt bits of an answer");
        string[] raised = entry.GetProperty("flags").EnumerateArray().Select(Text).ToArray();
        foreach ((string name, Sx126xIrq flag) in irqFlags)
        {
            Assert(bits.HasFlag(flag) == raised.Contains(name), $"{name} in an interrupt answer");
        }
    }

    foreach (JsonElement entry in vector.GetProperty("statuses").EnumerateArray())
    {
        Sx126xStatus status = Sx126x.Status(entry.GetProperty("byte").GetByte());
        Assert(
            status.ChipMode.ToString() == Pascal(Text(entry.GetProperty("chipMode"))),
            "the chip mode of a status byte");
        Assert(
            status.CommandStatus.ToString() == Pascal(Text(entry.GetProperty("commandStatus"))),
            "the command status of a status byte");
        Assert(status.Error == entry.GetProperty("error").GetBoolean(), "whether a status byte reports a failure");
    }

    foreach (JsonElement entry in vector.GetProperty("packetStatuses").EnumerateArray())
    {
        Sx126xPacketStatus status = Sx126x.PacketStatus(Unhex(entry.GetProperty("bytes")));
        Assert(Hundredths(status.RssiDbm) == entry.GetProperty("rssiHundredths").GetInt32(), "the RSSI of a frame");
        Assert(Hundredths(status.SnrDb) == entry.GetProperty("snrHundredths").GetInt32(), "the SNR of a frame");
        Assert(
            Hundredths(status.SignalRssiDbm) == entry.GetProperty("signalRssiHundredths").GetInt32(),
            "the signal RSSI of a frame");
    }

    foreach (JsonElement entry in vector.GetProperty("rxBufferStatuses").EnumerateArray())
    {
        Sx126xRxBufferStatus status = Sx126x.RxBufferStatus(Unhex(entry.GetProperty("bytes")));
        Assert(status.PayloadLength == entry.GetProperty("payloadLen").GetByte(), "the length of a received payload");
        Assert(status.Start == entry.GetProperty("start").GetByte(), "where a received payload starts");
    }

    foreach (JsonElement entry in vector.GetProperty("rssiInst").EnumerateArray())
    {
        Assert(
            Hundredths(Sx126x.RssiInstDbm(entry.GetProperty("byte").GetByte()))
                == entry.GetProperty("hundredths").GetInt32(),
            "the instantaneous RSSI");
    }

    foreach (JsonElement entry in vector.GetProperty("deviceErrors").EnumerateArray())
    {
        Sx126xDeviceErrors bits = Sx126x.DeviceErrors(Unhex(entry.GetProperty("bytes")));
        Assert((ushort)bits == entry.GetProperty("bits").GetUInt16(), "the device error bits");
        string[] reported = entry.GetProperty("flags").EnumerateArray().Select(Text).ToArray();
        foreach (Sx126xDeviceErrors fault in Enum.GetValues<Sx126xDeviceErrors>())
        {
            if (fault == Sx126xDeviceErrors.None)
            {
                continue;
            }

            string name = char.ToLowerInvariant(fault.ToString()[0]) + fault.ToString()[1..];
            Assert(bits.HasFlag(fault) == reported.Contains(name), $"{name} in a device error answer");
        }
    }

    JsonElement duty = vector.GetProperty("dutyCycle");
    using var guard = new RadioDutyCycle(duty.GetProperty("permille").GetUInt32());
    Assert(
        guard.Transmitted(
            duty.GetProperty("startedUs").GetUInt64(),
            links[Text(duty.GetProperty("link"))],
            duty.GetProperty("payloadLen").GetInt32())
            == duty.GetProperty("airtimeUs").GetUInt64(),
        "the airtime a transmission records");
    Assert(guard.EarliestMicros == duty.GetProperty("earliestUs").GetUInt64(), "the earliest next transmission");
    foreach (JsonElement check in duty.GetProperty("checks").EnumerateArray())
    {
        ulong nowMicros = check.GetProperty("nowUs").GetUInt64();
        Assert(guard.WaitMicros(nowMicros) == check.GetProperty("waitUs").GetUInt64(), "the silence still owed");
        Assert(guard.Ready(nowMicros) == check.GetProperty("ready").GetBoolean(), "whether a frame may start");
    }

    JsonElement forbidden = duty.GetProperty("forbidden");
    using var never = new RadioDutyCycle(forbidden.GetProperty("permille").GetUInt32());
    foreach (JsonElement at in forbidden.GetProperty("readyAt").EnumerateArray())
    {
        Assert(never.Ready(at.GetUInt64()) == forbidden.GetProperty("ready").GetBoolean(), "a zero limit at any time");
    }

    Assert(never.EarliestMicros is null, "a zero limit never clears");
}

static void ConformSx127x(JsonElement radios, JsonElement lora)
{
    JsonElement vector = radios.GetProperty("sx127x");
    var links = new Dictionary<string, LoraLink>(StringComparer.Ordinal);
    foreach (JsonElement described in lora.GetProperty("links").EnumerateArray())
    {
        links[described.GetProperty("name").GetString()!] = LinkOf(described);
    }

    static string Text(JsonElement value) => value.GetString()!;
    static string Pascal(string name) => char.ToUpperInvariant(name[0]) + name[1..];
    static int Hundredths(double value) => (int)Math.Round(value * 100, MidpointRounding.AwayFromZero);
    static Sx127xPaOutput OutputOf(JsonElement value) =>
        value.GetString() == "rfo" ? Sx127xPaOutput.Rfo : Sx127xPaOutput.PaBoost;
    static byte? OptionalByte(JsonElement value) =>
        value.ValueKind == JsonValueKind.Null ? null : value.GetByte();

    foreach (JsonProperty register in vector.GetProperty("registers").EnumerateObject())
    {
        System.Reflection.FieldInfo? field = typeof(Sx127xRegister).GetField(Pascal(register.Name));
        Assert(
            field is not null && (byte)field.GetValue(null)! == register.Value.GetByte(),
            $"the {register.Name} register");
    }

    (string Name, byte Value)[] named =
    [
        ("version", Sx127x.Version),
        ("write", Sx127x.Write),
        ("dio0RxDone", Sx127x.Dio0RxDone),
        ("dio0TxDone", Sx127x.Dio0TxDone),
        ("dio0CadDone", Sx127x.Dio0CadDone),
        ("paDacDefault", Sx127x.PaDacDefault),
        ("paDacHighPower", Sx127x.PaDacHighPower),
        ("imageCalStart", Sx127x.ImageCalStartBit),
        ("imageCalRunning", Sx127x.ImageCalRunningBit),
        ("syncWordPublic", Sx127x.SyncWordPublic),
        ("syncWordPrivate", Sx127x.SyncWordPrivate),
        ("lnaBoosted", Sx127x.LnaBoosted),
        ("tcxoInputOn", Sx127x.TcxoInputOn),
    ];
    JsonElement constants = vector.GetProperty("constants");
    foreach ((string name, byte value) in named)
    {
        Assert(constants.GetProperty(name).GetByte() == value, $"the {name} constant");
    }

    Assert(constants.EnumerateObject().Count() == named.Length, "every SX127x constant is named");

    foreach (JsonProperty flag in vector.GetProperty("irqFlags").EnumerateObject())
    {
        Assert(
            (byte)Enum.Parse<Sx127xIrq>(Pascal(flag.Name)) == flag.Value.GetByte(),
            $"the {flag.Name} flag");
    }

    foreach (JsonElement entry in vector.GetProperty("modes").EnumerateArray())
    {
        Sx127xMode mode = Enum.Parse<Sx127xMode>(Pascal(Text(entry.GetProperty("mode"))));
        Assert(Sx127x.LoraOpMode(mode) == entry.GetProperty("lora").GetByte(), "a LoRa op mode");
        Assert(Sx127x.FskOpMode(mode) == entry.GetProperty("fsk").GetByte(), "an FSK op mode");
        Assert(Sx127x.ModeFromOpMode(entry.GetProperty("lora").GetByte()) == mode, "the mode of an op mode");
    }

    foreach (JsonElement entry in vector.GetProperty("addresses").EnumerateArray())
    {
        byte address = entry.GetProperty("address").GetByte();
        Assert(Sx127x.ReadAddress(address) == entry.GetProperty("read").GetByte(), "a read address byte");
        Assert(Sx127x.WriteAddress(address) == entry.GetProperty("write").GetByte(), "a write address byte");
    }

    foreach (JsonElement entry in vector.GetProperty("frequencyWords").EnumerateArray())
    {
        Assert(
            Sx127x.FrequencyWord(entry.GetProperty("frequencyHz").GetUInt32())
                == entry.GetProperty("word").GetUInt32(),
            "the SX127x frequency word");
    }

    foreach (JsonElement entry in vector.GetProperty("modems").EnumerateArray())
    {
        LoraLink link = links[Text(entry.GetProperty("link"))];
        ushort symbols = entry.GetProperty("symbolTimeout").GetUInt16();
        Assert(Sx127x.SymbolTimeout(link, 100_000) == symbols, "the symbol timeout of a link");
        Sx127xModem modem = Sx127x.Modem(link, entry.GetProperty("frequencyHz").GetUInt32(), symbols);
        Assert(modem.ModemConfig1 == entry.GetProperty("modemConfig1").GetByte(), "RegModemConfig1");
        Assert(modem.ModemConfig2 == entry.GetProperty("modemConfig2").GetByte(), "RegModemConfig2");
        Assert(modem.ModemConfig3 == entry.GetProperty("modemConfig3").GetByte(), "RegModemConfig3");
        Assert(modem.DetectionOptimize == entry.GetProperty("detectionOptimize").GetByte(), "the detection bits");
        Assert(modem.DetectionThreshold == entry.GetProperty("detectionThreshold").GetByte(), "the detection threshold");
    }

    foreach (JsonElement entry in vector.GetProperty("modemRefusals").EnumerateArray())
    {
        var refused = new LoraLink(
            entry.GetProperty("spreadingFactor").GetByte(),
            entry.GetProperty("bandwidthHz").GetUInt32());
        uint frequency = entry.GetProperty("frequencyHz").GetUInt32();
        Refuses(() => Sx127x.Modem(refused, frequency), "a link the SX127x cannot use");
    }

    foreach (JsonElement entry in vector.GetProperty("symbolTimeouts").EnumerateArray())
    {
        Assert(
            Sx127x.SymbolTimeout(links[Text(entry.GetProperty("link"))], entry.GetProperty("timeoutUs").GetUInt64())
                == entry.GetProperty("symbols").GetUInt16(),
            "a symbol timeout");
    }

    foreach (JsonElement entry in vector.GetProperty("txPowers").EnumerateArray())
    {
        Sx127xTxPower power = Sx127x.TxPower(
            OutputOf(entry.GetProperty("output")),
            entry.GetProperty("requestedDbm").GetSByte());
        Assert(
            power == new Sx127xTxPower(
                entry.GetProperty("paConfig").GetByte(),
                entry.GetProperty("paDac").GetByte(),
                entry.GetProperty("ocp").GetByte(),
                entry.GetProperty("outputDbm").GetSByte()),
            "the SX127x amplifier settings for an output power");
    }

    foreach (JsonElement entry in vector.GetProperty("underCeilings").EnumerateArray())
    {
        var budget = new LoraLinkBudget
        {
            TransmitAntennaGainDbi = entry.GetProperty("transmitAntennaGainHundredths").GetInt32() / 100.0,
            TransmitCableLossDb = entry.GetProperty("transmitCableLossHundredths").GetInt32() / 100.0,
        };
        Sx127xTxPower power = Sx127x.TxPowerUnderCeiling(
            OutputOf(entry.GetProperty("output")),
            budget,
            entry.GetProperty("ceilingHundredths").GetInt32() / 100.0);
        Assert(
            power == new Sx127xTxPower(
                entry.GetProperty("paConfig").GetByte(),
                entry.GetProperty("paDac").GetByte(),
                entry.GetProperty("ocp").GetByte(),
                entry.GetProperty("outputDbm").GetSByte()),
            "the SX127x amplifier settings under a ceiling");
    }

    foreach (JsonElement entry in vector.GetProperty("ocp").EnumerateArray())
    {
        Assert(
            Sx127x.OcpRegister(entry.GetProperty("milliamps").GetUInt16()) == entry.GetProperty("register").GetByte(),
            "RegOcp for a current limit");
    }

    foreach (JsonElement entry in vector.GetProperty("invertIq").EnumerateArray())
    {
        Assert(
            Sx127x.InvertIq(entry.GetProperty("receive").GetBoolean(), entry.GetProperty("transmit").GetBoolean())
                == entry.GetProperty("register").GetByte(),
            "RegInvertIQ");
    }

    foreach (JsonElement entry in vector.GetProperty("invertIq2").EnumerateArray())
    {
        Assert(
            Sx127x.InvertIq2(entry.GetProperty("inverted").GetBoolean()) == entry.GetProperty("register").GetByte(),
            "RegInvertIQ2");
    }

    foreach (JsonElement entry in vector.GetProperty("highBwOptimize").EnumerateArray())
    {
        Sx127xHighBwOptimize optimize = Sx127x.HighBwOptimize(
            new LoraLink(7, entry.GetProperty("bandwidthHz").GetUInt32()),
            entry.GetProperty("frequencyHz").GetUInt32());
        Assert(optimize.Optimize1 == entry.GetProperty("optimize1").GetByte(), "RegHighBwOptimize1");
        Assert(optimize.Optimize2 == OptionalByte(entry.GetProperty("optimize2")), "RegHighBwOptimize2");
    }

    foreach (JsonElement entry in vector.GetProperty("spuriousReception").EnumerateArray())
    {
        Sx127xSpuriousReception erratum = Sx127x.SpuriousReception(
            new LoraLink(7, entry.GetProperty("bandwidthHz").GetUInt32()));
        Assert(erratum.AutomaticIf == entry.GetProperty("automaticIf").GetBoolean(), "the automatic IF");
        Assert(erratum.IfFreq2 == OptionalByte(entry.GetProperty("ifFreq2")), "RegIfFreq2");
        Assert(erratum.OffsetHz == entry.GetProperty("offsetHz").GetUInt32(), "the receive offset");
    }

    foreach (JsonElement entry in vector.GetProperty("imageCalStart").EnumerateArray())
    {
        Assert(
            Sx127x.ImageCalStart(entry.GetProperty("current").GetByte()) == entry.GetProperty("register").GetByte(),
            "RegImageCal to start a calibration");
    }

    foreach (JsonElement entry in vector.GetProperty("automaticIf").EnumerateArray())
    {
        Assert(
            Sx127x.AutomaticIf(entry.GetProperty("current").GetByte(), entry.GetProperty("on").GetBoolean())
                == entry.GetProperty("register").GetByte(),
            "RegDetectOptimize with the automatic IF");
    }

    foreach (JsonElement entry in vector.GetProperty("packetStatuses").EnumerateArray())
    {
        Sx127xPacketStatus status = Sx127x.PacketStatus(
            Convert.FromHexString(Text(entry.GetProperty("bytes"))),
            entry.GetProperty("frequencyHz").GetUInt32());
        Assert(Hundredths(status.RssiDbm) == entry.GetProperty("rssiHundredths").GetInt32(), "the RSSI of a packet");
        Assert(Hundredths(status.SnrDb) == entry.GetProperty("snrHundredths").GetInt32(), "the SNR of a packet");
        Assert(
            Hundredths(status.SignalRssiDbm) == entry.GetProperty("signalRssiHundredths").GetInt32(),
            "the strength of a packet");
    }

    foreach (JsonElement entry in vector.GetProperty("rssi").EnumerateArray())
    {
        Assert(
            Hundredths(Sx127x.RssiDbm(entry.GetProperty("byte").GetByte(), entry.GetProperty("frequencyHz").GetUInt32()))
                == entry.GetProperty("hundredths").GetInt32(),
            "the instantaneous RSSI");
    }

    foreach (JsonElement entry in vector.GetProperty("modemStatuses").EnumerateArray())
    {
        Sx127xModemStatus status = Sx127x.ModemStatus(entry.GetProperty("byte").GetByte());
        Assert(
            status == new Sx127xModemStatus(
                OptionalByte(entry.GetProperty("codingRateDenominator")),
                entry.GetProperty("clear").GetBoolean(),
                entry.GetProperty("headerValid").GetBoolean(),
                entry.GetProperty("rxOngoing").GetBoolean(),
                entry.GetProperty("signalSynchronized").GetBoolean(),
                entry.GetProperty("signalDetected").GetBoolean()),
            "RegModemStat");
    }

    foreach (JsonElement entry in radios.GetProperty("llcc68").EnumerateArray())
    {
        var link = new LoraLink(
            entry.GetProperty("spreadingFactor").GetByte(),
            entry.GetProperty("bandwidthHz").GetUInt32());
        Assert(
            Sx126x.Llcc68Supports(link) == entry.GetProperty("supported").GetBoolean(),
            "what an LLCC68 supports");
    }
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
        Assert(decision.NextHop == nextHop.GetUInt32(), "the neighbor to unicast to");
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
    ConformLorawanMac(vector.GetProperty("mac"));
}

/// <summary>
/// Checks the commands a network and a device configure each other with. Each one is read
/// in the direction it names and written back out, and the bytes have to come back the same.
/// </summary>
/// <param name="vector">The vector to check against.</param>
static void ConformLorawanMac(JsonElement vector)
{
    static LorawanDirection Facing(string name) =>
        name == "downlink" ? LorawanDirection.Downlink : LorawanDirection.Uplink;

    static string Hex(byte[] bytes) => Convert.ToHexString(bytes).ToLowerInvariant();

    // The same bytes are a request going down and an answer coming up, and the two are not
    // the same length, so a reader that guesses the direction walks off the end.
    JsonElement both = vector.GetProperty("bothDirections");
    byte[] shared = Convert.FromHexString(both.GetProperty("bytes").GetString()!);
    var down = LorawanMacCommand.Parse(LorawanDirection.Downlink, shared);
    var up = LorawanMacCommand.Parse(LorawanDirection.Uplink, shared);

    byte cid = both.GetProperty("cid").GetByte();
    Assert(down[0].Cid == cid, "the identifier is the same going down");
    Assert(up[0].Cid == cid, "and coming up");
    Assert(
        down[0].Encode().Length == both.GetProperty("downlinkLength").GetInt32(),
        "a request going down is the length the specification gives it");
    Assert(
        up[0].Encode().Length == both.GetProperty("uplinkLength").GetInt32(),
        "and the answer coming up is shorter");

    foreach (JsonElement entry in vector.GetProperty("commands").EnumerateArray())
    {
        string text = entry.GetProperty("bytes").GetString()!;
        byte[] bytes = Convert.FromHexString(text);
        var read = LorawanMacCommand.Parse(Facing(entry.GetProperty("direction").GetString()!), bytes);

        Assert(read.Count == 1, $"one command in {text}");
        Assert(read[0].Cid == entry.GetProperty("cid").GetByte(), $"the identifier of {text}");
        Assert(Hex(read[0].Encode()) == text, $"{text} is written back the way it was read");
    }

    JsonElement sequence = vector.GetProperty("sequence");
    var run = LorawanMacCommand.Parse(
        Facing(sequence.GetProperty("direction").GetString()!),
        Convert.FromHexString(sequence.GetProperty("bytes").GetString()!));
    int at = 0;
    foreach (JsonElement want in sequence.GetProperty("cids").EnumerateArray())
    {
        Assert(run[at].Cid == want.GetByte(), "a field of commands reads in order");
        at++;
    }
    Assert(at == run.Count, "and holds exactly what the vector says");

    // Nothing says how long an unknown command is, so reading stops rather than guessing.
    JsonElement stops = vector.GetProperty("stops");
    var stopped = LorawanMacCommand.Parse(
        Facing(stops.GetProperty("direction").GetString()!),
        Convert.FromHexString(stops.GetProperty("bytes").GetString()!));
    Assert(
        stopped.Count == stops.GetProperty("readable").GetInt32(),
        "reading stops at the unknown one");

    JsonElement truncated = vector.GetProperty("truncated");
    var cut = LorawanMacCommand.Parse(
        Facing(truncated.GetProperty("direction").GetString()!),
        Convert.FromHexString(truncated.GetProperty("bytes").GetString()!));
    Assert(cut.Count == 0, "a known command cut short is not half read");

    foreach (JsonElement entry in vector.GetProperty("relay").EnumerateArray())
    {
        string text = entry.GetProperty("bytes").GetString()!;
        var read = LorawanMacCommand.Parse(Facing(entry.GetProperty("direction").GetString()!), Convert.FromHexString(text));
        Assert(read.Count == 1, $"one relay command in {text}");
        Assert(read[0].Cid == entry.GetProperty("cid").GetByte(), $"the identifier of {text}");
        Assert(Hex(read[0].Encode()) == text, $"{text} is written back the way it was read");
        if (entry.TryGetProperty("fields", out JsonElement fields))
        {
            foreach (JsonProperty field in fields.EnumerateObject())
            {
                string name = char.ToUpperInvariant(field.Name[0]) + field.Name[1..];
                object? got = typeof(LorawanMacCommand).GetProperty(name)!.GetValue(read[0]);
                string gotText = got is byte[] bytes ? Hex(bytes) : Convert.ToString(got, System.Globalization.CultureInfo.InvariantCulture)!;
                string wantText = field.Value.ValueKind == JsonValueKind.String
                    ? field.Value.GetString()!
                    : field.Value.GetRawText();
                Assert(gotText == wantText, $"{field.Name} of {text}");
            }
        }
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
        Assert(header.AdrAckReq == want.GetProperty("adrAckReq").GetBoolean(), "the ADRACKReq bit");
        Assert(header.ClassB == want.GetProperty("classB").GetBoolean(), "the ClassB bit");
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

// Holds the defaults, the back-off, the channel list and the join settings to the vectors.
static void ConformLorawanLink(JsonElement vector, JsonElement vectors)
{
    JsonElement d = vector.GetProperty("defaults");
    uint[] defaults =
    [
        LorawanDefaults.ReceiveDelay1Micros,
        LorawanDefaults.ReceiveDelay2Micros,
        LorawanDefaults.JoinAcceptDelay1Micros,
        LorawanDefaults.JoinAcceptDelay2Micros,
        LorawanDefaults.ReceiveWindowToleranceMicros,
        LorawanDefaults.MaxFcntGap,
        LorawanDefaults.AdrAckLimit,
        LorawanDefaults.AdrAckDelay,
        LorawanDefaults.RetransmitTimeoutMinMicros,
        LorawanDefaults.RetransmitTimeoutMaxMicros,
    ];
    string[] names =
    [
        "receiveDelay1Us",
        "receiveDelay2Us",
        "joinAcceptDelay1Us",
        "joinAcceptDelay2Us",
        "receiveWindowToleranceUs",
        "maxFcntGap",
        "adrAckLimit",
        "adrAckDelay",
        "retransmitTimeoutMinUs",
        "retransmitTimeoutMaxUs",
    ];
    for (int index = 0; index < names.Length; index++)
    {
        Assert(defaults[index] == d.GetProperty(names[index]).GetUInt32(), $"the default {names[index]}");
    }

    foreach (JsonElement script in vector.GetProperty("backoff").EnumerateArray())
    {
        string version = script.GetProperty("version").GetString()!;
        uint limit = script.GetProperty("limit").GetUInt32();
        uint delay = script.GetProperty("delay").GetUInt32();
        string where = $"a {version} back-off with limit {limit} and delay {delay}";
        LorawanVersion revision = version == "1.0.3" ? LorawanVersion.V1_0_3 : LorawanVersion.V1_0_4;
        using LorawanBackoff backoff = new(revision, limit, delay);
        Assert(backoff.Version == revision, where);

        int dataRate = script.GetProperty("startDataRate").GetInt32();
        List<(int Uplink, string Step)> steps = [];
        int? firstAsked = null;
        int? lastAsked = null;
        int uplinks = script.GetProperty("uplinks").GetInt32();
        for (int sent = 1; sent <= uplinks; sent++)
        {
            LorawanBackoffStep step = backoff.Uplink(dataRate == 0);
            if (step.RequestAck)
            {
                firstAsked ??= sent;
                lastAsked = sent;
            }

            if (step.RestorePower)
            {
                steps.Add((sent, "restorePower"));
            }

            if (step.LowerDataRate)
            {
                steps.Add((sent, "lowerDataRate"));
                dataRate--;
            }

            if (step.RestoreChannels)
            {
                steps.Add((sent, "restoreChannels"));
            }
        }

        List<(int Uplink, string Step)> wantSteps = script
            .GetProperty("steps")
            .EnumerateArray()
            .Select(entry => (entry.GetProperty("uplink").GetInt32(), entry.GetProperty("step").GetString()!))
            .ToList();
        Assert(steps.SequenceEqual(wantSteps), $"the steps of {where}");
        AssertOptional(firstAsked, script.GetProperty("firstAsked"), $"the first request of {where}");
        AssertOptional(lastAsked, script.GetProperty("lastAsked"), $"the last request of {where}");
        Assert(backoff.Counter == script.GetProperty("counter").GetUInt32(), $"the counter of {where}");
        Assert(dataRate == script.GetProperty("endDataRate").GetInt32(), $"the data rate {where} ends at");

        backoff.Downlink();
        LorawanBackoffStep after = backoff.Uplink(dataRate == 0);
        JsonElement afterDownlink = script.GetProperty("afterDownlink");
        Assert(backoff.Counter == afterDownlink.GetProperty("counter").GetUInt32(), $"a downlink resets {where}");
        Assert(after.RequestAck == afterDownlink.GetProperty("requestAck").GetBoolean(), where);
    }

    JsonElement lists = vector.GetProperty("cflist");
    JsonElement frequencies = lists.GetProperty("frequencies");
    ConformCfList(
        LorawanCfList.FromFrequencies(
            frequencies.GetProperty("input").EnumerateArray().Select(hz => hz.GetUInt32()).ToArray()),
        frequencies.GetProperty("list"),
        "a list of frequencies");
    JsonElement masks = lists.GetProperty("channelMasks");
    ConformCfList(
        LorawanCfList.FromChannelMasks(
            masks.GetProperty("input").EnumerateArray().Select(mask => mask.GetUInt16()).ToArray()),
        masks.GetProperty("list"),
        "a list of masks");
    JsonElement reserved = lists.GetProperty("reserved");
    ConformCfList(
        LorawanCfList.FromBytes(Convert.FromHexString(reserved.GetProperty("bytes").GetString()!)),
        reserved,
        "a reserved list");
    foreach (JsonElement refused in lists.GetProperty("refusedFrequencies").EnumerateArray())
    {
        try
        {
            LorawanCfList.FromFrequencies([refused.GetUInt32(), 0, 0, 0, 0]);
            Fail($"{refused.GetUInt32()} Hz must be refused in a channel list");
        }
        catch (PamojaException)
        {
        }
    }

    try
    {
        LorawanCfList.FromBytes(new byte[15]);
        Fail("a channel list is sixteen bytes");
    }
    catch (PamojaException)
    {
    }

    byte[] devEui = Convert.FromHexString(vector.GetProperty("device").GetProperty("devEui").GetString()!);
    foreach (JsonElement want in vector.GetProperty("joinAccepts").EnumerateArray())
    {
        using LorawanDevice device = new(
            devEui,
            new byte[8],
            Convert.FromHexString(want.GetProperty("appKey").GetString()!));
        Assert(device.DevEui.AsSpan().SequenceEqual(devEui), "the device EUI");
        using LorawanJoinAccept accept = device.AcceptJoin(
            Convert.FromHexString(want.GetProperty("frame").GetString()!),
            want.GetProperty("devNonce").GetUInt16());
        Assert(accept.DlSettings == want.GetProperty("dlSettings").GetByte(), "the downlink settings");
        Assert(accept.RxDelay == want.GetProperty("rxDelay").GetByte(), "the delay byte");
        Assert(accept.Rx1DrOffset == want.GetProperty("rx1DrOffset").GetByte(), "the RX1 offset");
        Assert(accept.Rx2DataRate == want.GetProperty("rx2DataRate").GetByte(), "the RX2 data rate");
        Assert(
            accept.ReceiveDelayMicros == want.GetProperty("receiveDelayUs").GetUInt32(),
            "the receive delay");
        JsonElement wantList = want.GetProperty("cflist");
        LorawanCfList? list = accept.CfList();
        Assert(
            wantList.ValueKind == JsonValueKind.Null
                ? list is null
                : list is not null && Convert.ToHexString(list.Bytes).Equals(wantList.GetString(), StringComparison.OrdinalIgnoreCase),
            "the channel list");
    }

    JsonElement uplink = vectors
        .GetProperty("header")
        .GetProperty("frames")
        .EnumerateArray()
        .First(frame => frame.GetProperty("adrAckReq").GetBoolean());
    JsonElement frames = vectors.GetProperty("lorawan");
    using LorawanSession session = new(
        frames.GetProperty("devAddr").GetUInt32(),
        Convert.FromHexString(frames.GetProperty("nwkSKey").GetString()!),
        Convert.FromHexString(frames.GetProperty("appSKey").GetString()!));
    uint fcnt = uplink.GetProperty("fcnt").GetUInt32();
    byte[] asking = session.EncodeUplink(
        fcnt,
        uplink.GetProperty("fport").GetByte(),
        "x"u8,
        new LorawanOptions { Adr = true, AdrAckReq = true });
    Assert(
        Convert.ToHexString(asking).Equals(uplink.GetProperty("frame").GetString(), StringComparison.OrdinalIgnoreCase),
        "an uplink asking the network to answer");
    LorawanRxData decoded = session.Decode(asking, fcnt);
    Assert(decoded.AdrAckReq && !decoded.ClassB, "the decoded ADRACKReq and ClassB bits");
}

// Replays each end device script, holding every call to what it returned.
static void ConformLorawanDevice(JsonElement vector)
{
    Assert(vector.GetProperty("savedLen").GetInt32() == 1678, "the saved state length");
    Dictionary<string, LoraRegion> regions = new()
    {
        ["EU868"] = LoraRegion.Eu868,
        ["US915"] = LoraRegion.Us915,
        ["EU433"] = LoraRegion.Eu433,
        ["AU915"] = LoraRegion.Au915,
        ["CN470"] = LoraRegion.Cn470,
        ["AS923"] = LoraRegion.As923,
        ["KR920"] = LoraRegion.Kr920,
        ["IN865"] = LoraRegion.In865,
        ["RU864"] = LoraRegion.Ru864,
    };
    Dictionary<string, LoraCn470Plan> cn470 = new()
    {
        ["antenna_20mhz_a"] = LoraCn470Plan.Antenna20MhzA,
        ["antenna_20mhz_b"] = LoraCn470Plan.Antenna20MhzB,
        ["antenna_26mhz_a"] = LoraCn470Plan.Antenna26MhzA,
        ["antenna_26mhz_b"] = LoraCn470Plan.Antenna26MhzB,
        ["channels_96"] = LoraCn470Plan.Channels96,
    };

    foreach (JsonElement script in vector.GetProperty("scripts").EnumerateArray())
    {
        string name = script.GetProperty("name").GetString()!;
        JsonElement planName = script.GetProperty("plan");
        using LoraChannelPlan plan = planName.TryGetProperty("region", out JsonElement region)
            ? LoraChannelPlan.ForRegion(regions[region.GetString()!])
            : LoraChannelPlan.ForCn470(cn470[planName.GetProperty("cn470").GetString()!]);
        JsonElement s = script.GetProperty("settings");
        LorawanDeviceSettings settings = new(s.GetProperty("minOutputDbm").GetSByte(), s.GetProperty("maxOutputDbm").GetSByte())
        {
            Version = s.GetProperty("version").GetString() == "1.0.3" ? LorawanVersion.V1_0_3 : LorawanVersion.V1_0_4,
            Adr = s.GetProperty("adr").GetBoolean(),
            AntennaGainDb = s.GetProperty("antennaGainDb").GetSByte(),
            LowestHz = s.GetProperty("lowestHz").GetUInt32(),
            HighestHz = s.GetProperty("highestHz").GetUInt32(),
            RegionalDutyCycle = s.GetProperty("regionalDutyCycle").GetBoolean(),
            BehindRepeater = s.GetProperty("behindRepeater").GetBoolean(),
            Seed = s.GetProperty("seed").GetUInt32(),
        };
        JsonElement counters = script.GetProperty("counters");
        uint fcntUp = counters.ValueKind == JsonValueKind.Null ? 0 : counters.GetProperty("up").GetUInt32();
        uint? fcntDown = counters.ValueKind == JsonValueKind.Null || counters.GetProperty("down").ValueKind == JsonValueKind.Null
            ? null
            : counters.GetProperty("down").GetUInt32();
        JsonElement activation = script.GetProperty("activation");
        LorawanEndDevice device;
        if (activation.TryGetProperty("overTheAir", out JsonElement keys))
        {
            using LorawanDevice credentials = new(
                Convert.FromHexString(keys.GetProperty("devEui").GetString()!),
                Convert.FromHexString(keys.GetProperty("joinEui").GetString()!),
                Convert.FromHexString(keys.GetProperty("appKey").GetString()!));
            device = LorawanEndDevice.OverTheAir(plan, credentials, settings, fcntUp, fcntDown);
        }
        else
        {
            JsonElement abp = activation.GetProperty("personalized");
            using LorawanSession session = new(
                abp.GetProperty("devAddr").GetUInt32(),
                Convert.FromHexString(abp.GetProperty("nwkSKey").GetString()!),
                Convert.FromHexString(abp.GetProperty("appSKey").GetString()!));
            device = LorawanEndDevice.Personalized(plan, session, settings, fcntUp, fcntDown);
        }

        using (device)
        {
            int index = 0;
            foreach (JsonElement step in script.GetProperty("steps").EnumerateArray())
            {
                string call = step.GetProperty("call").GetString()!;
                string where = $"step {index} ({call}) of {name}";
                index++;
                if (step.TryGetProperty("error", out JsonElement wantError))
                {
                    try
                    {
                        RunDeviceStep(device, step);
                        Fail($"{where} should have failed");
                    }
                    catch (LorawanDeviceException error)
                    {
                        Dictionary<string, object?> got = new()
                        {
                            ["kind"] = SnakeCase(error.Kind.ToString()),
                            ["untilUs"] = error.UntilMicros,
                            ["max"] = error.Max,
                            ["dataRate"] = error.DataRate,
                            ["state"] = error.State is { } state ? SnakeCase(state.ToString()) : null,
                            ["format"] = error.Format,
                        };
                        Assert(Canonical(JsonSerializer.SerializeToElement(got)) == Canonical(wantError), where);
                    }

                    continue;
                }

                object? result = RunDeviceStep(device, step);
                string? field = call switch
                {
                    "join" or "send" or "sendEmpty" or "repeat" => "transmission",
                    "heard" => "heard",
                    "nothingHeard" => "next",
                    "save" => "saved",
                    "resume" => "resumed",
                    "status" => "status",
                    "useRelay" => "taken",
                    "heardWorAck" => "relayStatus",
                    "noWorAck" => "worNext",
                    "relayMode" => "relayMode",
                    _ => null,
                };
                if (field is not null)
                {
                    Assert(
                        Canonical(JsonSerializer.SerializeToElement(result)) == Canonical(step.GetProperty(field)),
                        where);
                }
            }
        }
    }
}

// Makes one scripted call on a device and describes what it returned the way the vectors do.
static object? RunDeviceStep(LorawanEndDevice device, JsonElement step)
{
    static object Link(LoraLink link) => new Dictionary<string, object>
    {
        ["spreadingFactor"] = link.SpreadingFactor,
        ["bandwidthHz"] = link.BandwidthHz,
        ["codingRateDenominator"] = link.CodingRateDenominator,
        ["preambleSymbols"] = link.PreambleSymbols,
        ["explicitHeader"] = link.ExplicitHeader,
        ["crc"] = link.Crc,
    };
    static object Window(LorawanWindow window) => new Dictionary<string, object>
    {
        ["delayUs"] = window.DelayMicros,
        ["frequencyHz"] = window.FrequencyHz,
        ["dataRate"] = window.DataRate,
        ["link"] = Link(window.Link),
    };
    static object Transmission(LorawanTransmission transmission) => new Dictionary<string, object?>
    {
        ["frame"] = Convert.ToHexString(transmission.Frame).ToLowerInvariant(),
        ["frequencyHz"] = transmission.FrequencyHz,
        ["dataRate"] = transmission.DataRate,
        ["link"] = Link(transmission.Link),
        ["outputDbm"] = transmission.OutputDbm,
        ["airtimeUs"] = transmission.AirtimeMicros,
        ["rx1"] = Window(transmission.Rx1),
        ["rx2"] = Window(transmission.Rx2),
        ["carriesPayload"] = transmission.CarriesPayload,
        ["relay"] = transmission.Relay is { } exchange ? Exchange(exchange) : null,
    };
    static object Exchange(LorawanRelayExchange exchange) => new Dictionary<string, object?>
    {
        ["wakeUp"] = new Dictionary<string, object>
        {
            ["frame"] = Convert.ToHexString(exchange.WakeUp.Frame).ToLowerInvariant(),
            ["startUs"] = exchange.WakeUp.StartMicros,
            ["frequencyHz"] = exchange.WakeUp.Carrier.FrequencyHz,
            ["dataRate"] = exchange.WakeUp.Carrier.DataRate,
            ["link"] = Link(exchange.WakeUp.Link),
            ["outputDbm"] = exchange.WakeUp.OutputDbm,
            ["airtimeUs"] = exchange.WakeUp.AirtimeMicros,
        },
        ["ack"] = exchange.Ack is { } ack
            ? new Dictionary<string, object>
            {
                ["startUs"] = ack.StartMicros,
                ["frequencyHz"] = ack.Carrier.FrequencyHz,
                ["dataRate"] = ack.Carrier.DataRate,
                ["link"] = Link(ack.Link),
                ["airtimeUs"] = ack.AirtimeMicros,
            }
            : null,
        ["uplinkStartUs"] = exchange.UplinkStartMicros,
        ["rxr"] = Window(exchange.Rxr),
    };
    static object RelayStatus(LorawanRelayStatus status) => new Dictionary<string, object>
    {
        ["cadPeriodicity"] = status.CadPeriodicity.ToString(),
        ["xtalAccuracy"] = status.XtalAccuracy.ToString(),
        ["cadToRx"] = status.CadToRx.ToString(),
        ["relayDataRate"] = status.RelayDataRate,
        ["forward"] = status.Forward.ToString(),
    };

    switch (step.GetProperty("call").GetString())
    {
        case "join":
            return Transmission(device.Join(step.GetProperty("devNonce").GetUInt16(), step.GetProperty("nowUs").GetUInt64()));
        case "send":
            return Transmission(device.Send(
                step.GetProperty("port").GetByte(),
                Convert.FromHexString(step.GetProperty("payload").GetString()!),
                step.GetProperty("nowUs").GetUInt64(),
                step.GetProperty("confirmed").GetBoolean()));
        case "sendEmpty":
            return Transmission(device.SendEmpty(step.GetProperty("nowUs").GetUInt64()));
        case "repeat":
            return Transmission(device.Repeat(step.GetProperty("nowUs").GetUInt64()));
        case "heard":
            LorawanReceiveWindow? window = step.GetProperty("window").GetString() switch
            {
                "rx1" => LorawanReceiveWindow.Rx1,
                "rx2" => LorawanReceiveWindow.Rx2,
                "rxr" => LorawanReceiveWindow.Rxr,
                _ => null,
            };
            LorawanHeard heard = device.Heard(
                Convert.FromHexString(step.GetProperty("frame").GetString()!),
                step.GetProperty("snrDb").GetSByte(),
                window);
            return heard switch
            {
                LorawanHeard.Data data => new Dictionary<string, object>
                {
                    ["kind"] = "data",
                    ["devAddr"] = data.DevAddr,
                    ["delivery"] = new Dictionary<string, object?>
                    {
                        ["port"] = data.Delivery.Port,
                        ["payload"] = Convert.ToHexString(data.Delivery.Payload).ToLowerInvariant(),
                        ["acknowledged"] = data.Delivery.Acknowledged,
                        ["confirmed"] = data.Delivery.Confirmed,
                        ["morePending"] = data.Delivery.MorePending,
                        ["linkCheck"] = data.Delivery.LinkCheck is { } check
                            ? new Dictionary<string, object> { ["marginDb"] = check.MarginDb, ["gateways"] = check.Gateways }
                            : null,
                        ["deviceTime"] = data.Delivery.DeviceTime is { } time
                            ? new Dictionary<string, object> { ["gpsSeconds"] = time.GpsSeconds, ["fraction"] = time.Fraction }
                            : null,
                    },
                },
                _ => new Dictionary<string, object> { ["kind"] = "joined", ["devAddr"] = heard.DevAddr },
            };
        case "useRelay":
            return device.UseRelay(step.GetProperty("on").GetBoolean());
        case "heardWorAck":
            return RelayStatus(device.HeardWorAck(
                Convert.FromHexString(step.GetProperty("frame").GetString()!)));
        case "noWorAck":
            LorawanWorNext worNext = device.NoWorAck(step.GetProperty("nowUs").GetUInt64());
            return new Dictionary<string, object?>
            {
                ["uplink"] = worNext.Uplink,
                ["wakeUp"] = worNext.WakeUp is { } again ? Exchange(again) : null,
            };
        case "relayMode":
            return new Dictionary<string, object?>
            {
                ["relaying"] = device.Relaying,
                ["activation"] = SnakeCase(device.RelayActivation.ToString()),
                ["sync"] = SnakeCase(device.RelaySync.ToString()),
                ["worCounter"] = device.WorCounter,
                ["status"] = device.RelayStatus is { } said ? RelayStatus(said) : null,
            };
        case "nothingHeard":
            LorawanNext next = device.NothingHeard(step.GetProperty("nowUs").GetUInt64());
            return new Dictionary<string, object?>
            {
                ["kind"] = SnakeCase(next.Kind.ToString()),
                ["notBeforeUs"] = next.NotBeforeMicros,
            };
        case "save":
            return Convert.ToHexString(device.Save(step.GetProperty("nowUs").GetUInt64())).ToLowerInvariant();
        case "resume":
            device.Resume(
                Convert.FromHexString(step.GetProperty("saved").GetString()!),
                step.GetProperty("nowUs").GetUInt64());
            return true;
        case "requestLinkCheck":
            device.RequestLinkCheck();
            return null;
        case "requestDeviceTime":
            device.RequestDeviceTime();
            return null;
        case "setBattery":
            JsonElement battery = step.GetProperty("battery");
            device.SetBattery(battery.ValueKind switch
            {
                JsonValueKind.Number => LorawanBattery.Level(battery.GetByte()),
                _ when battery.GetString() == "external" => LorawanBattery.External,
                _ => LorawanBattery.Unknown,
            });
            return null;
        case "status":
            (uint lowestHz, uint highestHz) = device.FrequencySpan;
            return new Dictionary<string, object?>
            {
                ["joined"] = device.IsJoined,
                ["devAddr"] = device.DevAddr,
                ["dataRate"] = device.DataRate,
                ["fcntUp"] = device.FcntUp,
                ["fcntDown"] = device.FcntDown,
                ["transmissions"] = device.Transmissions,
                ["rx2"] = new Dictionary<string, object> { ["frequencyHz"] = device.Rx2.FrequencyHz, ["dataRate"] = device.Rx2.DataRate },
                ["receiveDelayUs"] = device.ReceiveDelayMicros,
                ["frequencySpan"] = new Dictionary<string, object> { ["lowestHz"] = lowestHz, ["highestHz"] = highestHz },
                ["channels"] = device.Channels().Select(channel => new Dictionary<string, object>
                {
                    ["index"] = channel.Index,
                    ["uplinkHz"] = channel.UplinkHz,
                    ["downlinkHz"] = channel.DownlinkHz,
                    ["minDataRate"] = channel.MinDataRate,
                    ["maxDataRate"] = channel.MaxDataRate,
                }).ToList(),
            };
        default:
            throw new InvalidOperationException("an unknown step");
    }
}

// Renders a JSON value with its object keys sorted, so two documents compare by content.
static string Canonical(JsonElement element) => element.ValueKind switch
{
    JsonValueKind.Object => "{" + string.Join(",", element.EnumerateObject()
        .OrderBy(property => property.Name, StringComparer.Ordinal)
        .Select(property => JsonSerializer.Serialize(property.Name) + ":" + Canonical(property.Value))) + "}",
    JsonValueKind.Array => "[" + string.Join(",", element.EnumerateArray().Select(Canonical)) + "]",
    JsonValueKind.Number => element.GetDecimal().ToString(System.Globalization.CultureInfo.InvariantCulture),
    _ => element.GetRawText(),
};

// Converts a PascalCase name to the snake_case the vectors use.
static string SnakeCase(string name) =>
    string.Concat(name.Select((letter, at) =>
        char.IsUpper(letter) ? (at > 0 ? "_" : "") + char.ToLowerInvariant(letter) : letter.ToString()));

// Holds a channel list to the answers every binding must give.
static void ConformCfList(LorawanCfList list, JsonElement want, string where)
{
    Assert(
        Convert.ToHexString(list.Bytes).Equals(want.GetProperty("bytes").GetString(), StringComparison.OrdinalIgnoreCase),
        $"the bytes of {where}");
    string kind = list.Kind switch
    {
        LorawanCfListKind.Frequencies => "frequencies",
        LorawanCfListKind.ChannelMasks => "channel_masks",
        _ => "reserved",
    };
    Assert(kind == want.GetProperty("kind").GetString(), $"the kind of {where}");
    Assert(list.TypeByte == want.GetProperty("typeByte").GetByte(), $"the type byte of {where}");

    JsonElement wantFrequencies = want.GetProperty("frequenciesHz");
    uint[]? frequencies = list.FrequenciesHz();
    Assert(
        wantFrequencies.ValueKind == JsonValueKind.Null
            ? frequencies is null
            : frequencies is not null
                && frequencies.SequenceEqual(wantFrequencies.EnumerateArray().Select(hz => hz.GetUInt32())),
        $"the frequencies of {where}");

    JsonElement wantGroups = want.GetProperty("channelMaskGroups");
    ushort[]? groups = list.ChannelMaskGroups();
    Assert(
        wantGroups.ValueKind == JsonValueKind.Null
            ? groups is null
            : groups is not null
                && groups.SequenceEqual(wantGroups.EnumerateArray().Select(mask => mask.GetUInt16())),
        $"the masks of {where}");

    Assert(
        list.EnabledChannels().SequenceEqual(
            want.GetProperty("enabledChannels").EnumerateArray().Select(channel => channel.GetInt32())),
        $"the channels of {where}");
    foreach (JsonElement entry in want.GetProperty("enables").EnumerateArray())
    {
        int channel = entry.GetProperty("channel").GetInt32();
        JsonElement enabled = entry.GetProperty("enabled");
        bool? got = list.Enables(channel);
        Assert(
            enabled.ValueKind == JsonValueKind.Null ? got is null : got == enabled.GetBoolean(),
            $"channel {channel} of {where}");
    }

    Assert(LorawanCfList.FromBytes(list.Bytes).Equals(list), $"{where} reads back from its bytes");
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

    Audit.VerifyChain(keeper.PublicKey, entries);

    using AuditEntry tampered = AuditEntry.FromBytes(
        Convert.FromHexString(vector.GetProperty("tampered").GetString()!));
    Refuses(
        () => Audit.VerifyChain(keeper.PublicKey, [entries[0], entries[1], tampered]),
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

/// <summary>A link over two queues, standing in for a vendor SDK.</summary>
static string HexLower(byte[] bytes) => Convert.ToHexString(bytes).ToLowerInvariant();

static void Gateways()
{
    // A datagram this protocol does not define is refused, and one it does round trips.
    try
    {
        Gateway.Parse([1, 0, 1, 0]);
        Fail("protocol version 1 is not this protocol");
    }
    catch (PamojaException)
    {
    }

    byte[] datagram = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullData, 0x0102)
    {
        GatewayEui = "b827ebfffe010203",
    });
    Assert(datagram.Length == 12, "a PULL_DATA is twelve bytes");
    Assert(
        Gateway.Parse(datagram).GatewayEui == "b827ebfffe010203",
        "and carries the gateway's identifier");
    Assert(
        Gateway.Encode(Gateway.Acknowledgment(Gateway.Parse(datagram))!).Length == 4,
        "whose acknowledgment is four bytes");
}

static void GatewayNetworks()
{
    // The network side of a site admits a device, answers its join, and reads what it sends.
    byte[] devEui = new byte[8];
    Array.Fill(devEui, (byte)0x11);
    byte[] appEui = new byte[8];
    Array.Fill(appEui, (byte)0x22);
    byte[] appKey = new byte[16];
    Array.Fill(appKey, (byte)0x33);

    using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
    using var site = new GatewayNetwork(plan, 0x00002A, firstDevAddr: 0x26010001);
    site.Register(devEui, appEui, appKey);

    using var joiner = new LorawanDevice(devEui, appEui, appKey);
    GatewayNetworkEvent joined = site.Uplink(
        new GatewayRxpk(868_100_000, joiner.JoinRequest(0x0102))
        {
            Link = new LoraLink(7, 125_000),
            TimestampMicros = 1_000_000,
        });
    Assert(joined.Outcome == GatewayNetworkOutcome.Joined, "a join request is admitted");
    Assert(joined.DevAddr == 0x26010001, "and granted the first address");
    Assert(
        joined.Accept!.TimestampMicros == 6_000_000,
        "whose accept goes out five seconds later");
    Assert(joined.Accept!.InvertPolarity, "with the polarity a device listens for");

    using LorawanJoinAccept granted = joiner.AcceptJoin(joined.Accept!.Payload, 0x0102);
    using LorawanSession activated = granted.Session();
    GatewayNetworkEvent carried = site.Uplink(
        new GatewayRxpk(868_100_000, activated.EncodeUplink(0, 2, "21.5"u8))
        {
            Link = new LoraLink(7, 125_000),
            TimestampMicros = 9_000_000,
        });
    Assert(carried.Outcome == GatewayNetworkOutcome.Data, "a session frame is read");
    Assert(
        Encoding.UTF8.GetString(carried.Payload!) == "21.5",
        "and decrypted into what the node sent");
    Assert(carried.Slot!.TimestampUs == 10_000_000, "one second after the uplink");

    GatewayTxpk answered = site.Answer(carried.DevAddr, carried.Slot!, 2, "ok"u8);
    Assert(answered.InvertPolarity, "the answer is inverted too");
}

// A device out of a gateway's reach, reaching it through the relay next door.
static void RelayedReach()
{
    byte[] relayEui = new byte[8];
    Array.Fill(relayEui, (byte)0x41);
    byte[] sensorEui = new byte[8];
    Array.Fill(sensorEui, (byte)0x42);
    byte[] joinEui = new byte[8];
    Array.Fill(joinEui, (byte)0x22);
    byte[] appKey = new byte[16];
    Array.Fill(appKey, (byte)0x33);

    using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
    using var site = new GatewayNetwork(plan, 0x00002A, firstDevAddr: 0x26010001);
    site.Register(relayEui, joinEui, appKey);
    site.Register(sensorEui, joinEui, appKey);
    LorawanDeviceSettings settings = new(2, 14) { LowestHz = 863_000_000, HighestHz = 870_000_000 };

    // Both join the network the ordinary way, the relay first.
    using var relayCredentials = new LorawanDevice(relayEui, joinEui, appKey);
    using LorawanRelayNode node = LorawanRelayNode.OverTheAir(plan, relayCredentials, settings);
    LorawanTransmission relayJoin = node.Join(0x0101, 1_000_000);
    GatewayNetworkEvent relayAccept = site.Uplink(
        new GatewayRxpk(relayJoin.FrequencyHz, relayJoin.Frame)
        {
            Link = relayJoin.Link,
            TimestampMicros = 1_000_000,
        });
    Assert(relayAccept.Outcome == GatewayNetworkOutcome.Joined, "the relay joins like any device");
    node.HeardIn(LorawanReceiveWindow.Rx1, relayAccept.Accept!.Payload, 5);
    Assert(node.Joined, "and holds the session it was granted");

    using var sensorCredentials = new LorawanDevice(sensorEui, joinEui, appKey);
    using LorawanEndDevice sensor = LorawanEndDevice.OverTheAir(plan, sensorCredentials, settings);
    LorawanTransmission sensorJoin = sensor.Join(0x0102, 20_000_000);
    GatewayNetworkEvent sensorAccept = site.Uplink(
        new GatewayRxpk(sensorJoin.FrequencyHz, sensorJoin.Frame)
        {
            Link = sensorJoin.Link,
            TimestampMicros = 20_000_000,
        });
    sensor.Heard(sensorAccept.Accept!.Payload, 5, LorawanReceiveWindow.Rx1);
    uint sensorAddr = sensor.DevAddr!.Value;

    // The network hands the relay the key that lets it verify the sensor's wake-up frames.
    LorawanMacCommand trust = site.TrustCommand(sensorAddr, 0);
    LorawanTransmission relayUplink = node.SendEmpty(40_000_000);
    GatewayNetworkEvent heardRelay = site.Uplink(
        new GatewayRxpk(relayUplink.FrequencyHz, relayUplink.Frame)
        {
            Link = relayUplink.Link,
            TimestampMicros = 40_000_000,
        });
    GatewayTxpk configure = site.Command(node.DevAddr!.Value, heardRelay.Slot!, [trust]);
    Assert(
        node.HeardIn(LorawanReceiveWindow.Rx1, configure.Payload, 5) is LorawanRelayHeard.Device,
        "the relay reads its own configuration");

    node.Start(LorawanCadPeriodicity.Ms1000, 0);
    Assert(node.Running, "and starts listening for the devices around it");
    LorawanScan scan = node.NextScan(60_000_000)!;
    Assert(scan.Channel == LorawanWorChannel.Default, "on its default channel");

    // The sensor sends through the relay: a wake-up frame first, then the uplink itself.
    Assert(sensor.UseRelay(true), "the sensor decides to use a relay");
    Assert(sensor.RelaySync == LorawanRelaySync.Initialized, "knowing nothing of it yet");
    LorawanTransmission reading = sensor.Send(2, "21.5"u8, 61_000_000);
    Assert(reading.Relay is not null, "so the uplink goes out behind a wake-up frame");
    Assert(reading.Relay!.WakeUp.Frame.Length == 15, "fifteen bytes ahead of an uplink");

    LorawanWake woke = node.HeardWor(scan, reading.Relay!.WakeUp.Frame, -90, 4, scan.StartMicros + 500_000);
    Assert(woke is LorawanWake.Uplink, "the relay knows the device");
    LorawanWake.Uplink heardWor = (LorawanWake.Uplink)woke;
    Assert(heardWor.DevAddr == sensorAddr, "and which one it is");
    Assert(heardWor.Forward == LorawanRelayForward.Available, "and has room to forward");

    LorawanRelayStatus status = sensor.HeardWorAck(heardWor.Acknowledgment!.Frame);
    Assert(status.CadPeriodicity == LorawanCadPeriodicity.Ms1000, "the relay says how often it scans");
    Assert(
        sensor.RelaySync == LorawanRelaySync.Synchronized,
        "so the next wake-up frame needs only a short preamble");

    ulong dueUs = node.HeardUplink(reading.Frame, -88, 6, heardWor.Listen!.StartMicros + 100_000);
    Assert(node.ForwardDue == dueUs, "the uplink waits fifty milliseconds to be forwarded");
    LorawanTransmission forwarded = node.Forward(dueUs);
    GatewayNetworkEvent carried = site.Uplink(
        new GatewayRxpk(forwarded.FrequencyHz, forwarded.Frame)
        {
            Link = forwarded.Link,
            TimestampMicros = (uint)dueUs,
        });
    Assert(carried.Outcome == GatewayNetworkOutcome.Data, "the network reads the sensor's frame");
    Assert(carried.DevAddr == sensorAddr, "as the sensor's own");
    Assert(Encoding.UTF8.GetString(carried.Payload!) == "21.5", "with the reading it sent");
    Assert(carried.Relay!.Relay == node.DevAddr!.Value, "and says which relay carried it");
    Assert(carried.Relay!.WorChannel == 0, "on which channel");

    // The answer goes back the same way, into the window the sensor keeps for a relay.
    GatewayTxpk answer = site.Answer(sensorAddr, carried.Slot!, 2, "ok"u8);
    LorawanRelayHeard passed = node.HeardIn(LorawanReceiveWindow.Rx1, answer.Payload, 5);
    Assert(passed is LorawanRelayHeard.Downlink, "the relay passes it on rather than reading it");
    LorawanRxrDownlink sent = ((LorawanRelayHeard.Downlink)passed).Forwarded;
    LorawanHeard delivered = sensor.Heard(sent.Frame, 5, LorawanReceiveWindow.Rxr);
    Assert(delivered is LorawanHeard.Data, "and the sensor hears it");
    Assert(
        Encoding.UTF8.GetString(((LorawanHeard.Data)delivered).Delivery.Payload) == "ok",
        "with what the network sent");

    // A device the relay was never told about is reported to the network instead.
    byte[] strangerKey = new byte[16];
    Array.Fill(strangerKey, (byte)0x77);
    byte[] stranger = LorawanRelay.WorUplink(
        LorawanRelay.WorKeys(strangerKey, 0x26010009),
        0x26010009,
        0,
        scan.Carrier,
        scan.Carrier);
    LorawanScan later = node.NextScan(dueUs + 60_000_000)!;
    Assert(
        node.HeardWor(later, stranger, -95, 2, later.StartMicros + 500_000) is LorawanWake.Notified,
        "a relay tells its network about a device it cannot verify");
}


// Holds ChirpStack uplink events to the answers every binding must give.
// A LoRaWAN relay: the root key The Things Stack tests with, the WOR frames and
// acknowledgments Basics Modem produces, a forwarded uplink, and TS011-1.0.1 appendix 1.
static void ConformLorawanRelay(JsonElement vector)
{
    static string Hex(byte[] bytes) => Convert.ToHexString(bytes).ToLowerInvariant();
    static byte[] Bytes(JsonElement element) => Convert.FromHexString(element.GetString()!);
    static LorawanCarrier Carrier(JsonElement element) => new(
        element.GetProperty("frequencyHz").GetUInt32(),
        element.GetProperty("dataRate").GetByte());
    static LorawanStateSync State(JsonElement element) => new(
        Enum.Parse<LorawanCadToRx>(element.GetProperty("cadToRx").GetString()!),
        Enum.Parse<LorawanRelayForward>(element.GetProperty("forward").GetString()!),
        element.GetProperty("relayDataRate").GetByte(),
        Enum.Parse<LorawanXtalAccuracy>(element.GetProperty("xtalAccuracy").GetString()!),
        Enum.Parse<LorawanCadPeriodicity>(element.GetProperty("cadPeriodicity").GetString()!),
        element.GetProperty("tOffsetMs").GetUInt16());
    static LorawanUplinkMetadata Metadata(JsonElement element) => new(
        Enum.Parse<LorawanWorChannel>(element.GetProperty("worChannel").GetString()!),
        element.GetProperty("rssiDbm").GetInt16(),
        element.GetProperty("snrDb").GetSByte(),
        element.GetProperty("dataRate").GetByte());

    JsonElement constants = vector.GetProperty("constants");
    Assert(LorawanRelay.FPort == constants.GetProperty("laFportRelay").GetByte(), "LA_FPORT_RELAY");
    Assert(LorawanRelay.TrustedEndDevices == constants.GetProperty("trustedEdNumber").GetInt32(), "TRUSTED_ED_NUMBER");
    Assert(LorawanRelay.WorAttemptsWithoutAck == constants.GetProperty("worAttemptsWoAck").GetByte(), "WOR_ATTEMPTS_WO_ACK");
    Assert(LorawanRelay.WorDataDelayMicros == constants.GetProperty("worDataDelayUs").GetUInt32(), "WOR_DATA_DELAY");
    Assert(LorawanRelay.WorAckDelayMicros == constants.GetProperty("worAckDelayUs").GetUInt32(), "WOR_ACK_DELAY");
    Assert(LorawanRelay.RelayForwardDelayMicros == constants.GetProperty("relayFwdDelayUs").GetUInt32(), "RELAY_FWD_DELAY");
    Assert(LorawanRelay.RxrDelayMicros == constants.GetProperty("rxrDelayUs").GetUInt32(), "RXR_DELAY");
    Assert(LorawanRelay.ForwardOverhead == constants.GetProperty("forwardOverhead").GetInt32(), "the forward overhead");
    Assert(LorawanRelay.MinWorPreambleSymbols == constants.GetProperty("minWorPreambleSymbols").GetUInt16(), "the shortest preamble");

    JsonElement rootKey = vector.GetProperty("rootKey");
    Assert(
        Hex(LorawanRelay.RootWorSKey(Bytes(rootKey.GetProperty("networkKey")))) == rootKey.GetProperty("rootWorSKey").GetString(),
        "The Things Stack's RootWorSKey vector");

    JsonElement own = vector.GetProperty("session");
    uint devAddr = own.GetProperty("devAddr").GetUInt32();
    using LorawanSession session = new(devAddr, Bytes(own.GetProperty("nwkSKey")), Bytes(own.GetProperty("appSKey")));
    Assert(Hex(session.RootWorSKey()) == own.GetProperty("rootWorSKey").GetString(), "the session's root key");
    LorawanWorKeys keys = session.WorKeys();
    Assert(Hex(keys.Integrity) == own.GetProperty("integrity").GetString(), "WorSIntKey");
    Assert(Hex(keys.Encryption) == own.GetProperty("encryption").GetString(), "WorSEncKey");
    Assert(LorawanRelay.WorKeys(Bytes(own.GetProperty("rootWorSKey")), devAddr) == keys, "the keys from the root key");

    foreach (JsonElement entry in vector.GetProperty("worUplinks").EnumerateArray())
    {
        uint wfcnt = entry.GetProperty("wfcnt").GetUInt32();
        LorawanCarrier uplink = Carrier(entry.GetProperty("uplink"));
        LorawanCarrier wor = Carrier(entry.GetProperty("wor"));
        byte[] frame = LorawanRelay.WorUplink(keys, devAddr, wfcnt, uplink, wor);
        Assert(Hex(frame) == entry.GetProperty("frame").GetString(), $"the WOR uplink at WFCnt {wfcnt}");
        Assert(
            LorawanRelay.ParseWor(frame) == new LorawanWor(LorawanWorKind.Uplink, null, devAddr, (ushort)wfcnt),
            "the WOR uplink reads back");
        Assert(LorawanRelay.OpenWor(frame, keys, wfcnt, wor) == uplink, "the WOR uplink opens");
        try
        {
            LorawanRelay.OpenWor(frame, keys, wfcnt + 0x10000, wor);
            Fail("a WOR uplink with the wrong upper counter bits must not open");
        }
        catch (PamojaException)
        {
        }
    }

    JsonElement join = vector.GetProperty("worJoinRequest");
    byte[] joinFrame = LorawanRelay.WorJoinRequest(Carrier(join.GetProperty("uplink")));
    Assert(Hex(joinFrame) == join.GetProperty("frame").GetString(), "the join request WOR");
    Assert(
        LorawanRelay.ParseWor(joinFrame) == new LorawanWor(LorawanWorKind.JoinRequest, Carrier(join.GetProperty("uplink")), null, null),
        "the join request WOR reads back");
    foreach (JsonElement refused in vector.GetProperty("refusedWors").EnumerateArray())
    {
        try
        {
            LorawanRelay.ParseWor(Bytes(refused));
            Fail($"{refused.GetString()} must be refused");
        }
        catch (PamojaException)
        {
        }
    }

    foreach (JsonElement entry in vector.GetProperty("worAcks").EnumerateArray())
    {
        uint wfcnt = entry.GetProperty("wfcnt").GetUInt32();
        LorawanCarrier ack = Carrier(entry.GetProperty("ack"));
        LorawanCarrier uplink = Carrier(entry.GetProperty("uplink"));
        LorawanStateSync state = State(entry.GetProperty("state"));
        byte[] frame = LorawanRelay.WorAck(keys, devAddr, wfcnt, ack, uplink, state);
        Assert(Hex(frame) == entry.GetProperty("frame").GetString(), $"the WOR ACK at WFCnt {wfcnt}");
        Assert(LorawanRelay.OpenWorAck(frame, keys, devAddr, wfcnt, ack, uplink) == state, "the WOR ACK reads back");
    }

    foreach (JsonElement entry in vector.GetProperty("forwarded").EnumerateArray())
    {
        byte[] payload = LorawanRelay.EncodeForward(new LorawanForwardedUplink(
            Metadata(entry.GetProperty("metadata")),
            entry.GetProperty("frequencyHz").GetUInt32(),
            Bytes(entry.GetProperty("phyPayload"))));
        Assert(Hex(payload) == entry.GetProperty("payload").GetString(), "a forwarded uplink");
        LorawanForwardedUplink read = LorawanRelay.ParseForward(payload);
        JsonElement back = entry.TryGetProperty("readBack", out JsonElement readBack) ? readBack : entry.GetProperty("metadata");
        Assert(read.Metadata == Metadata(back), "the forwarded metadata reads back");
        Assert(read.FrequencyHz == entry.GetProperty("frequencyHz").GetUInt32(), "the forwarded frequency");
        Assert(Hex(read.PhyPayload) == entry.GetProperty("phyPayload").GetString(), "the forwarded frame");
    }

    foreach (JsonElement entry in vector.GetProperty("unsynchronizedPreambles").EnumerateArray())
    {
        Assert(
            LorawanRelay.UnsynchronizedPreamble(
                Enum.Parse<LorawanCadPeriodicity>(entry.GetProperty("cadPeriodicity").GetString()!),
                entry.GetProperty("symbolUs").GetUInt64(),
                Enum.Parse<LorawanCadToRx>(entry.GetProperty("cadToRx").GetString()!))
                == entry.GetProperty("symbols").GetUInt16(),
            "an unsynchronized preamble");
    }
    foreach (JsonElement entry in vector.GetProperty("tOffsets").EnumerateArray())
    {
        ushort? offset = LorawanRelay.TOffsetMs(
            entry.GetProperty("scanStartUs").GetUInt64(),
            entry.GetProperty("preambleEndUs").GetUInt64());
        JsonElement want = entry.GetProperty("offsetMs");
        Assert(
            want.ValueKind == JsonValueKind.Null ? offset is null : offset == want.GetUInt16(),
            "a WOR ACK offset");
    }

    JsonElement timing = vector.GetProperty("synchronization");
    LorawanSynchronization sync = LorawanRelay.Synchronization(
        timing.GetProperty("worStartUs").GetUInt64(),
        timing.GetProperty("preambleSymbols").GetUInt16(),
        timing.GetProperty("symbolUs").GetUInt64(),
        State(timing.GetProperty("state")));
    Assert(sync.ReferenceMicros == timing.GetProperty("referenceUs").GetUInt64(), "TREF of the appendix example");
    foreach (JsonElement entry in timing.GetProperty("slots").EnumerateArray())
    {
        LorawanWorSlot? slot = LorawanRelay.NextWor(
            sync,
            entry.GetProperty("nowUs").GetUInt64(),
            entry.GetProperty("deviceXtalPpm").GetUInt32(),
            entry.GetProperty("symbolUs").GetUInt64(),
            entry.GetProperty("otherChannel").GetBoolean());
        JsonElement want = entry.GetProperty("slot");
        Assert(
            want.ValueKind == JsonValueKind.Null
                ? slot is null
                : slot == new LorawanWorSlot(want.GetProperty("startUs").GetUInt64(), want.GetProperty("preambleSymbols").GetUInt16()),
            $"the slot after {entry.GetProperty("nowUs").GetUInt64()}");
    }

    foreach (JsonElement entry in vector.GetProperty("secondChannels").EnumerateArray())
    {
        LoraRelayChannel? channel = LorawanRelay.SecondChannel(
            entry.GetProperty("index").GetByte(),
            entry.GetProperty("dataRate").GetByte(),
            entry.GetProperty("ackOffset").GetByte(),
            entry.GetProperty("frequencyHz").GetUInt32());
        JsonElement want = entry.GetProperty("channel");
        Assert(
            want.ValueKind == JsonValueKind.Null
                ? channel is null
                : channel == new LoraRelayChannel(
                    want.GetProperty("worFrequencyHz").GetUInt32(),
                    want.GetProperty("ackFrequencyHz").GetUInt32(),
                    want.GetProperty("dataRate").GetByte()),
            "a second channel");
    }
}

static void ConformChirpstack(JsonElement vector)
{
    foreach (JsonElement entry in vector.GetProperty("events").EnumerateArray())
    {
        ChirpstackUplinkEvent got = ChirpstackUplinkEvent.FromJson(entry.GetProperty("json").GetString()!);
        JsonElement want = entry.GetProperty("event");
        string where = $"the ChirpStack event {got.DevEui}";
        Assert(got.DeduplicationId == want.GetProperty("deduplicationId").GetString(), where);
        JsonElement time = want.GetProperty("time");
        Assert(got.Time == (time.ValueKind == JsonValueKind.Null ? null : time.GetString()), where);
        Assert(got.ApplicationId == want.GetProperty("applicationId").GetString(), where);
        Assert(got.DeviceName == want.GetProperty("deviceName").GetString(), where);
        Assert(got.DevEui == want.GetProperty("devEui").GetString(), where);
        ConformOptionalUint(got.DevAddr, want.GetProperty("devAddr"), where);
        Assert(got.Adr == want.GetProperty("adr").GetBoolean(), where);
        Assert(got.DataRate == want.GetProperty("dataRate").GetByte(), where);
        Assert(got.Fcnt == want.GetProperty("fcnt").GetUInt32(), where);
        ConformOptionalByte(got.Fport, want.GetProperty("fport"), where);
        Assert(got.Confirmed == want.GetProperty("confirmed").GetBoolean(), where);
        Assert(Convert.ToHexString(got.Data).Equals(want.GetProperty("data").GetString(), StringComparison.OrdinalIgnoreCase), where);
        ConformOptionalUint(got.FrequencyHz, want.GetProperty("frequencyHz"), where);
        JsonElement receptions = want.GetProperty("receptions");
        Assert(got.Receptions.Count == receptions.GetArrayLength(), where);
        int index = 0;
        foreach (JsonElement reception in receptions.EnumerateArray())
        {
            Assert(got.Receptions[index].Gateway == reception.GetProperty("gateway").GetString(), where);
            Assert(got.Receptions[index].RssiDbm == reception.GetProperty("rssiDbm").GetInt32(), where);
            Assert(got.Receptions[index].SnrDb == reception.GetProperty("snrDb").GetDouble(), where);
            index++;
        }

        JsonElement best = want.GetProperty("bestReception");
        Assert(got.BestReception == (best.ValueKind == JsonValueKind.Null ? null : best.GetInt32()), where);
    }

    foreach (JsonElement refused in vector.GetProperty("refused").EnumerateArray())
    {
        try
        {
            ChirpstackUplinkEvent.FromJson(refused.GetString()!);
            Fail($"{refused.GetString()} must be refused");
        }
        catch (PamojaException)
        {
        }
    }

    JsonElement topic = vector.GetProperty("topic");
    Assert(
        ChirpstackUplinkEvent.UplinkTopic(topic.GetProperty("applicationId").GetString()!) == topic.GetProperty("topic").GetString(),
        "the ChirpStack uplink topic");
    Assert(
        ChirpstackUplinkEvent.AllApplicationsTopic == vector.GetProperty("allApplications").GetString()
            && ChirpstackUplinkEvent.UplinkTopic("+") == ChirpstackUplinkEvent.AllApplicationsTopic,
        "every application's uplink topic");
}

static void ConformStation(JsonElement vector)
{
    string router = vector.GetProperty("router").GetString()!;
    byte[] identifier = Convert.FromHexString(router);
    var levels = new GatewayStationLevels(0, 1_000_000) { Rssi = -35.0, Snr = 5.1 };
    byte dataRate = vector.GetProperty("dataRate").GetByte();
    uint frequencyHz = vector.GetProperty("frequencyHz").GetUInt32();

    // The station names itself the way the protocol prefers, and reads any form back.
    Assert(
        GatewayStation.Id6(identifier) == vector.GetProperty("routerId6").GetString(),
        "the station in ID6");
    Assert(
        HexLower(GatewayStation.EuiOf(vector.GetProperty("routerId6").GetString()!)) == router,
        "the ID6 read back");
    Assert(
        GatewayStation.Discovery(identifier) == vector.GetProperty("discovery").GetString(),
        "the discovery request");

    GatewayStationRouter routed = GatewayStation.RouterParse(
        vector.GetProperty("routerAnswer").GetString()!);
    Assert(routed.Uri is not null, "an accepted station is sent somewhere");

    // A join request the radio heard, split into the fields the protocol names.
    JsonElement wanted = vector.GetProperty("join");
    GatewayStationMessage join = GatewayStation.Heard(
        Convert.FromHexString(wanted.GetProperty("frame").GetString()!),
        dataRate,
        frequencyHz,
        levels);
    Assert(join.Kind == GatewayStationKind.JoinRequest, "a join request is read as one");
    Assert(
        HexLower(join.JoinEui!) == wanted.GetProperty("joinEui").GetString(),
        "the application it joins");
    Assert(
        HexLower(join.DevEui!) == wanted.GetProperty("devEui").GetString(),
        "the device asking");
    Assert(join.DevNonce == wanted.GetProperty("devNonce").GetUInt16(), "the nonce it used");
    Assert(join.Mic == wanted.GetProperty("mic").GetInt32(), "its integrity code");
    Assert(join.Json == wanted.GetProperty("message").GetString(), "the jreq it sends");

    // Then a data frame, whose payload stays encrypted as it passes through.
    JsonElement carried = vector.GetProperty("uplink");
    GatewayStationMessage uplink = GatewayStation.Heard(
        Convert.FromHexString(carried.GetProperty("frame").GetString()!),
        dataRate,
        frequencyHz,
        levels);
    Assert(uplink.Kind == GatewayStationKind.Uplink, "a data frame is read as one");
    Assert(
        uplink.DevAddr == carried.GetProperty("devAddr").GetInt32(),
        "the address it came from");
    Assert(uplink.Fcnt == carried.GetProperty("fcnt").GetUInt16(), "the counter it carried");
    Assert(uplink.Fport == carried.GetProperty("fport").GetByte(), "the port it was sent on");
    Assert(
        HexLower(uplink.Payload) == carried.GetProperty("payload").GetString(),
        "the payload, still encrypted");
    Assert(uplink.Mic == carried.GetProperty("mic").GetInt32(), "its integrity code");
    Assert(uplink.Json == carried.GetProperty("message").GetString(), "the updf it sends");

    // What arrives on the websocket reads back into the same fields.
    GatewayStationMessage read = GatewayStation.Parse(
        carried.GetProperty("message").GetString()!);
    Assert(read.DevAddr == uplink.DevAddr, "the address read back");
    Assert(read.Fcnt == uplink.Fcnt, "the counter read back");
}

static void ConformGatewayNetwork(JsonElement vector)
{
    byte[] devEui = Convert.FromHexString(vector.GetProperty("devEui").GetString()!);
    byte[] appEui = Convert.FromHexString(vector.GetProperty("appEui").GetString()!);
    byte[] appKey = Convert.FromHexString(vector.GetProperty("appKey").GetString()!);
    ushort devNonce = vector.GetProperty("devNonce").GetUInt16();
    uint devAddr = vector.GetProperty("devAddr").GetUInt32();
    uint frequencyHz = vector.GetProperty("frequencyHz").GetUInt32();
    var dr = new LoraLink(
        vector.GetProperty("spreadingFactor").GetByte(),
        vector.GetProperty("bandwidthHz").GetUInt32());

    JsonElement join = vector.GetProperty("join");
    JsonElement uplink = vector.GetProperty("uplink");
    JsonElement downlink = vector.GetProperty("downlink");

    using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
    using var site = new GatewayNetwork(
        plan, vector.GetProperty("netId").GetUInt32(), firstDevAddr: devAddr);
    site.Register(devEui, appEui, appKey);

    using var joiner = new LorawanDevice(devEui, appEui, appKey);
    byte[] request = joiner.JoinRequest(devNonce);
    Assert(
        HexLower(request) == join.GetProperty("request").GetString(),
        "the join request the device sends");

    GatewayNetworkEvent joined = site.Uplink(
        new GatewayRxpk(frequencyHz, request)
        {
            Link = dr,
            TimestampMicros = join.GetProperty("heardAtUs").GetUInt32(),
        });
    Assert(joined.Outcome == GatewayNetworkOutcome.Joined, "a join request is admitted");
    Assert(joined.DevAddr == devAddr, "the address granted");
    Assert(
        HexLower(joined.Accept!.Payload) == join.GetProperty("accept").GetString(),
        "the accept it answers with");
    Assert(
        joined.Accept!.TimestampMicros == join.GetProperty("timestampUs").GetUInt32(),
        "the join window it goes out in");

    using LorawanJoinAccept granted = joiner.AcceptJoin(joined.Accept!.Payload, devNonce);
    using LorawanSession activated = granted.Session();
    byte[] sent = activated.EncodeUplink(
        uplink.GetProperty("fcnt").GetUInt32(),
        uplink.GetProperty("fport").GetByte(),
        Encoding.UTF8.GetBytes(uplink.GetProperty("payload").GetString()!));
    Assert(
        HexLower(sent) == uplink.GetProperty("frame").GetString(),
        "the frame the device sends");

    GatewayNetworkEvent carried = site.Uplink(
        new GatewayRxpk(frequencyHz, sent)
        {
            Link = dr,
            TimestampMicros = uplink.GetProperty("heardAtUs").GetUInt32(),
        });
    Assert(carried.Outcome == GatewayNetworkOutcome.Data, "a session frame is read");
    Assert(
        Encoding.UTF8.GetString(carried.Payload!) == uplink.GetProperty("payload").GetString(),
        "what the node sent");
    Assert(
        carried.Slot!.TimestampUs == uplink.GetProperty("slotTimestampUs").GetUInt32(),
        "the window its answer goes in");

    GatewayTxpk answer = site.Answer(
        carried.DevAddr, carried.Slot!, uplink.GetProperty("fport").GetByte(), "ok"u8);
    Assert(
        HexLower(answer.Payload) == downlink.GetProperty("frame").GetString(),
        "the downlink frame");
    Assert(
        answer.TimestampMicros == downlink.GetProperty("timestampUs").GetUInt32(),
        "when it transmits");
}

static void ConformGateway(JsonElement vector)
{
    string identifier = vector.GetProperty("gateway").GetString()!;
    JsonElement heard = vector.GetProperty("pushData").GetProperty("rxpk");
    JsonElement reported = vector.GetProperty("pushData").GetProperty("stat");
    var link = new LoraLink(
        heard.GetProperty("spreadingFactor").GetByte(),
        heard.GetProperty("bandwidthHz").GetUInt32())
        .WithCodingRate(heard.GetProperty("codingRateDenominator").GetByte());

    byte[] push = Gateway.Encode(new GatewayPacket(
        GatewayPacketKind.PushData,
        vector.GetProperty("pushData").GetProperty("token").GetUInt16())
    {
        GatewayEui = identifier,
        Packets =
        [
            new GatewayRxpk(
                heard.GetProperty("frequencyHz").GetUInt32(),
                Encoding.UTF8.GetBytes(heard.GetProperty("payload").GetString()!))
            {
                Link = link,
                RssiDbm = heard.GetProperty("rssiDbm").GetDouble(),
                SnrDb = heard.GetProperty("snrDb").GetDouble(),
                TimestampMicros = heard.GetProperty("timestampUs").GetUInt32(),
                ReceivedAtMicros = heard.GetProperty("receivedAtUs").GetUInt64(),
                Channel = heard.GetProperty("channel").GetByte(),
            },
        ],
        Status = new GatewayStat
        {
            TimeSeconds = reported.GetProperty("timeS").GetUInt64(),
            LatitudeDeg = reported.GetProperty("latitudeDeg").GetDouble(),
            LongitudeDeg = reported.GetProperty("longitudeDeg").GetDouble(),
            AltitudeM = reported.GetProperty("altitudeM").GetInt32(),
            Received = reported.GetProperty("received").GetUInt32(),
            ReceivedOk = reported.GetProperty("receivedOk").GetUInt32(),
            Forwarded = reported.GetProperty("forwarded").GetUInt32(),
            AcknowledgedPercent = reported.GetProperty("acknowledgedPercent").GetDouble(),
            Downlinks = reported.GetProperty("downlinks").GetUInt32(),
            Transmitted = reported.GetProperty("transmitted").GetUInt32(),
        },
    });
    Assert(
        HexLower(push) == vector.GetProperty("pushData").GetProperty("datagram").GetString(),
        "the PUSH_DATA datagram");

    GatewayPacket read = Gateway.Parse(push);
    Assert(read.GatewayEui == identifier, "the gateway identifier");
    Assert(
        read.Packets[0].FrequencyHz == heard.GetProperty("frequencyHz").GetUInt32(),
        "the carrier in hertz");
    Assert(
        Encoding.UTF8.GetString(read.Packets[0].Payload) == heard.GetProperty("payload").GetString(),
        "the payload in bytes");
    Assert(read.Status!.AltitudeM == reported.GetProperty("altitudeM").GetInt32(), "the altitude");
    Assert(
        HexLower(Gateway.Encode(Gateway.Acknowledgment(read)!))
            == vector.GetProperty("pushAck").GetString(),
        "the PUSH_ACK datagram");

    Assert(
        HexLower(Gateway.Encode(
            new GatewayPacket(GatewayPacketKind.PullData, 0x0304) { GatewayEui = identifier }))
            == vector.GetProperty("pullData").GetString(),
        "the PULL_DATA datagram");
    Assert(
        HexLower(Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullAck, 0x0304)))
            == vector.GetProperty("pullAck").GetString(),
        "the PULL_ACK datagram");

    JsonElement asked = vector.GetProperty("pullResp").GetProperty("txpk");
    byte[] downlink = Gateway.Encode(new GatewayPacket(
        GatewayPacketKind.PullResp,
        vector.GetProperty("pullResp").GetProperty("token").GetUInt16())
    {
        Transmit = new GatewayTxpk(
            asked.GetProperty("frequencyHz").GetUInt32(),
            Encoding.UTF8.GetBytes(asked.GetProperty("payload").GetString()!))
        {
            Link = new LoraLink(
                asked.GetProperty("spreadingFactor").GetByte(),
                asked.GetProperty("bandwidthHz").GetUInt32()),
            TimestampMicros = asked.GetProperty("timestampUs").GetUInt32(),
            PowerDbm = asked.GetProperty("powerDbm").GetSByte(),
            InvertPolarity = asked.GetProperty("invertPolarity").GetBoolean(),
            WithoutCrc = asked.GetProperty("withoutCrc").GetBoolean(),
        },
    });
    Assert(
        HexLower(downlink) == vector.GetProperty("pullResp").GetProperty("datagram").GetString(),
        "the PULL_RESP datagram");
    Assert(
        Gateway.Parse(downlink).Transmit!.PowerDbm == asked.GetProperty("powerDbm").GetSByte(),
        "the transmit power");

    string refusal = vector.GetProperty("txAck").GetProperty("status").GetString()!;
    byte[] refused = Gateway.Encode(new GatewayPacket(
        GatewayPacketKind.TxAck,
        vector.GetProperty("txAck").GetProperty("token").GetUInt16())
    {
        GatewayEui = identifier,
        TxStatus = GatewayTxStatus.CollisionPacket,
    });
    Assert(
        HexLower(refused) == vector.GetProperty("txAck").GetProperty("datagram").GetString(),
        "the TX_ACK datagram");
    Assert(Gateway.NameOf(Gateway.Parse(refused).TxStatus!.Value) == refusal, "the refusal");

    int code = 0;
    foreach (JsonElement status in vector.GetProperty("statuses").EnumerateArray())
    {
        Assert(
            Gateway.NameOf((GatewayTxStatus)code) == status.GetString(),
            $"the {status.GetString()} status");
        code++;
    }
}

// The four application layer packages: the key chains, the parity matrix, a whole
// fragmentation session, and one command of each kind read and written back.
static void ConformLorawanPackages(JsonElement vector)
{
    JsonElement ports = vector.GetProperty("ports");
    Assert(LorawanPackages.ClockPort == ports.GetProperty("clock").GetByte(), "the clock port");
    Assert(
        LorawanPackages.FragmentPort == ports.GetProperty("fragment").GetByte(),
        "the fragmentation port");
    Assert(
        LorawanPackages.MulticastPort == ports.GetProperty("multicast").GetByte(),
        "the multicast port");
    Assert(
        LorawanPackages.FirmwarePort == ports.GetProperty("firmware").GetByte(),
        "the firmware port");
    Assert(
        LorawanPackages.MaxFragments == vector.GetProperty("maxFragments").GetUInt16(),
        "the session ceiling");

    JsonElement mc = vector.GetProperty("multicast");
    byte[] rootKey = Unhex(mc, "rootKey");
    Assert(
        Hex(LorawanPackages.McRootKey(rootKey)) == mc.GetProperty("mcRootKey").GetString(),
        "the multicast root key of a 1.0.x device");
    Assert(
        Hex(LorawanPackages.McRootKey(rootKey, true)) == mc.GetProperty("mcRootKey11").GetString(),
        "and of a 1.1 device");
    byte[] keKey = LorawanPackages.McKeKey(Unhex(mc, "mcRootKey"));
    Assert(Hex(keKey) == mc.GetProperty("mcKeKey").GetString(), "the key encryption key");
    Assert(
        Hex(LorawanPackages.WrapMcKey(keKey, Unhex(mc, "groupKey")))
            == mc.GetProperty("wrapped").GetString(),
        "a group key wraps");
    Assert(
        Hex(LorawanPackages.McKey(keKey, Unhex(mc, "wrapped")))
            == mc.GetProperty("groupKey").GetString(),
        "and unwraps");
    uint mcAddr = mc.GetProperty("mcAddr").GetUInt32();
    Assert(
        Hex(LorawanPackages.McAppSKey(Unhex(mc, "groupKey"), mcAddr))
            == mc.GetProperty("mcAppSKey").GetString(),
        "the group's payload key");
    Assert(
        Hex(LorawanPackages.McNwkSKey(Unhex(mc, "groupKey"), mcAddr))
            == mc.GetProperty("mcNwkSKey").GetString(),
        "and its frame key");

    JsonElement frag = vector.GetProperty("fragment");
    foreach (JsonElement step in frag.GetProperty("prbs23").EnumerateArray())
    {
        Assert(
            LorawanPackages.FragPrbs23(step.GetProperty("x").GetUInt32())
                == step.GetProperty("next").GetUInt32(),
            "the pseudo-random sequence steps");
    }

    foreach (JsonElement line in frag.GetProperty("parityLines").EnumerateArray())
    {
        ushort[] want = line.GetProperty("fragments").EnumerateArray()
            .Select(entry => entry.GetUInt16()).ToArray();
        Assert(
            LorawanPackages.FragParityLine(
                line.GetProperty("coded").GetUInt16(),
                line.GetProperty("nbFrag").GetUInt16()).SequenceEqual(want),
            "the parity line of a coded fragment");
    }

    foreach (JsonElement shape in frag.GetProperty("sessions").EnumerateArray())
    {
        LorawanFragSession cut = LorawanPackages.FragSession(
            shape.GetProperty("blockLen").GetInt32(),
            shape.GetProperty("fragSize").GetByte());
        Assert(cut.NbFrag == shape.GetProperty("nbFrag").GetUInt16(), "how many fragments");
        Assert(cut.Padding == shape.GetProperty("padding").GetByte(), "and how much padding");
    }

    byte[] block = Unhex(frag, "block");
    foreach (JsonElement piece in frag.GetProperty("fragments").EnumerateArray())
    {
        Assert(
            Hex(LorawanPackages.FragFragment(block, 32, piece.GetProperty("n").GetUInt16()))
                == piece.GetProperty("bytes").GetString(),
            "one fragment of the session");
    }

    // The same session put back together over a link that drops every fourth fragment.
    JsonElement arrived = frag.GetProperty("received");
    ushort nbFrag = arrived.GetProperty("nbFrag").GetUInt16();
    using var receiver = new LorawanDefragmenter(nbFrag, 32, nbFrag);
    int sent = 0;
    int coded = 0;
    for (ushort n = 1; n <= nbFrag * 2; n++)
    {
        if (n % 4 == 0)
        {
            continue;
        }

        sent++;
        if (n > nbFrag)
        {
            coded++;
        }

        if (receiver.Fragment(n, LorawanPackages.FragFragment(block, 32, n)))
        {
            break;
        }
    }

    Assert(sent == arrived.GetProperty("sent").GetInt32(), "the same fragments arrive");
    Assert(coded == arrived.GetProperty("coded").GetInt32(), "the same number of them coded");
    Assert(
        Hex(receiver.Block().AsSpan(0, block.Length).ToArray()) == frag.GetProperty("block").GetString(),
        "and the block comes back whole");

    byte[] blockKey = LorawanPackages.DataBlockIntKey(rootKey);
    Assert(
        Hex(blockKey) == frag.GetProperty("dataBlockIntKey").GetString(),
        "the block signing key");
    JsonElement mic = frag.GetProperty("mic");
    using var code = new LorawanBlockMic(
        blockKey,
        mic.GetProperty("sessionCnt").GetUInt16(),
        mic.GetProperty("fragIndex").GetByte(),
        Encoding.ASCII.GetBytes(mic.GetProperty("descriptor").GetString()!),
        mic.GetProperty("blockLen").GetUInt32());
    code.Update(block);
    Assert(Hex(code.Finish()) == mic.GetProperty("bytes").GetString(), "the code over the block");

    foreach (JsonElement entry in vector.GetProperty("commands").EnumerateArray())
    {
        string text = entry.GetProperty("bytes").GetString()!;
        byte port = entry.GetProperty("port").GetByte();
        LorawanPackageCommand read = LorawanPackageCommand.Parse(
            port,
            entry.GetProperty("uplink").GetBoolean(),
            Convert.FromHexString(text));
        Assert(read.Cid == entry.GetProperty("cid").GetByte(), $"the identifier of {text}");
        Assert(Hex(read.Encode()) == text, $"{text} is written back the way it was read");
        foreach (JsonProperty field in entry.GetProperty("fields").EnumerateObject())
        {
            string name = char.ToUpperInvariant(field.Name[0]) + field.Name[1..];
            object? got = typeof(LorawanPackageCommand).GetProperty(name)!.GetValue(read);
            string gotText = got switch
            {
                byte[] bytes => Hex(bytes),
                bool flag => flag ? "true" : "false",
                _ => Convert.ToString(got, System.Globalization.CultureInfo.InvariantCulture)!,
            };
            string wantText = field.Value.ValueKind == JsonValueKind.String
                ? field.Value.GetString()!
                : field.Value.GetRawText();
            Assert(gotText == wantText, $"{field.Name} of {text}");
        }
    }

    foreach (JsonElement entry in vector.GetProperty("statusItems").EnumerateArray())
    {
        string text = entry.GetProperty("bytes").GetString()!;
        LorawanPackageCommand read = LorawanPackageCommand.StatusItem(Convert.FromHexString(text));
        Assert(read.McGroupId == entry.GetProperty("mcGroupId").GetByte(), $"the group of {text}");
        Assert(read.McAddr == entry.GetProperty("mcAddr").GetUInt32(), "its address");
        Assert(Hex(read.Encode()) == text, $"{text} is written back the way it was read");
    }

    // Nothing says how long an unknown command is, so reading stops rather than guessing.
    JsonElement stops = vector.GetProperty("stops");
    Assert(
        LorawanPackageCommand.ParseAll(
            stops.GetProperty("port").GetByte(),
            stops.GetProperty("uplink").GetBoolean(),
            Unhex(stops, "bytes")).Count == stops.GetProperty("readable").GetInt32(),
        "reading stops at the unknown one");

    JsonElement refused = vector.GetProperty("notAPackage");
    try
    {
        LorawanPackageCommand.Parse(
            refused.GetProperty("port").GetByte(),
            refused.GetProperty("uplink").GetBoolean(),
            Unhex(refused, "bytes"));
        Assert(false, "a port that names no package is refused");
    }
    catch (PamojaException)
    {
    }

    static string Hex(byte[] bytes) => Convert.ToHexString(bytes).ToLowerInvariant();
    static byte[] Unhex(JsonElement owner, string name) =>
        Convert.FromHexString(owner.GetProperty(name).GetString()!);
}

sealed class QueueLink : IReceivingTransportHandlers
{
    private readonly System.Threading.Channels.Channel<TransportMessage?> _inbox =
        System.Threading.Channels.Channel.CreateUnbounded<TransportMessage?>();

    public List<TransportMessage> Sent { get; } = new();

    public List<string> Filters { get; } = new();

    public Task ConnectAsync() => Task.CompletedTask;

    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        Sent.Add(new TransportMessage(topic, payload.ToArray()));
        return Task.CompletedTask;
    }

    public Task SubscribeAsync(string topic)
    {
        Filters.Add(topic);
        return Task.CompletedTask;
    }

    public async Task<TransportMessage?> ReceiveAsync() => await _inbox.Reader.ReadAsync();

    public void Deliver(TransportMessage? message) => _inbox.Writer.TryWrite(message);
}

/// <summary>A link that only sends.</summary>
sealed class SendOnlyLink : ITransportHandlers
{
    public Task ConnectAsync() => Task.CompletedTask;

    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload) => Task.CompletedTask;

    public Task SubscribeAsync(string topic) =>
        throw new InvalidOperationException("an uplink is never subscribed");
}

/// <summary>A link whose sends fail with a reason.</summary>
sealed class RefusingLink : ITransportHandlers
{
    public Task ConnectAsync() => Task.CompletedTask;

    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload) =>
        throw new InvalidOperationException("the radio is out of range");

    public Task SubscribeAsync(string topic) => Task.CompletedTask;
}

/// <summary>A coil line whose driver has come unplugged.</summary>
sealed class Unplugged : IOutputLine
{
    public void Drive(PinLevel level) => throw new InvalidOperationException("line unplugged");
}
