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
  can,
  gpio,
  lora,
  lorawan,
  mesh,
  modbus,
  routing,
  serial,
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

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
