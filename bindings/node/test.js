// Smoke test: confirms the facade loads, the native core is reachable, and each
// capability behaves through it (no broker or hardware required).
const assert = require("node:assert");
const {
  version,
  MqttClient,
  Qos,
  DeviceIdentity,
  verify,
  fingerprint,
  toCbor,
  fromCbor,
  packSamples,
  unpackSamples,
  Quantizer,
  Smoother,
  Thermostat,
  Depletion,
  Calibration,
  Geofence,
  Boundary,
  deadband,
  distanceBetween,
  audit,
  bus,
  can,
  coap,
  gpio,
  lora,
  lorawan,
  mesh,
  modbus,
  ladder,
  loopback,
  power,
  profile,
  ros2,
  routing,
  serial,
  session,
  sim,
  sync,
  telemetry,
  transport,
  update,
  zenoh,
  actuators,
  sensors,
  Window,
  Median,
  Trend,
  Anomaly,
  WINDOW_CAPACITY,
} = require("./dist/index.js");

async function main() {
  const v = version();
  console.log("pamoja version:", v);
  assert.strictEqual(typeof v, "string", "version() should return a string");

  assert.strictEqual(Qos.AtLeastOnce, "AtLeastOnce", "Qos should expose string levels");

  const client = new MqttClient({
    clientId: "smoke",
    host: "127.0.0.1",
    port: 47811,
    keepAliveSecs: 1,
  });

  assert.strictEqual(
    await client.isConnected(),
    false,
    "a fresh client should not be connected",
  );

  await assert.rejects(
    () => client.connect(),
    /transport error/,
    "connecting to a closed port should reject with a transport error",
  );

  assert.strictEqual(
    await client.isConnected(),
    false,
    "a failed connect should leave the client disconnected",
  );

  identity();
  codecs();
  helpers();
  fieldIo();
  sensingAndActuation();
  radioAndReach();
  trustAndOperation();
  await asyncTransports();

  console.log("ok");
}

// Signing a payload and checking it, the way a gateway verifies a reading.
function identity() {
  const device = DeviceIdentity.fromSeed(Buffer.alloc(32, 7));
  const publicKey = device.publicKey();
  assert.strictEqual(publicKey.length, 32, "a public key should be 32 bytes");

  const signature = device.sign("21.5");
  assert.strictEqual(signature.length, 64, "a signature should be 64 bytes");
  assert.ok(verify(publicKey, "21.5", signature), "a signature should verify");
  assert.ok(
    !verify(publicKey, "21.6", signature),
    "a tampered payload should not verify",
  );

  assert.match(fingerprint(publicKey), /^[0-9a-f]{16}$/, "a fingerprint is 16 hex characters");
  assert.strictEqual(fingerprint(publicKey), device.fingerprint());

  assert.throws(
    () => verify(Buffer.alloc(8), "21.5", signature),
    /publicKey must be exactly 32 bytes/,
    "a wrong-length key is an argument error, not a failed verification",
  );
}

// Moving a document to the compact form a metered link should carry, and back.
function codecs() {
  const reading = { id: "probe-1", c: 21.5, battery: 88 };
  const cbor = toCbor(reading);
  assert.ok(
    cbor.length < Buffer.byteLength(JSON.stringify(reading)),
    "CBOR should be smaller than the JSON it came from",
  );
  assert.deepStrictEqual(fromCbor(cbor), reading, "a document should round-trip");

  const samples = [10, 11, 13, 12, 900];
  assert.deepStrictEqual(unpackSamples(packSamples(samples)), samples);

  const quantizer = new Quantizer(100);
  const packed = quantizer.encode([20.0, 20.1, 20.2, 20.3]);
  assert.ok(packed.length < 4 * 4, "packed readings should beat four bytes each");
  for (const [i, value] of quantizer.decode(packed).entries()) {
    assert.ok(Math.abs(value - (20.0 + i * 0.1)) < 0.05, "readings decode to precision");
  }

  assert.throws(() => new Quantizer(0), "a non-positive scale should throw");
  assert.throws(() => fromCbor(Buffer.from([0xff, 0xff])), "malformed CBOR should throw");
}

// The helper math a field node runs between reading a sensor and acting on it.
function helpers() {
  const smoother = new Smoother(0.5);
  assert.strictEqual(smoother.value(), null, "a fresh smoother has no value");
  smoother.update(10);
  const smoothed = smoother.update(20);
  assert.ok(smoothed > 10 && smoothed < 20, "smoothing should lag the step");
  smoother.reset();
  assert.strictEqual(smoother.value(), null, "reset should clear the value");

  const fridge = Thermostat.cooling(8, 1);
  assert.ok(!fridge.update(7), "a cool fridge leaves the compressor off");
  assert.ok(fridge.update(9.5), "a warm fridge switches the compressor on");
  assert.ok(fridge.isOn());

  const tank = new Depletion(10);
  assert.strictEqual(tank.update(100), null, "the first reading sets no rate");
  assert.ok(tank.update(90) > 0, "a falling level projects a countdown");

  const probe = Calibration.twoPoint(0, 0, 1024, 100);
  assert.ok(Math.abs(probe.apply(512) - 50) < 0.01, "a two-point fit maps its midpoint");

  assert.strictEqual(deadband(0.2, 0, 0.5), 0, "noise inside the band does not act");

  const centre = { latitude: -1.2921, longitude: 36.8219 };
  const away = { latitude: -1.293, longitude: 36.8219 };
  const pen = new Geofence(centre, 50);
  assert.strictEqual(pen.update(centre), Boundary.Inside);
  assert.strictEqual(pen.update(away), Boundary.Exited, "the crossing fix reports once");
  assert.strictEqual(pen.update(away), Boundary.Outside, "later fixes stay outside");
  assert.ok(distanceBetween(centre, away) > 50, "the fix is beyond the radius");
}

// The wires a gateway actually has: framed serial packets, an RS485 request and
// the reply it draws, a CAN frame, and the address a chip answers on.
function fieldIo() {
  const payload = Buffer.from([0xc0, 0xdb, 0x00, 0x2a]);
  assert.deepStrictEqual(
    serial.slip.decode(serial.slip.encode(payload)),
    payload,
    "a SLIP frame round-trips",
  );
  assert.deepStrictEqual(
    serial.cobs.decode(serial.cobs.encode(payload)),
    payload,
    "a COBS frame round-trips",
  );

  const decoder = new serial.SlipDecoder();
  const frames = decoder.feed(Buffer.from([0x6f, 0x6b, 0xc0, 0xdb, 0xc0, 0x67, 0x6f, 0xc0]));
  assert.strictEqual(frames.length, 2, "the frames either side of a corrupt one survive");
  assert.strictEqual(decoder.discarded, 1, "the corrupt frame is counted");

  assert.deepStrictEqual(
    modbus.readHoldingRegisters(0x11, 0x006b, 3),
    Buffer.from([0x11, 0x03, 0x00, 0x6b, 0x00, 0x03, 0x76, 0x87]),
    "the request carries the address, the PDU, and the CRC",
  );

  const body = Buffer.from([0x11, 0x03, 0x06, 0x02, 0x2b, 0x00, 0x00, 0x00, 0x64]);
  const checksum = Buffer.alloc(2);
  checksum.writeUInt16LE(modbus.crc16(body));
  const reply = modbus.parseFrame(Buffer.concat([body, checksum]));
  assert.strictEqual(reply.exception, null, "a served request reports no exception");
  assert.deepStrictEqual(reply.registers(), [0x022b, 0x0000, 0x0064], "registers read back");
  const corrupt = Buffer.concat([body, checksum]);
  corrupt[2] ^= 0xff;
  assert.throws(
    () => modbus.parseFrame(corrupt),
    "a frame mangled on the wire should throw",
  );

  const frame = can.frame(0x20a, Buffer.from([0x01, 0xf4]));
  assert.strictEqual(frame.dlc, 2, "a classic frame carries its payload");
  const remote = can.remoteFrame(0x20a, 4);
  assert.strictEqual(remote.len, 4, "a remote frame asks for a length");
  assert.strictEqual(remote.data.length, 0, "without carrying the bytes");

  assert.strictEqual(can.decodeJ1939(0x0cf00400).pgn, 61444, "the engine broadcast decodes");
  assert.strictEqual(can.decodeJ1939(0x123, false), null, "J1939 needs an extended identifier");

  assert.deepStrictEqual(
    gpio.i2c.addressFrame(0x76),
    Buffer.from([0xec]),
    "a write frame shifts in the r/w bit",
  );
  assert.ok(gpio.i2c.isReserved(0x00) && gpio.i2c.isGeneralCall(0x00), "the general call is reserved");
  assert.deepStrictEqual(
    gpio.spi.clockFor(3),
    { cpol: true, cpha: true },
    "mode 3 idles high and samples late",
  );
  assert.strictEqual(
    gpio.pin.levelFor(gpio.PinPolarity.ActiveLow, true),
    gpio.PinLevel.Low,
    "an active-low relay is energised by a low level",
  );
}

// The parts wired to a board: a thermometer that checks its own bytes, a servo
// pulse, a stepper walking its coils, and the stats over a rolling window.
function sensingAndActuation() {
  const scratchpad = Buffer.from([0x91, 0x01, 0x4b, 0x46, 0x7f, 0xff, 0x0c, 0x10, 0x00]);
  scratchpad[8] = sensors.ds18b20.crc8(scratchpad.subarray(0, 8));
  const reading = sensors.ds18b20.parseScratchpad(scratchpad);
  assert.strictEqual(reading.microCelsius, 25062500, "the thermometer decodes its register");
  assert.strictEqual(reading.resolutionBits, 12, "and reports its resolution");

  const corrupt = Buffer.from(scratchpad);
  corrupt[0] ^= 0xff;
  assert.throws(
    () => sensors.ds18b20.parseScratchpad(corrupt),
    "a scratchpad failing its CRC should throw",
  );

  assert.strictEqual(sensors.ina219.calibration(1000, 2), 0x5000, "the datasheet example");
  assert.strictEqual(
    sensors.ina219.powerMicrowatts(100, 1000),
    2000000,
    "the power LSB is twenty times the current LSB",
  );

  const reset = sensors.ads1115.configFromBits(0x8583);
  assert.strictEqual(sensors.ads1115.configBits(reset), 0x8583, "the config round-trips");
  assert.strictEqual(
    sensors.ads1115.fullScaleMicrovolts(1),
    4096000,
    "gain code 1 is plus or minus 4.096 V",
  );

  assert.strictEqual(
    actuators.pwm.fullOff()[3],
    0x10,
    "fully off is its own encoding, not a zero duty",
  );
  assert.strictEqual(actuators.pca9685.channelRegister(0), 0x06, "the first channel block");

  const motor = new actuators.Stepper(actuators.StepDrive.HalfStep);
  const first = motor.coils;
  for (let step = 0; step < actuators.stepCount(actuators.StepDrive.HalfStep); step += 1) {
    motor.step(actuators.StepDirection.Forward);
  }
  assert.strictEqual(motor.coils, first, "one electrical cycle returns to its first pattern");
  assert.strictEqual(motor.steps, 8, "and the position counts every step");

  const window = new Window();
  [10, 20, 30].forEach((value) => window.push(value));
  assert.strictEqual(window.len, 3, "the window fills");
  assert.strictEqual(window.capacity, WINDOW_CAPACITY, "up to its documented capacity");
  assert.ok(Math.abs(window.mean() - 20) < 1e-5, "and averages its readings");

  const median = new Median();
  [20, 21, 20.5].forEach((value) => median.update(value));
  assert.ok(median.update(900) < 30, "a median does not follow a single spike");

  const trend = new Trend();
  [1, 2, 3, 4].forEach((value) => trend.push(value));
  assert.ok(Math.abs(trend.slope() - 1) < 1e-4, "a rising signal has a positive slope");

  const anomaly = new Anomaly(3);
  for (let i = 0; i < 8; i += 1) anomaly.check(20);
  assert.ok(anomaly.check(900), "a reading far outside the window is flagged");
}

// Budgeting airtime, framing a mesh packet, routing it, and securing a LoRaWAN
// uplink: everything a node needs to reach a network it cannot see.
function radioAndReach() {
  const link = lora.link(12, 125_000);
  assert.strictEqual(link.spreadingFactor, 12, "SF12 is the longest-range setting");
  assert.strictEqual(lora.airtimeUs(link, 10), 991_232, "the published LoRa airtime");
  assert.strictEqual(
    lora.minOffTimeUs(link, 20, 10),
    lora.airtimeUs(link, 20) * 99,
    "a 1% duty cycle costs ninety-nine times the airtime in silence",
  );
  assert.strictEqual(
    lora.minOffTimeUs(link, 20, 0),
    null,
    "a zero duty cycle forbids transmitting at all",
  );
  assert.ok(lora.messagesPerHour(link, 20, 10) > 0, "and a 1% budget still allows some");

  const reading = mesh.broadcast(0x1234_5678, 1, Buffer.from("level=high"));
  const received = mesh.parse(reading.bytes);
  assert.ok(received.broadcast, "a broadcast is addressed to every node");
  assert.strictEqual(received.payload.toString(), "level=high", "and carries its reading");

  const seen = new mesh.SeenPackets();
  assert.ok(seen.record(received.src, received.id), "the first copy is new");
  assert.ok(!seen.record(received.src, received.id), "a second copy is a duplicate");

  const forwarded = mesh.relayed(received.bytes);
  assert.strictEqual(
    forwarded.hopLimit,
    received.hopLimit - 1,
    "relaying spends one hop",
  );

  const corrupt = Buffer.from(received.bytes);
  corrupt[corrupt.length - 3] ^= 0xff;
  assert.throws(() => mesh.parse(corrupt), /CRC/, "a mangled frame is refused");

  const router = routing.router(0x01);
  router.observe(0x09, 0x05, 2);
  assert.strictEqual(router.forward(0x09).nextHop, 0x05, "a learned route relays");
  router.observe(0x09, 0x07, 1);
  assert.strictEqual(router.forward(0x09).nextHop, 0x07, "a cheaper neighbour wins");
  assert.strictEqual(
    router.forward(0x01).action,
    routing.ForwardAction.Deliver,
    "a packet for this node is delivered",
  );
  assert.strictEqual(
    router.forward(0x20).action,
    routing.ForwardAction.Flood,
    "and an unknown destination falls back to flooding",
  );

  const session = lorawan.session(0x2601_1bda, Buffer.alloc(16, 0x2b), Buffer.alloc(16, 0x99));
  const uplink = session.encodeUplink(42, 1, Buffer.from("temp=4.8"), { confirmed: true });
  const rx = session.decode(uplink, 42);
  assert.strictEqual(rx.direction, lorawan.Direction.Uplink, "the frame went up");
  assert.ok(rx.confirmed, "and asked to be acknowledged");
  assert.strictEqual(rx.payload.toString(), "temp=4.8", "the payload decrypts");

  const forged = Buffer.from(uplink);
  forged[forged.length - 1] ^= 0xff;
  assert.throws(() => session.decode(forged, 42), /MIC/, "a forged frame is refused");

  const node = lorawan.device(
    Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]),
    Buffer.from([0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]),
    Buffer.alloc(16, 0x2b),
  );
  assert.strictEqual(node.joinRequest(0x0102).length, 23, "a join request is 23 bytes");
  assert.throws(
    () => node.acceptJoin(Buffer.alloc(17, 0x20), 0x0102),
    /MIC/,
    "a join accept the network never signed does not activate a session",
  );
}


// Proving what a node did, saying it in confidence, fixing it in the field, and
// deciding how often it can afford to do any of that.
function trustAndOperation() {
  // A signed, chained log: what a node did, in an order nobody can quietly edit.
  const keeper = new DeviceIdentity(Buffer.alloc(32, 0x21));
  const log = new audit.AuditLog(keeper);
  const opened = log.append(Buffer.from("valve=open"));
  const shut = log.append(Buffer.from("valve=shut"));

  assert.strictEqual(Number(opened.index), 0, "the first record sits at index zero");
  assert.deepStrictEqual(
    Buffer.from(shut.previous),
    Buffer.from(opened.digest),
    "each record carries the hash of the one before it",
  );
  assert.ok(
    audit.verifyChain(keeper.publicKey(), [opened, shut]),
    "an untouched chain verifies",
  );

  const edited = Buffer.from(shut.toBytes());
  edited[edited.length - 1] ^= 0xff;
  assert.ok(
    !audit.verifyChain(keeper.publicKey(), [
      opened,
      audit.AuditEntry.fromBytes(edited),
    ]),
    "and an altered record breaks it",
  );

  // A resumed log continues the chain rather than starting a second one.
  const resumed = audit.AuditLog.resume(keeper, shut);
  const afterReboot = resumed.append(Buffer.from("valve=open"));
  assert.strictEqual(Number(afterReboot.index), 2, "a reboot leaves no gap");

  // Two devices that know each other's public keys, talking in confidence.
  const node = new session.AgreementKey(Buffer.alloc(32, 0x01));
  const gateway = new session.AgreementKey(Buffer.alloc(32, 0x02));
  const salt = Buffer.alloc(16, 0x09);
  const uplink = new session.Session(node, gateway.publicKey(), salt, session.Role.Initiator);
  const downlink = new session.Session(gateway, node.publicKey(), salt, session.Role.Responder);

  const label = Buffer.from("pump-3");
  const sealed = uplink.seal(Buffer.from("4.8C"), label);
  assert.notStrictEqual(
    sealed.ciphertext.toString(),
    "4.8C",
    "the reading does not travel in the clear",
  );
  assert.strictEqual(
    downlink.open(sealed, label).toString(),
    "4.8C",
    "the peer recovers it",
  );
  assert.throws(
    () => downlink.open(sealed, label),
    /repeat|replay/i,
    "and refuses the same message twice",
  );

  const tampered = { ...uplink.seal(Buffer.from("4.9C"), label) };
  tampered.ciphertext = Buffer.from(tampered.ciphertext);
  tampered.ciphertext[0] ^= 0xff;
  assert.throws(
    () => downlink.open(tampered, label),
    /authenticat/i,
    "an altered message is refused",
  );

  // Fixing a device in the field: a signed release, staged in pieces, tried, and
  // confirmed only once it has run.
  const vendor = Buffer.alloc(16, 0x0a);
  const deviceClass = Buffer.alloc(16, 0x0b);
  const publisher = new DeviceIdentity(Buffer.alloc(32, 0x31));
  const image = Buffer.alloc(600, 0xa5);
  const manifest = {
    structureVersion: update.STRUCTURE_VERSION,
    sequence: 2,
    vendorId: vendor,
    classId: deviceClass,
    format: update.FORMAT_RAW,
    storage: 1,
    digest: require("node:crypto").createHash("sha256").update(image).digest(),
    size: image.length,
    expires: 0,
  };
  const envelope = update.signManifest(manifest, publisher);
  assert.deepStrictEqual(
    update.verifyEnvelope(envelope, publisher.publicKey()).digest,
    manifest.digest,
    "the release verifies against the key that signed it",
  );

  const fleet = new update.Updater(vendor, deviceClass, publisher.publicKey(), 2, 4096);
  fleet.provision(0, 1);
  assert.strictEqual(fleet.begin(envelope), 1, "the release names the spare slot");
  for (let at = 0; at < image.length; at += 128) {
    fleet.write(image.subarray(at, at + 128));
  }
  assert.strictEqual(
    fleet.progress().written,
    image.length,
    "every byte arrived",
  );
  assert.strictEqual(fleet.finish(), 1, "and the image matched what was promised");

  const boot = fleet.onBoot();
  assert.strictEqual(boot.action, update.BootAction.Trying, "a new image is on trial");
  assert.strictEqual(fleet.confirm(), 1, "and confirms once it has run");
  assert.strictEqual(
    fleet.slotRecord(1).state,
    update.SlotState.Confirmed,
    "so the slot holds the release from now on",
  );

  const impostor = new DeviceIdentity(Buffer.alloc(32, 0x32));
  assert.throws(
    () => fleet.stage(update.signManifest({ ...manifest, sequence: 3 }, impostor), image),
    /signature/i,
    "a release signed by anyone else is refused",
  );
  assert.throws(
    () => fleet.stage(update.signManifest({ ...manifest, sequence: 1 }, publisher), image),
    /roll/i,
    "and one that would roll the device back is refused",
  );

  // How often a node on a battery can afford to do any of the above.
  const plan = new power.PowerPlan(60_000_000, 300_000_000, 3_600_000_000);
  assert.strictEqual(plan.mode(0.9), power.PowerMode.Active, "a healthy charge works normally");
  assert.strictEqual(plan.mode(0.1), power.PowerMode.Critical, "a flat one barely works at all");
  assert.strictEqual(
    plan.modeWhileCharging(0.1, true),
    power.PowerMode.Saver,
    "and sunlight eases it back one step",
  );
  assert.strictEqual(plan.intervalUs(0.1), 3_600_000_000, "which is an hour between readings");

  const duty = power.DutyCycle.fromFraction(1_000_000, 0.25);
  assert.strictEqual(duty.activeUs, 250_000, "a quarter of the period is spent awake");

  // What it says about itself on the way back, and what it drops when the link
  // costs too much to say it.
  const reporter = new telemetry.Reporter(telemetry.Level.Trace);
  reporter.adaptTo(telemetry.LinkCost.Expensive);
  assert.strictEqual(
    reporter.record({ level: telemetry.Level.Info, code: "loop.tick" }),
    null,
    "routine detail is dropped on a costly link",
  );
  const warned = reporter.record({
    level: telemetry.Level.Warn,
    code: "battery.low",
    value: 0.18,
  });
  assert.strictEqual(warned?.code, "battery.low", "but a warning still ships");
  assert.strictEqual(warned?.value, 0.18, "with the measurement that triggered it");

  const counts = reporter.snapshot();
  assert.strictEqual(counts.dropped, 1, "the dropped event was still counted");
  assert.strictEqual(counts.emitted, 1, "alongside the one that shipped");
  assert.strictEqual(
    telemetry.linkCostThreshold(telemetry.LinkCost.Offline),
    telemetry.Level.Error,
    "and an offline link ships only failures",
  );
}


// Reaching the network when no single link always works, and testing all of it
// with nothing plugged in.
async function asyncTransports() {
  // An in-process broker: publish on one link, receive on another.
  const broker = new loopback.LoopbackBroker();
  const publisher = broker.link();
  const subscriber = broker.link();
  await publisher.connect();
  await subscriber.connect();
  assert.ok(await subscriber.isConnected(), "a connected link reports it");

  await subscriber.subscribe("sensors/1");
  await publisher.send("sensors/1", Buffer.from("21.5"));

  const received = await subscriber.recv();
  assert.strictEqual(received.topic, "sensors/1", "the topic survives");
  assert.strictEqual(received.payload.toString(), "21.5", "and so does the reading");

  // A buffer holds what cannot be sent yet.
  const store = sync.Store.memory();
  await store.append(Buffer.from("one"));
  await store.append(Buffer.from("two"));
  assert.strictEqual(await store.len(), 2);
  assert.strictEqual((await store.peek()).toString(), "one", "peek leaves it in place");
  assert.strictEqual((await store.pop()).toString(), "one");
  assert.strictEqual((await store.pop()).toString(), "two");
  assert.strictEqual(await store.pop(), null, "an empty store yields nothing");

  const bounded = sync.Store.memory(1);
  await bounded.append(Buffer.from("one"));
  await assert.rejects(
    () => bounded.append(Buffer.from("two")),
    "a full store tells the caller rather than dropping something",
  );

  // With no rung, a ladder buffers rather than losing the reading.
  const offline = new ladder.Ladder(sync.Store.memory());
  assert.strictEqual(
    await offline.send("sensors/1", Buffer.from("21.5")),
    ladder.Delivery.Buffered,
    "buffering is a success, not a failure",
  );
  assert.strictEqual(await offline.buffered(), 1);

  // The link comes back, and the buffer drains over it.
  await offline.rung(broker.rung());
  await offline.connect();
  assert.strictEqual(await offline.flush(), 1, "the buffered reading went out");
  assert.strictEqual(await offline.buffered(), 0);

  // A rung that refuses falls through to the next.
  const rungs = new ladder.Ladder(sync.Store.memory());
  await rungs.rung(transport.Transport.faulty(broker.rung(), 1));
  await rungs.rung(broker.rung());
  await rungs.connect();
  assert.strictEqual(
    await rungs.send("sensors/1", Buffer.from("4.8C")),
    ladder.Delivery.Sent,
    "the second rung carried what the first refused",
  );

  // A transport handed to a ladder is spent.
  const spent = broker.rung();
  assert.ok(spent.isAvailable, "a fresh transport is holdable");
  await rungs.rung(spent);
  assert.ok(!spent.isAvailable, "and is not once it has been added");
  await assert.rejects(
    () => rungs.rung(spent),
    /already added/,
    "adding it twice is refused",
  );

  // One publisher, many subscribers, in one process.
  const hub = new bus.EventBus(8);
  const first = await hub.subscribe();
  const second = await hub.subscribe();
  await hub.publish(Buffer.from("battery.low"));
  assert.strictEqual((await first.next()).toString(), "battery.low");
  assert.strictEqual((await second.next()).toString(), "battery.low");

  // Devices that need no hardware.
  const seeded = new sim.SimulatedSensor(20.0, 0.5, 1.0, 42);
  const twin = new sim.SimulatedSensor(20.0, 0.5, 1.0, 42);
  for (let at = 0; at < 5; at += 1) {
    assert.strictEqual(await seeded.read(), await twin.read(), "a seed makes a run repeat");
  }

  const replay = new sim.Replay([21.0, 21.5, 22.0], true);
  for (let round = 0; round < 2; round += 1) {
    for (const want of [21.0, 21.5, 22.0]) {
      assert.ok(Math.abs((await replay.read()) - want) < 1e-6, "a capture reads back");
    }
  }

  const actuator = new sim.RecordingActuator();
  for (const command of [0.0, 0.5, 1.0]) {
    await actuator.apply(command);
  }
  assert.strictEqual(await actuator.length(), 3, "every command was recorded");
  assert.deepStrictEqual(await actuator.commands(), [0.0, 0.5, 1.0]);

  const robot = new sim.SimulatedRobot(1.0);
  await robot.apply({ vx: 1.0, vy: 0.0, omega: 0.0 });
  assert.ok(
    Math.abs((await robot.pose()).x - 1.0) < 1e-5,
    "one second at one metre a second puts it a metre ahead",
  );
  // A profile decides what a reading calls for, with no hardware wired up.
  const fridge = profile.Profile.vaccineFridgeMonitor();
  assert.strictEqual(fridge.name, "vaccine-fridge-monitor");
  assert.strictEqual(fridge.control.kind, profile.ControlKind.Setpoint);

  const control = fridge.controller();
  const warm = control.evaluate(9.0);
  assert.strictEqual(warm.actuator, true, "a warm fridge runs the cooler");
  assert.strictEqual(
    warm.alert.kind,
    profile.AlertKind.OutOfRange,
    "and 9 C is a spoilage excursion",
  );
  assert.ok(Math.abs(warm.alert.reading - 9.0) < 1e-6);

  const observed = profile.Controller.monitor().evaluate(21.5);
  assert.ok(observed.actuator == null, "a monitor drives no output");
  assert.ok(observed.alert == null, "and judges nothing");

  const manifest = fridge.toJson();
  const reloaded = profile.Profile.fromJson(manifest);
  assert.strictEqual(reloaded.topic, fridge.topic, "a manifest round-trips");
  assert.throws(() => profile.Profile.fromJson("{"), "a malformed manifest throws");

  // The ROS 2 naming rules, with no ROS installation in sight.
  assert.ok(ros2.name.isValid("/robot1/camera_left/image_raw"));
  assert.ok(!ros2.name.isValid("/2foo"), "a token may not start with a digit");
  assert.strictEqual(
    ros2.name.ddsTopic("/robot1/cmd_vel", ros2.EntityKind.Topic),
    "rt/robot1/cmd_vel",
  );
  assert.strictEqual(ros2.name.prefixFor(ros2.EntityKind.ServiceRequest), "rq");
  assert.strictEqual(
    ros2.name.ddsTypeName("std_msgs/msg/String"),
    "std_msgs::msg::dds_::String_",
  );

  const chatterHash =
    "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18";
  assert.strictEqual(ros2.typeHash.digest(chatterHash).length, 32);
  assert.strictEqual(
    ros2.typeHash.entityKey(0, "/chatter", "std_msgs/msg/String", chatterHash),
    `0/chatter/std_msgs::msg::dds_::String_/${chatterHash}`,
  );

  const command = {
    linear: { x: 1.5, y: 0.0, z: 0.0 },
    angular: { x: 0.0, y: 0.0, z: -0.25 },
  };
  const decoded = ros2.cdr.twistFromBytes(ros2.cdr.twistToBytes(command));
  assert.strictEqual(decoded.linear.x, 1.5, "a twist survives a CDR round trip");
  assert.strictEqual(decoded.angular.z, -0.25);

  const writer = ros2.cdr.writer();
  writer.writeU32(7);
  writer.writeF64(2.5);
  writer.writeI32(-3);
  const reader = ros2.cdr.reader(writer.bytes);
  assert.strictEqual(reader.readU32(), 7);
  assert.strictEqual(reader.readF64(), 2.5, "an eight-byte field keeps its alignment");
  assert.strictEqual(reader.readI32(), -3, "and the field after it is not skewed");
  assert.strictEqual(reader.readU32(), null, "reading past the end yields null");

  // Zenoh key expressions, which is how a fleet subtree is addressed.
  assert.ok(zenoh.keyexpr.isValid("fleet/*/battery"));
  assert.ok(zenoh.keyexpr.matches("fleet/*/battery", "fleet/n7/battery"));
  assert.ok(!zenoh.keyexpr.matches("fleet/*/battery", "fleet/n7/rack/battery"));
  assert.strictEqual(
    zenoh.keyexpr.canonize("fleet/**/**/battery"),
    "fleet/**/battery",
    "a redundant double wildcard canonizes away",
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
