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
  Pid,
  Debounce,
  Thermostat,
  Trigger,
  Edge,
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
  hal,
  lora,
  lorawan,
  mavlink,
  mesh,
  modbus,
  ladder,
  loopback,
  power,
  profile,
  gateway,
  radios,
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
  Complementary,
  tiltFromAccel,
  dewPoint,
  celsiusToFahrenheit,
  pascalsToHectopascals,
  DiffDrive,
  Ackermann,
  SkidSteer,
  Mecanum,
  TwoLinkArm,
  Elbow,
  forwardKinematics,
  Transform,
  Odometry,
  WaypointFollower,
  obstacleStop,
  SafetyGate,
  Limits,
  Watchdog,
  EStop,
  ServoMap,
  Esc,
  Quadrature,
  QuadratureScale,
} = require("pamoja");

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
  motion();
  fieldIo();
  sensingAndActuation();
  laterSensors();
  await buses();
  await serialPorts();
  await modbusClients();
  await canBuses();
  await sensorDrivers();
  await actuatorDrivers();
  await stepperDrivers();
  radioAndReach();
  relayedReach();
  broadcastUpdates();
  mavlinkWire();
  mavlinkShapes();
  mavlinkProtocols();
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

  assert.throws(
    () => new Quantizer(0),
    /a quantizer's scale must be a positive, finite number, not 0/,
    "a non-positive scale should throw",
  );
  assert.throws(() => fromCbor(Buffer.from([0xff, 0xff])), "malformed CBOR should throw");

  assert.throws(
    () => quantizer.encode([20.0, NaN]),
    /codec error: reading 1 is NaN, which cannot be quantized/,
    "a missing reading is refused rather than packed as a number",
  );
  assert.throws(() => quantizer.encode([Infinity]), /reading 0 is inf/);
  assert.throws(
    () => packSamples([10, 10.5]),
    /sample 1 is 10.5, which is not a whole number/,
    "a fraction is refused rather than rounded",
  );
  assert.throws(() => packSamples([NaN]), /sample 0 is NaN, which is not a whole number/);
  assert.throws(
    () => packSamples([2 ** 53]),
    /sample 0 is 9007199254740992, past the largest whole number a JavaScript number holds exactly/,
  );
  assert.deepStrictEqual(
    unpackSamples(packSamples([Number.MAX_SAFE_INTEGER, -Number.MAX_SAFE_INTEGER])),
    [Number.MAX_SAFE_INTEGER, -Number.MAX_SAFE_INTEGER],
    "the largest exact whole numbers round-trip",
  );
  const pastSafe = Buffer.from([0x01, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x20]);
  assert.throws(
    () => unpackSamples(pastSafe),
    /sample 0 is 9007199254740992, past the largest whole number/,
    "a sample JavaScript cannot hold exactly is refused rather than rounded",
  );
  assert.throws(
    () => unpackSamples(Buffer.concat([packSamples([1, 2, 3]), packSamples([4])])),
    /2 bytes follow the batch's last sample/,
    "two batches run together are refused",
  );
}

// The helper math a field node runs between reading a sensor and acting on it.
function helpers() {
  const smoother = new Smoother(0.5);
  assert.strictEqual(smoother.value, null, "a fresh smoother has no value");
  smoother.update(10);
  const smoothed = smoother.update(20);
  assert.ok(smoothed > 10 && smoothed < 20, "smoothing should lag the step");
  smoother.reset();
  assert.strictEqual(smoother.value, null, "reset should clear the value");

  const dry = Trigger.below(30, 5);
  assert.strictEqual(dry.update(42), null, "above the line nothing fires");
  assert.strictEqual(dry.update(28), Edge.Set, "crossing it fires once");
  assert.ok(dry.isSet, "and the trigger reports the condition holds");
  assert.strictEqual(dry.update(33), null, "inside the band it holds");
  assert.strictEqual(dry.update(36), Edge.Cleared, "coming back past the band clears it");
  assert.strictEqual(dry.threshold, 30);

  const fridge = Thermostat.cooling(8, 1);
  assert.ok(!fridge.update(7), "a cool fridge leaves the compressor off");
  assert.ok(fridge.update(9.5), "a warm fridge switches the compressor on");
  assert.ok(fridge.isOn);

  const tank = new Depletion(10);
  assert.strictEqual(tank.update(100), null, "the first reading sets no rate");
  assert.ok(tank.update(90) > 0, "a falling level projects a countdown");

  const probe = Calibration.twoPoint(0, 0, 1024, 100);
  assert.ok(Math.abs(probe.apply(512) - 50) < 0.01, "a two-point fit maps its midpoint");

  assert.strictEqual(deadband(0.2, 0, 0.5), 0, "noise inside the band does not act");

  const center = { latitude: -1.2921, longitude: 36.8219 };
  const away = { latitude: -1.293, longitude: 36.8219 };
  const pen = new Geofence(center, 50);
  assert.strictEqual(pen.update(center), Boundary.Inside);
  assert.strictEqual(pen.update(away), Boundary.Exited, "the crossing fix reports once");
  assert.strictEqual(pen.update(away), Boundary.Outside, "later fixes stay outside");
  assert.ok(distanceBetween(center, away) > 50, "the fix is beyond the radius");

  assert.strictEqual(celsiusToFahrenheit(100), 212);
  assert.strictEqual(pascalsToHectopascals(101325), 1013.25);
  const rolled = tiltFromAccel(0, 1, 1);
  assert.ok(Math.abs(rolled.roll - 45) < 1e-9 && Math.abs(rolled.pitch) < 1e-9);
  assert.ok(Math.abs(dewPoint(15, 100) - 15) < 1e-6, "saturated air dews at its temperature");
  const tilt = new Complementary(0.98, 0);
  const settled = tilt.update(10, 1, 0.1);
  assert.strictEqual(tilt.update(NaN, 1, 0.1), settled, "a reading that is not a number is ignored");
  assert.strictEqual(tilt.estimate, settled);
}

// Driving a robot: the chassis, an arm, odometry, a waypoint, and the gate every command
// passes through.
function motion() {
  const drive = new DiffDrive(0.5);
  assert.deepStrictEqual(drive.wheelSpeeds(0, 2), { left: -0.5, right: 0.5 }, "spinning in place");
  assert.deepStrictEqual(drive.bodyMotion(-0.5, 0.5), { linear: 0, angular: 2 });

  const car = new Ackermann(2.5);
  assert.strictEqual(car.turnRadius(0), Infinity, "wheels straight never turn");
  const omega = car.yawRate(5, 0.4);
  assert.ok(Math.abs(car.steeringAngle(5, omega) - 0.4) < 1e-5);

  const tracked = new SkidSteer(0.5, 1.2);
  const spin = tracked.wheelSpeeds(0, 2);
  assert.ok(Math.abs(spin.left + 0.6) < 1e-6 && Math.abs(spin.right - 0.6) < 1e-6);

  const base = new Mecanum(0.4, 0.3);
  assert.deepStrictEqual(
    base.wheelSpeeds({ vx: 0, vy: 1, omega: 0 }),
    { frontLeft: -1, frontRight: 1, rearLeft: 1, rearRight: -1 },
    "a strafe spins the diagonals against each other",
  );

  const arm = new TwoLinkArm(1, 1);
  const hand = arm.tip(0.5, 0.7);
  const joints = arm.jointsFor(hand.x, hand.y, Elbow.Up);
  assert.ok(Math.abs(joints.shoulder - 0.5) < 1e-4 && Math.abs(joints.elbow - 0.7) < 1e-4);
  assert.strictEqual(arm.jointsFor(5, 0, Elbow.Up), null, "out of reach");
  assert.deepStrictEqual(arm.reach, { min: 0, max: 2 });

  const link = { a: 1, alpha: 0, d: 0, theta: 0 };
  const tool = forwardKinematics([link, link]);
  assert.ok(Math.abs(tool.position.x - 2) < 1e-6 && Math.abs(tool.position.y) < 1e-6);
  assert.deepStrictEqual(forwardKinematics([]).elements, Transform.identity().elements);
  assert.strictEqual(Transform.ofJoint(link).elements[3], 1);

  const odometry = new Odometry();
  const pose = odometry.integrate(1, 1, Math.PI / 2);
  assert.ok(Math.abs(pose.x - 1) < 1e-5 && Math.abs(pose.y - 1) < 1e-5, "a quarter circle");
  assert.deepStrictEqual(odometry.integrate(NaN, 1, 1), pose, "a bad sample leaves the pose");
  odometry.integrateWheels(0.1, 0.1, drive);
  assert.ok(odometry.pose.y > pose.y, "rolling on along the new heading");

  const follower = new WaypointFollower(1.5, 3, 1.5, 1);
  const here = { latitude: 0, longitude: 0 };
  const east = { latitude: 0, longitude: 0.01 };
  const guidance = follower.guide(here, 90, east);
  assert.strictEqual(guidance.arrived, false);
  assert.ok(Math.abs(guidance.twist.vx - 1.5) < 1e-3, "pointed at it, so cruise");
  assert.strictEqual(follower.guide(here, 90, here).arrived, true);
  assert.deepStrictEqual(
    obstacleStop({ vx: 1, vy: 0, omega: 0.5 }, NaN, 0.5),
    { vx: 0, vy: 0, omega: 0.5 },
    "a range that is not a number is an obstacle",
  );

  const gate = new SafetyGate(new Limits(1, 2, 0.5, 4), 0.2);
  gate.feed();
  assert.ok(Math.abs(gate.command({ vx: 1, vy: 0, omega: 0 }, 0.1).vx - 0.05) < 1e-6);
  gate.engageEstop();
  assert.ok(gate.isStopped);
  assert.strictEqual(gate.command({ vx: 1, vy: 0, omega: 0 }, 0.1).vx, 0);
  const dog = new Watchdog(0.5);
  assert.strictEqual(dog.update(NaN), true, "an unknown silence expires the watchdog");
  const estop = new EStop();
  estop.engage();
  assert.strictEqual(estop.gate({ vx: 1, vy: 0, omega: 0 }).vx, 0);

  const servo = ServoMap.standard();
  assert.strictEqual(servo.pulse(90), 1500);
  assert.strictEqual(servo.pulse(NaN), 0, "no pulse for an angle that is not a number");
  assert.throws(() => new ServoMap(-1, 2000, 180), /minUs must be a whole number of microseconds/);
  assert.strictEqual(Esc.bidirectional().pulse(0.5), 1750);
  const encoder = new Quadrature();
  for (const [a, b] of [[false, true], [true, true], [true, false], [false, false]]) {
    assert.strictEqual(encoder.update(a, b), 1);
  }
  assert.strictEqual(encoder.count, 4);
  const scale = new QuadratureScale(360, 0.05);
  assert.ok(Math.abs(scale.distance(360) - 2 * Math.PI * 0.05) < 1e-6);
  assert.throws(() => scale.distance(1.5), /count must be a whole number of steps/);
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
    "an active-low relay is energized by a low level",
  );

  const relay = gpio.Switch.activeLow(new gpio.PinScript());
  assert.strictEqual(relay.isAsserted, false, "a switch starts off and drives nothing");
  relay.set(true);
  relay.set(false);
  assert.deepStrictEqual(
    relay.release().driven,
    [gpio.PinLevel.Low, gpio.PinLevel.High],
    "an active-low switch drives low to turn on and high to turn off",
  );
  const button = gpio.Contact.activeLow(new gpio.PinScript([gpio.PinLevel.High, gpio.PinLevel.Low]));
  assert.deepStrictEqual(
    [button.isAsserted(), button.isAsserted()],
    [false, true],
    "an active-low contact reads closed on a low level",
  );
  const idle = new gpio.PinScript();
  assert.strictEqual(idle.read(), gpio.PinLevel.High, "a script starts released and high");
  idle.drive(gpio.PinLevel.Low);
  assert.strictEqual(idle.read(), gpio.PinLevel.Low, "past its reads a script answers its driven level");

  // A line opens on Linux alone, and a missing chip is refused naming the chip and line.
  assert.throws(
    () => gpio.GpioLine.openInput("/dev/gpiochip-pamoja-absent", 27),
    process.platform === "linux"
      ? /^Error: \/dev\/gpiochip-pamoja-absent line 27: /
      : /only Linux/,
    "opening a line off Linux, or on a missing chip, says why",
  );
}

// One I2C bus shared by the program and a driver: a simulated part read through the
// BME280 driver, a script that refuses what it did not expect, and an adapter that opens
// on Linux alone.
async function buses() {
  const { I2cBus, I2cPart, I2cStep, I2cBusKind, I2cFault } = hal;
  const { Bme280, bme280 } = sensors;
  const address = bme280.addressPrimary;

  const bus = I2cBus.simulated([bme280.sim.part(address)]);
  assert.strictEqual(bus.kind, I2cBusKind.Simulated, "a bus of parts is simulated");
  const reading = await new Bme280(bus, address).measure();
  assert.strictEqual(reading.celsius.toFixed(2), "20.44", "the shipped part reads 20.44 C");
  assert.strictEqual(bus.transfers, 11, "initializing and one measurement is eleven transfers");
  assert.strictEqual(bus.waitedMicros, 2_000 + 9_300, "the start-up and one measurement's wait");
  const held = bus.part(address);
  assert.strictEqual(
    bme280.ctrlMeasFromBits(held.register(bme280.register.ctrlMeas)).mode,
    bme280.mode.forced,
    "the part keeps what the driver last wrote",
  );
  assert.strictEqual(bus.part(0x10), null, "no part at an empty address");
  assert.strictEqual(bus.remaining, null, "only a script has steps left");

  await assert.rejects(
    new Bme280(I2cBus.simulated(), address).init(),
    /^Error: nothing answered at 0x76$/,
    "an empty bus names the address nothing answered at",
  );
  const bmp280 = new I2cPart(address);
  bmp280.load(bme280.register.chipId, Buffer.from([0x58]));
  await assert.rejects(
    new Bme280(I2cBus.simulated([bmp280]), address).init(),
    /identification mismatch/,
    "a part that is not a BME280 is refused",
  );

  const script = I2cBus.scripted([
    I2cStep.writeRead(address, Buffer.from([bme280.register.chipId]), Buffer.from([bme280.chipId])),
    I2cStep.fault(address, I2cFault.Bus),
  ]);
  assert.strictEqual(script.remaining, 2, "a script starts with every step");
  assert.throws(
    () => script.write(address, Buffer.from([0x00])),
    /i2c script step 0/,
    "a transfer the script does not expect is refused",
  );
  assert.strictEqual(
    script.writeRead(address, Buffer.from([bme280.register.chipId]), 1)[0],
    bme280.chipId,
    "a matching transfer gets the scripted reply",
  );
  assert.throws(() => script.read(address, 1), /i2c script fault/, "a fault step fails the transfer");
  assert.strictEqual(script.remaining, 0, "and the script is spent");
  assert.throws(
    () => script.attach(new I2cPart(address)),
    /only a simulated bus takes parts/,
    "a script takes no parts",
  );

  assert.throws(
    () => I2cBus.open("/dev/i2c-pamoja-absent"),
    process.platform === "linux" ? /^Error: \/dev\/i2c-pamoja-absent: / : /only Linux/,
    "opening an adapter off Linux, or a missing one, says why",
  );
}
// Every I2C part's driver against its simulated twin, all on one bus at the addresses a
// board would give them, and the three kinds of simulated part the bus hands back.
async function sensorDrivers() {
  const { I2cBus, I2cPart, WordPart, CommandPart } = hal;
  const s = sensors;
  const tmp117At = s.tmp117.address.add0Vplus;
  const ads1115At = s.ads1115.addressSda;
  const opt3001At = s.opt3001.addressScl;
  const ina219At = s.ina219.baseAddress + 1;
  const ina226At = 0x45;

  const bus = I2cBus.simulated([
    s.bmp280.sim.reporting(s.bmp280.addressSecondary, -7.5, 1003.0),
    s.tmp117.sim.part(tmp117At),
    s.opt3001.sim.reporting(opt3001At, 1200),
    s.hdc1080.sim.reporting(-20.0, 12.5),
    s.ina219.sim.part(ina219At),
    s.ina226.sim.reporting(ina226At, 2, 20_000_000, 3_300_000, -10_000_000),
    s.ads1115.sim.reporting(ads1115At, s.ads1115.pga.fsr4_096, 3.0),
    s.sht3x.sim.reporting(s.sht3x.addressA, 30.0, 70.0),
    s.scd4x.sim.part(),
  ]);

  const pressure = await new s.Bmp280(bus, s.bmp280.addressSecondary).measure();
  assert.ok(Math.abs(pressure.celsius + 7.5) < 0.01, "the BMP280 reads what its twin reports");
  assert.ok(Math.abs(pressure.hectopascals - 1003.0) < 0.01, "and the pressure");

  const thermometer = new s.Tmp117(bus, tmp117At, { averaging: s.tmp117.averaging.x8 });
  assert.strictEqual(thermometer.siliconRevision, null, "no revision before initialization");
  assert.strictEqual((await thermometer.measure()).celsius, s.tmp117.sim.celsius, "21.25 C");
  assert.notStrictEqual(thermometer.siliconRevision, null, "the revision is read at init");
  await thermometer.setAlertLimits(30, 10);
  assert.deepStrictEqual(await thermometer.alerts(), { high: false, low: false }, "no alerts");

  const light = new s.Opt3001(bus, opt3001At, { longConversion: false });
  assert.strictEqual((await light.measure()).lux, 1200, "the OPT3001 reads 1200 lux");
  assert.strictEqual(light.configuration.longConversion, false, "at the short conversion");

  const climate = new s.Hdc1080(bus);
  const air = await climate.measure();
  assert.ok(Math.abs(air.celsius + 20) < 0.003, "the HDC1080 reads -20 C");
  assert.ok(Math.abs(air.relativeHumidity - 12.5) < 0.002, "and 12.5 %");
  assert.throws(
    () => new s.Hdc1080(bus, { temperatureResolutionBits: 12 }),
    /14 or 11 bits/,
    "a resolution the part does not have is refused",
  );

  const solar = new s.Ina219(bus, ina219At);
  const panel = await solar.measure();
  assert.strictEqual(panel.busMillivolts, s.ina219.sim.busMillivolts, "12 V on the bus");
  assert.ok(Math.abs(panel.currentMicroamps - s.ina219.sim.microamps) < panel.currentLsbMicroamps);
  assert.strictEqual(solar.currentLsbMicroamps, s.ina219.minimumCurrentLsbMicroamps(3_200_000));

  const battery = new s.Ina226(bus, ina226At, { shuntMilliohms: 2, maxMicroamps: 20_000_000 });
  assert.strictEqual(battery.identity, null, "no identity before initialization");
  const cell = await battery.measure();
  assert.ok(Math.abs(cell.currentAmps + 10) < 0.001, "10 A flowing out of the battery");
  assert.strictEqual(battery.identity.device, s.ina226.deviceId, "an INA226 answered");

  const adc = new s.Ads1115(bus, ads1115At, { pga: s.ads1115.pga.fsr4_096 });
  const probe = await adc.sample();
  assert.ok(Math.abs(probe.volts - 3.0) < 0.001, "the ADS1115 reads 3 V");
  assert.strictEqual(probe.pga, s.ads1115.pga.fsr4_096, "at the range it was built for");

  const humidity = new s.Sht3x(bus, s.sht3x.addressA, { repeatability: s.Sht3xRepeatability.Low });
  assert.ok(Math.abs((await humidity.measure()).celsius - 30) < 0.003, "the SHT3x reads 30 C");
  assert.strictEqual(humidity.lastStatus.bits, s.sht3x.statusDefault, "the status after reset");
  await humidity.heaterOn();

  const co2 = new s.Scd4x(bus);
  assert.strictEqual((await co2.measure()).co2Ppm, s.scd4x.sim.co2Ppm, "the SCD4x reads 800 ppm");
  assert.strictEqual(co2.serial, s.scd4x.sim.serial, "and its serial");
  assert.strictEqual(await co2.dataReady(), true, "a result is always waiting");

  const sht = bus.part(s.sht3x.addressA);
  assert.ok(sht instanceof CommandPart, "a command part comes back as one");
  assert.deepStrictEqual(sht.received[0], Buffer.from([0x30, 0xa2]), "the reset went first");
  const tmp = bus.part(tmp117At);
  assert.ok(tmp instanceof WordPart, "a word part comes back as one");
  assert.strictEqual(tmp.word(s.tmp117.register.thighLimit), s.tmp117.rawFromCelsius(30) & 0xffff);
  assert.ok(bus.part(s.bmp280.addressSecondary) instanceof I2cPart, "and a byte part as one");

  const words = new WordPart(0x48);
  words.set(0x01, 0x2000);
  words.readOnly(0x01, 0xf000);
  const commands = new CommandPart(0x44);
  commands.answer(Buffer.from([0xf3, 0x2d]), Buffer.from([0x80, 0x10, 0xe1]));
  const handmade = I2cBus.simulated([words, commands]);
  handmade.write(0x48, Buffer.from([0x01, 0x00, 0x20]));
  assert.deepStrictEqual(
    handmade.writeRead(0x48, Buffer.from([0x01]), 2),
    Buffer.from([0x20, 0x20]),
    "the flag the part keeps survives a write",
  );
  handmade.write(0x44, Buffer.from([0xf3, 0x2d]));
  assert.deepStrictEqual(handmade.read(0x44, 3), Buffer.from([0x80, 0x10, 0xe1]));
  assert.throws(() => handmade.read(0x44, 3), /no command had left a reply/, "the reply was taken");

  const text = s.ds18b20.parseW1Slave(
    `${[...s.ds18b20.buildScratchpad(21.5, 12, 75, -10)].map((byte) => byte.toString(16).padStart(2, "0")).join(" ")} : crc=00 YES\n`,
  );
  assert.strictEqual(text.microCelsius, 21_500_000, "the kernel's text decodes");
  const rendered = s.ds18b20.w1SlaveText(s.ds18b20.buildScratchpad(21.5, 12, 75, -10))
  assert.ok(rendered.endsWith(' t=21500\n'), "the kernel prints millidegrees on the second line");
  assert.strictEqual(s.ds18b20.parseW1Slave(rendered).microCelsius, 21_500_000, "and it reads back");
  assert.throws(
    () => s.Ds18b20Thermometer.discover("/pamoja-absent-w1"),
    /pamoja-absent-w1/,
    "a missing device directory is named",
  );
  assert.strictEqual(s.Ds18b20Thermometer.forSerial("000005e2fdc3").serial, "000005e2fdc3");
  assert.strictEqual(s.Ds18b20Thermometer.at("/tmp/w1_slave").serial, null, "a bare file has no serial");
}

// The PCA9685 driven over a simulated part that keeps its datasheet's rules: the prescale for
// 50 Hz only lands while the part sleeps, a servo channel reads back as loaded, one ALL_LED
// write reaches every channel, and a channel the part does not have is refused.
async function actuatorDrivers() {
  const { I2cBus } = hal;
  const { Pca9685, pca9685, pwm } = actuators;
  const address = pca9685.defaultAddress;
  const bus = I2cBus.simulated([pca9685.sim.part(address)]);
  const board = new Pca9685(bus, address, { frequencyHz: 50 });
  assert.strictEqual(board.prescale, 121, "round(25 MHz / 4096 / 50) - 1");
  await board.setChannel(0, pwm.servo(1500));
  assert.strictEqual(bus.waitedMicros, pca9685.oscillatorStartupMicros, "the oscillator's start-up");
  let part = bus.part(address);
  assert.strictEqual(part.register(pca9685.register.preScale), 121);
  assert.strictEqual(part.register(pca9685.register.mode1), pca9685.mode1.autoIncrement);
  const first = pca9685.channelRegister(0);
  const loaded = [0, 1, 2, 3].map((offset) => part.register(first + offset));
  assert.deepStrictEqual(Buffer.from(loaded), pwm.servo(1500), "the servo channel reads back");
  assert.deepStrictEqual(await board.channel(0), pwm.servo(1500), "the driver reads it back too");
  await assert.rejects(board.channel(16), /sixteen channels/);
  await board.setAll(pwm.fullOff());
  part = bus.part(address);
  assert.strictEqual(part.register(pca9685.channelRegister(15) + 3), 0x10, "every channel off");
  await assert.rejects(board.setChannel(16, pwm.fullOn()), /sixteen channels/);
  await assert.rejects(board.softwareReset(), /nothing answered at 0x00/);
}

// A serial port over each kind of line a test reaches: a pair carries bytes both ways, a
// looped line reads back what it wrote, a script refuses a write it did not expect, and a read
// with nothing coming resolves at once and counts its timeout.
async function serialPorts() {
  const { SerialPort, SerialStep, Parity, SerialPortKind } = hal;
  const modbus = { baud: 9600, parity: Parity.Even };
  assert.strictEqual(SerialPort.bitsPerCharacter(modbus), 11, "start, eight data, parity, stop");
  assert.strictEqual(SerialPort.characterNanos(modbus), 1145834);
  assert.strictEqual(SerialPort.transferMicros(modbus, 8), 9167);

  const [gateway, node] = SerialPort.pair({ baud: 115200 });
  await node.write(Buffer.from("t=21.5"));
  assert.deepStrictEqual(await gateway.read(16, 100), Buffer.from("t=21.5"));
  const started = Date.now();
  assert.strictEqual((await gateway.read(16, 250)).length, 0);
  assert.ok(Date.now() - started < 200, "a simulated read does not wait");
  assert.strictEqual(gateway.waitedMicros, 250000);
  assert.strictEqual(gateway.kind, SerialPortKind.Paired);
  assert.deepStrictEqual(gateway.settings, { baud: 115200, parity: Parity.None, stopBits: 1 });

  const line = SerialPort.looped(modbus);
  await line.write(Buffer.from([1, 2, 3]));
  assert.deepStrictEqual(await line.read(2, 0), Buffer.from([1, 2]));
  line.discardInput();
  assert.strictEqual((await line.read(8, 0)).length, 0);
  await line.wait(2);
  assert.strictEqual(line.waitedMicros, 2000);
  assert.strictEqual(line.written, 3);
  assert.strictEqual(line.received, 2);

  const script = SerialPort.scripted({ baud: 9600 }, [
    SerialStep.write(Buffer.from("?")),
    SerialStep.read(Buffer.from("42")),
  ]);
  await assert.rejects(script.write(Buffer.from("!")), /expected 3f/);
  await script.write(Buffer.from("?"));
  assert.deepStrictEqual(await script.read(4, 10), Buffer.from("42"));
  assert.strictEqual(script.remaining, 0);
  assert.strictEqual(line.remaining, null);

  assert.throws(() => SerialPort.looped({ baud: 9600, stopBits: 3 }), /3 stop bits/);
  if (process.platform !== "linux") {
    assert.throws(() => SerialPort.open("/dev/serial0", { baud: 115200 }), /only Linux/);
  }
}

// A Modbus client polls devices on a simulated line as it would a real one: it reads and writes
// every table, a broadcast reaches every device and draws no answer, a refusal comes back as the
// device's exception, and a unit that never answers times out after the response timeout,
// counted rather than waited.
async function modbusClients() {
  const { ModbusClient, ModbusClientError, ModbusLine, ModbusServer, BROADCAST, ExceptionCode } =
    modbus;
  const { Parity } = hal;
  const settings = { baud: 19200, parity: Parity.Even };
  assert.strictEqual(ModbusClient.frameGapNanos({ baud: 9600, parity: Parity.Even }), 4010419);
  assert.strictEqual(ModbusClient.frameGapNanos(settings), 2005210);
  assert.strictEqual(ModbusClient.frameGapNanos({ baud: 115200 }), 1750000);

  const meter = new ModbusServer(17);
  meter.setHoldingRegisters(107, [2301, 418, 0]);
  meter.setCoils(0, [false, false]);
  const pump = new ModbusServer(18);
  pump.setHoldingRegisters(109, [0]);
  const line = new ModbusLine();
  line.attach(meter);
  line.attach(pump);
  assert.strictEqual(line.count, 2);
  const port = line.port(settings);
  const client = new ModbusClient(port, { responseTimeoutMs: 250 });
  assert.strictEqual(client.responseTimeoutMs, 250);
  assert.strictEqual(client.turnaroundMs, 100);

  assert.deepStrictEqual(await client.readHoldingRegisters(17, 107, 3), [2301, 418, 0]);
  await client.writeSingleCoil(17, 1, true);
  assert.deepStrictEqual(await client.readCoils(17, 0, 2), [false, true]);
  await client.writeMultipleRegisters(17, 107, [2300, 420]);
  assert.strictEqual(meter.holdingRegister(108), 420);
  assert.strictEqual(meter.holdingRegister(110), null);

  await client.writeSingleRegister(BROADCAST, 109, 5);
  assert.strictEqual(meter.holdingRegister(109), 5);
  assert.strictEqual(pump.holdingRegister(109), 5);
  await assert.rejects(
    client.readHoldingRegisters(BROADCAST, 107, 1),
    (error) => error instanceof ModbusClientError && error.kind === "BroadcastRead",
  );

  await assert.rejects(client.readHoldingRegisters(17, 108, 3), (error) => {
    assert.ok(error instanceof ModbusClientError);
    assert.strictEqual(error.kind, "Exception");
    assert.strictEqual(error.exception, ExceptionCode.IllegalDataAddress);
    assert.strictEqual(error.functionCode, 3);
    assert.strictEqual(error.unit, 17);
    return true;
  });

  const before = port.waitedMicros;
  await assert.rejects(
    client.readHoldingRegisters(19, 0, 1),
    (error) => error.kind === "Timeout" && error.received === 0,
  );
  assert.strictEqual(port.waitedMicros - before, 2005 + 250000);
  assert.strictEqual(meter.served, 5, "the refusal is not counted");

  assert.throws(() => new ModbusServer(0), /broadcast/);
  assert.strictEqual(meter.answer(modbus.readHoldingRegisters(18, 109, 1)), null);
  assert.ok(meter.answer(modbus.readHoldingRegisters(17, 109, 1)).length > 0);
}
// A CAN bus as SocketCAN behaves: every node hears every frame but its own, keeps what its
// filters pass, and a receive with nothing waiting resolves at once and counts its timeout.
async function canBuses() {
  const { CanBus, CanBusKind, filterPgn, filterExact, filterMatches, broadcastJ1939, frame, fdFrame, remoteFrame } = can;
  const speed = broadcastJ1939(3, 61444, 0x00);
  const engine = CanBus.simulated();
  const gateway = engine.join();
  const laptop = gateway.join();
  assert.strictEqual(engine.kind, CanBusKind.Simulated);
  assert.strictEqual(engine.interface, null);

  gateway.setFilters([filterPgn(61444)]);
  await engine.send(frame(0x20a, Buffer.from([1, 2])));
  await engine.send(frame(speed, Buffer.alloc(8, 0xff), true));
  await engine.send(fdFrame(0x123, Buffer.alloc(32, 0xa5)));
  await engine.send(remoteFrame(0x301, 4));

  const kept = await gateway.receive(10);
  assert.strictEqual(kept.id, speed);
  assert.strictEqual(kept.extended, true);
  assert.strictEqual(await gateway.receive(10), null);

  const heard = [];
  let next;
  while ((next = await laptop.receive(0)) !== null) heard.push(next);
  assert.deepStrictEqual(heard.map((f) => [f.id, f.fd, f.remote, f.len]), [
    [0x20a, false, false, 2],
    [speed, false, false, 8],
    [0x123, true, false, 32],
    [0x301, false, true, 4],
  ]);
  assert.strictEqual(await engine.receive(250), null, "a node does not hear itself");
  assert.strictEqual(engine.waitedMicros, 250000);
  assert.deepStrictEqual([engine.sent, gateway.received, laptop.received], [4, 1, 4]);

  gateway.setFilters([]);
  await engine.send(frame(speed, Buffer.alloc(8), true));
  assert.strictEqual(await gateway.receive(0), null, "an empty filter list keeps nothing");
  gateway.clearFilters();
  await engine.send(frame(0x20a, Buffer.from([3])));
  assert.ok(await gateway.receive(0));

  const exact = filterExact(0x20a);
  assert.ok(filterMatches(exact, 0x20a));
  assert.ok(!filterMatches(exact, 0x20a, true));
  if (process.platform !== "linux") {
    assert.throws(() => CanBus.open("can0"), /only Linux/);
  }
}
// The stepper drivers walk the same coil pairs and pulse the same lines as the Rust
// drivers' own tests, with every wait counted rather than slept.
async function stepperDrivers() {
  const { DelayLog } = hal;
  const { PinScript, PinLevel } = gpio;
  const { FourWire, StepDir, StepDrive, stepper } = actuators;
  const { High, Low } = PinLevel;
  const lines = () => [new PinScript(), new PinScript(), new PinScript(), new PinScript()];

  const delay = new DelayLog();
  const motor = new FourWire(lines(), StepDrive.FullStep, { stepMicros: 1_500, delay });
  await motor.steps(4);
  assert.strictEqual(motor.position, 4);
  assert.strictEqual(motor.drive, StepDrive.FullStep);
  const [a, b, c, d] = motor.release();
  assert.deepStrictEqual(a.driven, [Low, Low, High, High]);
  assert.deepStrictEqual(b.driven, [High, Low, Low, High]);
  assert.deepStrictEqual(c.driven, [High, High, Low, Low]);
  assert.deepStrictEqual(d.driven, [Low, High, High, Low]);
  assert.deepStrictEqual(delay.waitsMicros, [1_500, 1_500, 1_500, 1_500]);
  assert.strictEqual(delay.totalMillis, 6);

  const wave = new FourWire(lines(), StepDrive.Wave, { delay: new DelayLog() });
  await wave.steps(-2);
  assert.strictEqual(wave.position, -2);
  wave.idle();
  const [waveA, , , waveD] = wave.release();
  assert.deepStrictEqual(waveA.driven, [Low, Low, Low]);
  assert.deepStrictEqual(waveD.driven, [High, Low, Low], "wave drive backward starts at coil D");

  const pulses = new DelayLog();
  const carriage = new StepDir(new PinScript(), new PinScript(), {
    pulseMicros: 5,
    stepMicros: 1_000,
    delay: pulses,
  });
  await carriage.steps(2);
  await carriage.steps(-1);
  assert.strictEqual(carriage.position, 1);
  const [step, direction] = carriage.release();
  assert.deepStrictEqual(direction.driven, [High, High, Low]);
  assert.deepStrictEqual(step.driven, [High, Low, High, Low, High, Low]);
  assert.deepStrictEqual(pulses.waitsMicros.slice(0, 3), [5, 5, 1_000]);

  const defaults = new StepDir(new PinScript(), new PinScript(), { delay: new DelayLog() });
  assert.strictEqual(defaults.pulseMicros, stepper.defaultPulseMicros);
  assert.strictEqual(defaults.stepMicros, stepper.defaultStepMicros);
  assert.strictEqual(stepper.defaultStepMicros, 2_000);
  assert.strictEqual(stepper.defaultPulseMicros, 10);

  const failing = { drive: () => { throw new Error("line unplugged"); } };
  const stuck = new FourWire([failing, failing, failing, failing], StepDrive.Wave, {
    delay: new DelayLog(),
  });
  await assert.rejects(stuck.step("Forward"), /line unplugged/);
  assert.strictEqual(stuck.position, 0, "a step that could not be driven is not counted");
}

// The seven parts added after the first four: a datasheet figure each, and the
// input each one is meant to refuse.
function laterSensors() {
  const measurement = sensors.sht3x.parseMeasurement(
    sensors.sht3x.measurementBytes(0x6666, 0x9999),
  );
  assert.strictEqual(measurement.milliCelsius, 25000, "0x6666 is two fifths of full scale");
  assert.strictEqual(measurement.milliPercent, 60000, "and 0x9999 is three fifths");
  assert.strictEqual(sensors.sht3x.crc(Buffer.from([0xbe, 0xef])), 0x92, "Sensirion's check value");

  const air500 = sensors.scd4x.measurementFromPhysical(500, 25000, 37000);
  const frame = sensors.scd4x.measurementBytes(
    air500.co2Ppm,
    air500.temperatureRaw,
    air500.humidityRaw,
  );
  const air = sensors.scd4x.parseMeasurement(frame);
  assert.strictEqual(air.co2Ppm, 500, "the carbon dioxide word survives the frame");
  const corruptFrame = Buffer.from(frame);
  corruptFrame[2] ^= 0xff;
  assert.throws(
    () => sensors.scd4x.parseMeasurement(corruptFrame),
    "a flipped checksum byte should throw",
  );

  assert.strictEqual(sensors.tmp117.microCelsius(0x0c80), 25000000, "the datasheet's 25 C row");
  assert.strictEqual(sensors.tmp117.microCelsius(-1), -7812, "and one count below zero");

  assert.strictEqual(sensors.hdc1080.milliCelsius(0x8000), 42500, "mid-scale on the HDC1080");
  assert.throws(
    () => sensors.hdc1080.configFromRegister(0x1300),
    "the undefined humidity-resolution code should throw",
  );

  assert.strictEqual(sensors.opt3001.milliLux(0xbfff), 83865600, "the OPT3001 full scale");
  assert.strictEqual(
    sensors.opt3001.fullScaleMilliLux(12),
    null,
    "a reserved range number has no full scale",
  );

  assert.strictEqual(sensors.ina226.calibration(1000, 2), 2560, "the INA226 design example");
  assert.strictEqual(
    sensors.ina226.powerMicrowatts(4792, 1000),
    119800000,
    "which reads 119.8 W at the example's load",
  );
  assert.throws(
    () => sensors.ina226.identify(0x5449, 0x2270),
    "a die that is not an INA226 should throw",
  );
  const { averaging, conversionTime, mode } = sensors.ina226;
  assert.strictEqual(
    sensors.ina226.configToRegister({
      reset: false,
      averaging: averaging.samples1,
      busConversionTime: conversionTime.us1100,
      shuntConversionTime: conversionTime.us1100,
      mode: mode.shuntAndBusContinuous,
    }),
    sensors.ina226.configReset,
    "the named settings spell the power-on register",
  );
  assert.strictEqual(
    sensors.ina226.configFromRegister(0x4527).averaging,
    averaging.samples16,
    "and read back by name",
  );

  const coefficients = sensors.bmp280.calibration(
    Buffer.from(
      "70 6b 43 67 18 fc 7d 8e 43 d6 d0 0b 27 0b 8c 00 f9 ff 8c 3c f8 c6 70 17".replace(/ /g, ""),
      "hex",
    ),
  );
  const air280 = coefficients.compensate(Buffer.from("655ac07eed00".replace(/ /g, ""), "hex"));
  assert.ok(air280.pascals > 90000 && air280.pascals < 110000, "the BMP280 reads a sane pressure");
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
    "fully off is its own flag in LEDn_OFF_H",
  );
  assert.deepStrictEqual(
    actuators.pwm.duty(0),
    actuators.pwm.fullOff(),
    "the datasheet rules out the same count in on and off",
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
  assert.ok(Math.abs(trend.slope - 1) < 1e-4, "a rising signal has a positive slope");

  const anomaly = new Anomaly(3);
  for (let i = 0; i < 8; i += 1) anomaly.check(20);
  assert.ok(anomaly.check(900), "a reading far outside the window is flagged");
  assert.ok(anomaly.check(NaN), "and so is one that is not a number");

  const small = new Median(3);
  const wide = new Median();
  for (let i = 0; i < 10; i += 1) {
    small.update(10);
    wide.update(10);
  }
  small.update(20);
  wide.update(20);
  assert.strictEqual(small.update(20), 20, "a small median follows a real change");
  assert.strictEqual(wide.update(20), 10, "a wide one follows it late");
  assert.strictEqual(small.capacity, 3, "and says how many readings it keeps");
  assert.throws(() => new Median(0), /capacity must be a whole number from 1 to 32, not 0/);
  assert.throws(() => new Window(33), /capacity must be a whole number from 1 to 32, not 33/);
  assert.throws(() => new Trend(2.5), /capacity must be a whole number from 2 to 32, not 2.5/);
  assert.throws(() => new Trend(1), /from 2 to 32, not 1/, "a line needs two readings");
  assert.throws(() => new Anomaly(3, 1), /from 2 to 32, not 1/, "and so does a spread");
  assert.strictEqual(new Anomaly(3, 8).capacity, 8);

  const lastFew = new Window(2);
  [1, 2, 3].forEach((value) => lastFew.push(value));
  assert.ok(lastFew.isFull, "a small window fills");
  assert.strictEqual(lastFew.oldest(), 2, "and drops its oldest");
  assert.strictEqual(lastFew.latest(), 3);
  lastFew.push(NaN);
  assert.strictEqual(lastFew.mean(), 2.5, "a reading that is not a number is not kept");

  assert.throws(() => new Debounce(2.9, false), /samples must be a whole number from 0 to 65535, not 2.9/);
  assert.throws(() => new Debounce(-1, false), /not -1/);
  assert.strictEqual(new Debounce(3, true).state, true, "a debouncer starts where it is told");

  const pid = new Pid(2, 0.5, 0);
  const before = pid.update(10, 7, 1);
  assert.strictEqual(pid.update(10, NaN, 1), before, "a missing reading holds the last output");
  assert.ok(Number.isFinite(pid.update(10, 7, 1)), "and does not poison the controller");

  const level = Trigger.below(20, 2);
  assert.strictEqual(level.watchesAbove, false, "a below trigger watches a falling reading");
  assert.strictEqual(level.update(NaN), null, "a reading that is not a number fires nothing");
}

// Talking to an autopilot: framing a message, reading it back off a link that
// splits and garbles it, and proving a signed frame came from who it claims.
function mavlinkWire() {
  const header = { systemId: 1, componentId: 1, sequence: 7 };
  // HEARTBEAT announcing an onboard controller in an active state.
  const heartbeat = Buffer.from([0, 0, 0, 0, 18, 0, 0, 4, 3]);

  assert.strictEqual(mavlink.knownCrcExtra(0), 50, "HEARTBEAT's published CRC_EXTRA");
  assert.strictEqual(mavlink.knownCrcExtra(9999), null, "an id outside the common dialect");

  const frame = mavlink.frame(header, 0, heartbeat);
  assert.strictEqual(frame.version, mavlink.MavlinkVersion.V2, "v2 is the current format");
  assert.strictEqual(frame.messageId, 0);
  assert.strictEqual(frame.signed, false, "an ordinary frame carries no signature");
  assert.strictEqual(frame.signature, null);
  assert.deepStrictEqual(frame.header, header, "the addressing fields survive");

  const received = mavlink.MavlinkFrame.parseKnown(frame.bytes);
  assert.strictEqual(received.messageId, 0, "and the frame reads back");
  assert.deepStrictEqual(received.payload, heartbeat, "with its payload intact");

  // A frame mangled in transit is refused rather than acted on.
  const mangled = Buffer.from(frame.bytes);
  mangled[12] ^= 0xff;
  assert.throws(
    () => mavlink.MavlinkFrame.parseKnown(mangled),
    /checksum|CRC/i,
    "a corrupt frame does not reach the application",
  );

  // A parser joins a stream already in progress and survives arbitrary splits.
  const parser = new mavlink.MavlinkParser();
  assert.deepStrictEqual(
    parser.push(Buffer.from([0x11, 0x22, 0x33])),
    [],
    "noise between frames is skipped, not reported",
  );
  const wire = frame.bytes;
  assert.deepStrictEqual(parser.push(wire.subarray(0, 5)), [], "half a frame is not a frame");
  const found = parser.push(wire.subarray(5));
  assert.strictEqual(found.length, 1, "the rest of it completes one");
  assert.strictEqual(found[0].messageId, 0);

  // The queueing form, for a caller that drains on its own schedule.
  parser.feed(wire);
  assert.strictEqual(parser.pending, 1);
  assert.strictEqual(parser.nextFrame().messageId, 0);
  assert.strictEqual(parser.nextFrame(), null, "an empty parser means feed it more");

  // A private dialect: describe the message once, and it checks from then on.
  const dialect = new mavlink.Dialect();
  const seed = dialect.addMessage(50_000, "PRIVATE_STATUS", [
    { typeName: "uint32_t", fieldName: "uptime" },
  ]);
  assert.strictEqual(
    seed,
    mavlink.messageCrcExtra("PRIVATE_STATUS", [{ typeName: "uint32_t", fieldName: "uptime" }]),
    "the seed is derived, not invented",
  );
  assert.strictEqual(dialect.crcExtra(50_000), seed);
  assert.strictEqual(dialect.crcExtra(0), 50, "and the common dialect still answers");

  const privateFrame = mavlink.MavlinkFrame.raw(
    header,
    50_000,
    seed,
    Buffer.from(new Uint32Array([42]).buffer),
  );
  assert.throws(
    () => mavlink.MavlinkFrame.parseKnown(privateFrame.bytes),
    /50000|unknown/i,
    "the common registry alone cannot check a private message",
  );
  const privateBack = mavlink.MavlinkFrame.parseKnown(privateFrame.bytes, dialect);
  assert.strictEqual(privateBack.messageId, 50_000, "but the dialect can");
  // MAVLink 2 drops trailing zero bytes, so a four-byte field holding 42 arrives
  // as one byte; a decoder zero-extends it.
  assert.deepStrictEqual(privateBack.payload, Buffer.from([42]));

  // Signing: a ground station trusts a command came from the vehicle it expects.
  const key = Buffer.alloc(mavlink.KEY_LEN, 7);
  const signer = new mavlink.MavlinkSigner(key, 1, mavlink.timestampNow());
  assert.strictEqual(signer.linkId, 1);
  const signed = signer.sign(header, 0, heartbeat, 50);
  assert.strictEqual(signed.signed, true);
  assert.strictEqual(signed.signature.length, mavlink.SIGNATURE_LEN);
  assert.strictEqual(signed.signature[0], 1, "the link id leads the signature block");

  const verifier = new mavlink.MavlinkVerifier(key);
  verifier.verify(signed);
  assert.throws(
    () => verifier.verify(signed),
    /replay|timestamp/i,
    "the same frame a second time is a replay",
  );
  assert.throws(
    () => new mavlink.MavlinkVerifier(Buffer.alloc(mavlink.KEY_LEN, 9)).verify(signed),
    /signature/i,
    "and a different key is a different sender",
  );

  // An unsigned frame is not silently treated as authentic.
  assert.throws(() => new mavlink.MavlinkVerifier(key).verify(frame), /signed/i);

  assert.strictEqual(
    mavlink.crc16(Buffer.from("123456789")),
    mavlink.crc16(Buffer.from("123456789")),
    "the checksum is a pure function of its input",
  );
}

// Budgeting airtime, framing a mesh packet, routing it, and securing a LoRaWAN
// uplink: everything a node needs to reach a network it cannot see.
// Message shapes: filling a message in by name, and describing one this build has never
// heard of so a vendor dialect needs no code here.
function mavlinkShapes() {
  const heartbeat = mavlink.schemaFor("HEARTBEAT");
  assert.strictEqual(heartbeat.id, 0, "HEARTBEAT's id");
  assert.strictEqual(heartbeat.crcExtra, mavlink.knownCrcExtra(0), "and its published seed");
  assert.strictEqual(heartbeat.wireLen, 9, "and its length on the wire");
  assert.strictEqual(
    heartbeat.fields[0].name,
    "custom_mode",
    "wire order puts the 32-bit field first",
  );
  assert.ok(mavlink.knownMessages().includes("GLOBAL_POSITION_INT"), "the registry lists it");
  assert.throws(() => mavlink.schemaFor("NOT_A_MESSAGE"), "an unknown name is refused");

  // Fill it in by name and send it, then read it back the way a receiver would.
  const built = mavlink.fromObject(heartbeat, {
    type: 18, // MAV_TYPE_ONBOARD_CONTROLLER
    autopilot: 0,
    system_status: 4, // MAV_STATE_ACTIVE
    mavlink_version: 3,
  });
  const frame = built.toFrame({ systemId: 1, componentId: 1, sequence: 0 });
  assert.strictEqual(frame.messageId, 0, "the frame carries HEARTBEAT");

  const received = mavlink.MavlinkMessage.decode(heartbeat, frame.payload);
  assert.strictEqual(received.get("system_status"), 4, "the status survives the wire");
  assert.deepStrictEqual(
    mavlink.toObject(received, heartbeat).type,
    18,
    "and so does the vehicle type",
  );

  // A field the message does not have, and a value its type cannot hold.
  assert.throws(() => received.get("throttle"), "an unknown field is refused");
  assert.throws(() => built.set("type", 300), "a value past a uint8_t is refused");
  assert.throws(() => built.set("type", 1.5), "and so is a fractional one");

  // Text lives in a fixed-length char array, padded with zeros.
  const status = mavlink.schemaFor("STATUSTEXT");
  const spoken = mavlink.message(status);
  spoken.setText("text", "preflight checks passed");
  assert.strictEqual(spoken.getText("text"), "preflight checks passed", "the text reads back");

  // A private message: described once, then carried and checked like any other.
  const builder = new mavlink.MessageSchemaBuilder(50_001, "BATTERY_CELLS");
  builder.field("cell_mv", mavlink.MavlinkFieldType.UINT16, 6);
  builder.field("pack_id", mavlink.MavlinkFieldType.UINT8);
  builder.field("uptime_ms", mavlink.MavlinkFieldType.UINT32);
  const cells = builder.build();
  assert.strictEqual(
    cells.fields[0].name,
    "uptime_ms",
    "the builder puts the widest field first",
  );

  const pack = mavlink.fromObject(cells, { pack_id: 2, cell_mv: [4150, 4148, 4151, 0, 0, 0] });
  const dialect = new mavlink.Dialect();
  dialect.add(cells.id, cells.crcExtra);
  const sent = pack.toFrame({ systemId: 9, componentId: 1, sequence: 0 });
  const back = mavlink.MavlinkFrame.parseKnown(sent.bytes, dialect);
  assert.strictEqual(
    mavlink.MavlinkMessage.decode(cells, back.payload).get("cell_mv", 2),
    4151,
    "a private message survives the wire whole",
  );
}

// The service protocols: a plan crosses from a station to a vehicle one frame at a time,
// a command is matched to its acknowledgment, and a setpoint goes out as the right message.
function mavlinkProtocols() {
  const vehicle = { systemId: 1, componentId: 1, sequence: 0 };
  const station = { systemId: 255, componentId: 190, sequence: 0 };

  // Two waypoints, described by field name; the sender numbers them itself.
  const upload = new mavlink.MissionSender(1, 1);
  const itemShape = mavlink.schemaFor("MISSION_ITEM_INT");
  upload.addItem(mavlink.fromObject(itemShape, { command: 22, z: 20 }).payload);
  upload.addItem(
    mavlink.fromObject(itemShape, { command: 16, x: -338567800, y: 1512153000, z: 50 }).payload,
  );
  assert.strictEqual(upload.length, 2, "the plan holds both items");

  // The station opens the download and the two sides take turns until it is acknowledged.
  const download = new mavlink.MissionReceiver(255, 190);
  let fromVehicle = upload.onFrame(download.requestList(station), vehicle).reply;
  const accepted = [];
  for (;;) {
    const step = download.onFrame(fromVehicle, station);
    assert.ok(step !== null, "the vehicle only sends what the receiver handles");
    if (step.accepted !== null) {
      accepted.push(step.accepted.get("command"));
    }
    const answer = upload.onFrame(step.reply, vehicle);
    if (answer.kind === "finished") {
      assert.strictEqual(answer.result, 0, "the transfer is accepted");
      break;
    }
    fromVehicle = answer.reply;
  }
  assert.deepStrictEqual(accepted, [22, 16], "both items arrive in order");
  assert.ok(download.complete, "and the download is complete");

  // A command is matched to its acknowledgment, and a stray frame is passed over.
  const arm = new mavlink.CommandProtocol(400);
  assert.strictEqual(arm.confirmation, 0, "the first send carries confirmation 0");
  const ackShape = mavlink.schemaFor("COMMAND_ACK");
  const ack = mavlink.fromObject(ackShape, { command: 400, result: 0 }).toFrame(vehicle);
  assert.deepStrictEqual(arm.onFrame(ack), { kind: "final", value: 0 }, "an accepted arm");
  assert.strictEqual(arm.onFrame(fromVehicle), null, "a mission frame is not an ack");
  assert.strictEqual(arm.onTimeout(), 1, "a timeout allows a resend with confirmation 1");

  // A setpoint goes out as the right message with the right mask.
  const setpoint = mavlink.offboard.localVelocity(station, 1000, 1, 1, 1, 0.5, 0, 0);
  assert.strictEqual(setpoint.messageId, 84, "SET_POSITION_TARGET_LOCAL_NED");
  const read = mavlink.MavlinkMessage.decode(
    mavlink.schemaFor("SET_POSITION_TARGET_LOCAL_NED"),
    setpoint.payload,
  );
  assert.strictEqual(
    read.get("type_mask"),
    mavlink.offboard.typeMask(mavlink.MavlinkTypeMask.VELOCITY),
    "only the velocity fields are active",
  );
}

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

  // A published region turns a data-rate number into radio settings and reports
  // what the band allows, without ever refusing a transmission.
  const eu868 = lora.planFor(lora.LoraRegion.Eu868);
  const us915 = lora.planFor(lora.LoraRegion.Us915);
  const us915Downlink = us915.dataRate(lora.LoraDirection.Downlink, 2);
  assert.strictEqual(eu868.name, "EU863-870", "the plan names its band");
  assert.strictEqual(
    eu868.linkSettings(0).spreadingFactor,
    12,
    "EU868 DR0 is the slowest LoRa rate",
  );
  assert.strictEqual(
    eu868.dutyCyclePermille(868_100_000),
    10,
    "the 868.1 MHz sub-band is limited to 1%",
  );
  assert.strictEqual(eu868.maxEirpDbm(868_100_000), 16, "and to 16 dBm");
  assert.strictEqual(
    eu868.maxPayload(5).application,
    242,
    "DR5 carries the largest EU868 application payload",
  );
  assert.strictEqual(eu868.rx1DataRate(5, 0), 5, "RX1 at offset 0 mirrors the uplink rate");
  assert.strictEqual(eu868.rx2().frequencyHz, 869_525_000, "RX2 listens on 869.525 MHz");
  assert.strictEqual(eu868.nextBackoffDataRate(0), null, "DR0 has nothing slower to fall back to");

  // EU868 defines every number it has, including the LR-FHSS rates.
  const fhss = eu868.dataRate(lora.LoraDirection.Uplink, 9);
  assert.strictEqual(fhss.kind, lora.LoraModulation.LrFhss, "EU868 DR9 is LR-FHSS");
  assert.strictEqual(fhss.codingRateNumerator, 2, "at coding rate 2/3");
  assert.strictEqual(fhss.codingRateDenominator, 3);
  assert.strictEqual(
    eu868.dataRate(lora.LoraDirection.Uplink, 200),
    null,
    "a number past the end of the table is absent",
  );

  // A number the region reserves is reported as reserved, which is different
  // from one it never defines: US915 numbers its downlink rates from DR8.
  assert.strictEqual(
    us915Downlink.kind,
    lora.LoraModulation.Reserved,
    "US915 reserves downlink DR2",
  );
  assert.strictEqual(
    us915.dataRate(lora.LoraDirection.Downlink, 8).kind,
    lora.LoraModulation.Lora,
    "and starts its downlink rates at DR8",
  );

  // The regions differ in exactly the way that makes this worth having.
  assert.strictEqual(
    us915.dutyCyclePermille(903_000_000),
    null,
    "the FCC caps dwell time rather than duty cycle, so US915 describes no sub-band",
  );
  assert.notStrictEqual(
    us915.info().downlinkDataRateCount,
    us915.info().uplinkDataRateCount,
    "US915 numbers its downlink data rates differently from its uplink ones",
  );
  assert.strictEqual(
    lora.planFor(lora.LoraRegion.Au915).info().hasDwellTimeLimit,
    true,
    "AU915 does limit dwell time",
  );

  // The budget question a deployment actually asks.
  assert.ok(
    lora.messagesPerHourAt(eu868, 5, 20, 868_100_000) > 0,
    "a fast EU868 rate leaves room for many messages an hour",
  );

  // A private plan on licensed spectrum answers everything a published one does.
  const priv = new lora.LoraPlanBuilder("private-915");
  priv.dataRate(lora.LoraDirection.Uplink, {
    kind: lora.LoraModulation.Lora,
    bitrateBps: 250,
    bandwidthHz: 125_000,
    spreadingFactor: 12,
  });
  priv.dataRate(lora.LoraDirection.Uplink, {
    kind: lora.LoraModulation.Lora,
    bitrateBps: 5_470,
    bandwidthHz: 125_000,
    spreadingFactor: 7,
  });
  priv.channelBlock(lora.LoraChannelSet.Default, {
    startHz: 915_000_000,
    stepHz: 500_000,
    count: 4,
    minDataRate: 0,
    maxDataRate: 1,
  });
  priv.subBand({
    startHz: 915_000_000,
    endHz: 917_000_000,
    dutyCyclePermille: 1000,
    maxEirpDbm: 30,
  });
  priv.rx(915_000_000, 0, 0);
  priv.rx1Row([0]);
  priv.rx1Row([1]);
  const licensed = priv.build();
  assert.strictEqual(licensed.name, "private-915");
  assert.strictEqual(licensed.channelFrequencyHz(3), 916_500_000, "four channels, 500 kHz apart");
  assert.strictEqual(
    licensed.dutyCyclePermille(915_500_000),
    1000,
    "licensed spectrum is reported as unrestricted, not refused",
  );
  assert.strictEqual(
    licensed.maxEirpDbm(915_500_000),
    30,
    "and carries the power its license allows",
  );
  assert.strictEqual(
    licensed.nextBackoffDataRate(1),
    0,
    "an unset back-off chain steps down one rate at a time",
  );

  // A plan that would answer a question wrongly is refused where it is built.
  const broken = new lora.LoraPlanBuilder("too-narrow");
  broken.dataRate(lora.LoraDirection.Uplink, {
    kind: lora.LoraModulation.Lora,
    bitrateBps: 250,
    bandwidthHz: 125_000,
    spreadingFactor: 12,
  });
  broken.rx(915_000_000, 0, 5);
  broken.rx1Row([0]);
  assert.throws(
    () => broken.build(),
    /RX1 row/,
    "offsets up to 5 need six entries in every row",
  );

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
  assert.strictEqual(router.forward(0x09).nextHop, 0x07, "a cheaper neighbor wins");
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

  const { sx126x } = radios;
  const whip = lora.linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 });
  const power = sx126x.txPowerUnderCeiling(sx126x.Amplifier.HighPower, whip, 16);
  assert.strictEqual(power.settingDbm, 14, "the amplifier drives as hard as the EIRP ceiling allows");
  assert.deepStrictEqual(
    [...sx126x.setRfFrequency(868_100_000)],
    [0x86, 0x36, 0x41, 0x99, 0x9a],
    "the frequency word goes out most significant byte first",
  );
  assert.throws(
    () => sx126x.setLoraModulationParams(lora.link(7, 203_125)),
    /bandwidth/,
    "a bandwidth the chip lacks is refused",
  );
  assert.strictEqual(sx126x.status(0x2c).chipMode, "StandbyRc", "the status byte decodes");
  const guard = new radios.DutyCycle(10);
  const airtime = guard.transmitted(0, lora.link(12, 125_000), 10);
  assert.strictEqual(guard.waitUs(0), airtime * 100, "a 1% duty cycle holds the radio silent");

  assert.ok(sx126x.llcc68Supports(lora.link(9, 125_000)), "an LLCC68 has SF9 at 125 kHz");
  assert.ok(!sx126x.llcc68Supports(lora.link(10, 125_000)), "but not SF10");

  // A radio is reached over spidev and the GPIO character device, so opening one says
  // either that this platform has neither or which device it could not open.
  assert.throws(
    () => radios.LoraRadio.openSx127x(
      { spi: "/dev/spidev-pamoja-absent", gpioChip: "/dev/gpiochip0", resetLine: 25 },
      { output: "PaBoost" },
    ),
    /spidev-pamoja-absent|only Linux/,
    "a radio that is not there is reported with the device or the platform",
  );
  assert.throws(
    () => radios.LoraRadio.openSx126x(
      { spi: "/dev/spidev0.0", gpioChip: "/dev/gpiochip0", resetLine: 25, busyLine: 24 },
      { amplifier: sx126x.Amplifier.HighPower, tcxoVolts: 1.9 },
    ),
    /1\.9 V/,
    "and a TCXO voltage DIO3 cannot supply is refused before any device is opened",
  );
  // A gateway datagram round trips, and one that is not this protocol is refused.
  const pull = gateway.encode({
    kind: gateway.PacketKind.PullData,
    token: 0x0102,
    gateway: "b827ebfffe010203",
  });
  assert.strictEqual(pull.length, 12, "a PULL_DATA is twelve bytes");
  assert.strictEqual(
    gateway.parse(pull).gateway,
    "b827ebfffe010203",
    "and carries the gateway's identifier",
  );
  assert.throws(
    () => gateway.parse(Buffer.from([1, 0, 1, 0])),
    /version/,
    "protocol version 1 is not this protocol",
  );

  // The network side of a site admits a device, answers its join, and reads what it sends.
  const site = new gateway.Network(lora.planFor(lora.LoraRegion.Eu868), 0x00002a, null, 0x26010001);
  const devEui = Buffer.alloc(8, 0x11);
  const appEui = Buffer.alloc(8, 0x22);
  const appKey = Buffer.alloc(16, 0x33);
  site.register(devEui, appEui, appKey);

  const joiner = lorawan.device(devEui, appEui, appKey);
  const joined = site.uplink({
    frequencyHz: 868_100_000,
    payload: joiner.joinRequest(0x0102),
    link: lora.link(7, 125_000),
    timestampUs: 1_000_000,
  });
  assert.strictEqual(joined.outcome, "Joined", "a join request is admitted");
  assert.strictEqual(joined.devAddr, 0x26010001, "and granted the first address");
  assert.strictEqual(
    joined.accept.timestampUs,
    6_000_000,
    "whose accept goes out five seconds later",
  );
  assert.ok(joined.accept.invertPolarity, "with the polarity a device listens for");

  const granted = joiner.acceptJoin(joined.accept.payload, 0x0102);
  const carried = site.uplink({
    frequencyHz: 868_100_000,
    payload: granted.session().encodeUplink(0, 2, Buffer.from("21.5")),
    link: lora.link(7, 125_000),
    timestampUs: 9_000_000,
  });
  assert.strictEqual(carried.outcome, "Data", "a session frame is read");
  assert.strictEqual(carried.payload.toString(), "21.5", "and decrypted");
  assert.strictEqual(carried.slot.timestampUs, 10_000_000, "one second after the uplink");

  const answered = site.answer(carried.devAddr, carried.slot, 2, Buffer.from("ok"));
  assert.ok(answered.invertPolarity, "the answer is inverted too");

  const stranger = site.uplink({
    frequencyHz: 868_100_000,
    payload: lorawan
      .session(0x12345678, Buffer.alloc(16, 0x09), Buffer.alloc(16, 0x08))
      .encodeUplink(0, 1, Buffer.from("hello")),
    link: lora.link(7, 125_000),
    timestampUs: 11_000_000,
  });
  assert.strictEqual(
    stranger.outcome,
    "Foreign",
    "and a frame for another network is reported, not refused",
  );

  const { sx127x } = radios;
  assert.strictEqual(sx127x.frequencyWord(868_100_000), 0xd90666, "the SX1276 carrier word");
  assert.strictEqual(sx127x.loraOpMode(sx127x.Mode.Tx), 0x8b, "TX mode on the LoRa page");
  assert.strictEqual(
    sx127x.txPower(sx127x.PaOutput.PaBoost, 20).paDac,
    sx127x.PA_DAC_HIGH_POWER,
    "+20 dBm on PA_BOOST needs the high power setting",
  );
  assert.throws(
    () => sx127x.modem(lora.link(5, 125_000), 868_100_000),
    /SF5/,
    "the SX1276 has no SF5",
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


// A device out of a gateway's reach, reaching it through the relay next door.
function relayedReach() {
  const plan = lora.planFor(lora.LoraRegion.Eu868);
  const site = new gateway.Network(plan, 0x00002a, null, 0x26010001);
  const settings = { minOutputDbm: 2, maxOutputDbm: 14, lowestHz: 863_000_000, highestHz: 870_000_000 };

  const relayEui = Buffer.alloc(8, 0x41);
  const sensorEui = Buffer.alloc(8, 0x42);
  const joinEui = Buffer.alloc(8, 0x22);
  const appKey = Buffer.alloc(16, 0x33);
  site.register(relayEui, joinEui, appKey);
  site.register(sensorEui, joinEui, appKey);

  // Both join the network the ordinary way, the relay first.
  const node = lorawan.Relay.overTheAir(plan, relayEui, joinEui, appKey, settings);
  const relayJoin = node.join(0x0101, 1_000_000);
  const relayAccept = site.uplink({
    frequencyHz: relayJoin.frequencyHz,
    payload: relayJoin.frame,
    link: relayJoin.link,
    timestampUs: 1_000_000,
  });
  assert.strictEqual(relayAccept.outcome, "Joined", "the relay joins like any device");
  node.heardIn(lorawan.ReceiveWindow.Rx1, relayAccept.accept.payload, 5);
  assert.ok(node.joined, "and holds the session it was granted");

  const sensor = lorawan.EndDevice.overTheAir(plan, sensorEui, joinEui, appKey, settings);
  const sensorJoin = sensor.join(0x0102, 20_000_000);
  const sensorAccept = site.uplink({
    frequencyHz: sensorJoin.frequencyHz,
    payload: sensorJoin.frame,
    link: sensorJoin.link,
    timestampUs: 20_000_000,
  });
  sensor.heard(sensorAccept.accept.payload, 5, lorawan.ReceiveWindow.Rx1);
  const sensorAddr = sensor.devAddr;

  // The network hands the relay the key that lets it verify the sensor's wake-up frames.
  const trust = site.trustCommand(sensorAddr, 0, 63, 0);
  assert.strictEqual(trust.kind, "updateUplinkListReq", "which travels as a relay command");
  const relayUplink = node.sendEmpty(40_000_000);
  const heardRelay = site.uplink({
    frequencyHz: relayUplink.frequencyHz,
    payload: relayUplink.frame,
    link: relayUplink.link,
    timestampUs: 40_000_000,
  });
  const configure = site.command(node.devAddr, heardRelay.slot, [trust]);
  assert.strictEqual(
    node.heardIn(lorawan.ReceiveWindow.Rx1, configure.payload, 5).kind,
    "Device",
    "the relay reads its own configuration",
  );

  node.start(lorawan.CadPeriodicity.Ms1000, 0);
  assert.ok(node.running, "and starts listening for the devices around it");
  const scan = node.nextScan(60_000_000);
  assert.strictEqual(scan.channel, lorawan.WorChannel.Default, "on its default channel");

  // The sensor sends through the relay: a wake-up frame first, then the uplink itself.
  assert.ok(sensor.useRelay(true), "the sensor decides to use a relay");
  assert.strictEqual(sensor.relaySync, lorawan.RelaySync.Initialized, "knowing nothing of it yet");
  const reading = sensor.send(2, "21.5", 61_000_000);
  assert.ok(reading.relay, "so the uplink goes out behind a wake-up frame");
  assert.strictEqual(reading.relay.wakeUp.frame.length, 15, "fifteen bytes ahead of an uplink");

  const woke = node.heardWor(scan, reading.relay.wakeUp.frame, -90, 4, scan.startUs + 500_000);
  assert.strictEqual(woke.kind, "Uplink", "the relay knows the device");
  assert.strictEqual(woke.devAddr, sensorAddr, "and which one it is");
  assert.strictEqual(woke.forward, lorawan.RelayForward.Available, "and has room to forward");

  const status = sensor.heardWorAck(woke.acknowledgment.frame);
  assert.strictEqual(status.cadPeriodicity, lorawan.CadPeriodicity.Ms1000, "the relay says how often it scans");
  assert.strictEqual(
    sensor.relaySync,
    lorawan.RelaySync.Synchronized,
    "so the next wake-up frame needs only a short preamble",
  );

  const dueUs = node.heardUplink(reading.frame, -88, 6, woke.listen.startUs + 100_000);
  assert.strictEqual(node.forwardDue, dueUs, "the uplink waits fifty milliseconds to be forwarded");
  const forwarded = node.forward(dueUs);
  const carried = site.uplink({
    frequencyHz: forwarded.frequencyHz,
    payload: forwarded.frame,
    link: forwarded.link,
    timestampUs: dueUs,
  });
  assert.strictEqual(carried.outcome, "Data", "the network reads the sensor's frame");
  assert.strictEqual(carried.devAddr, sensorAddr, "as the sensor's own");
  assert.strictEqual(carried.payload.toString(), "21.5", "with the reading it sent");
  assert.strictEqual(carried.relay.relay, node.devAddr, "and says which relay carried it");
  assert.strictEqual(carried.relay.worChannel, lorawan.WorChannel.Default, "on which channel");

  // The answer goes back the same way, into the window the sensor keeps for a relay.
  const answer = site.answer(sensorAddr, carried.slot, 2, Buffer.from("ok"));
  const passed = node.heardIn(lorawan.ReceiveWindow.Rx1, answer.payload, 5);
  assert.strictEqual(passed.kind, "Downlink", "the relay passes it on rather than reading it");
  const delivered = sensor.heard(passed.downlink.frame, 5, lorawan.ReceiveWindow.Rxr);
  assert.strictEqual(delivered.kind, "Data", "and the sensor hears it");
  assert.strictEqual(delivered.delivery.payload.toString(), "ok", "with what the network sent");

  // A device the relay was never told about is reported to the network instead.
  const stranger = lorawan.relay.worUplink(
    lorawan.relay.worKeys(Buffer.alloc(16, 0x77), 0x26010009),
    0x26010009,
    0,
    scan.carrier,
    scan.carrier,
  );
  const later = node.nextScan(dueUs + 60_000_000);
  assert.strictEqual(
    node.heardWor(later, stranger, -95, 2, later.startUs + 500_000).kind,
    "Notified",
    "a relay tells its network about a device it cannot verify",
  );
}


// Getting a firmware image to a whole field of devices: one clock, one group, one
// broadcast cut into pieces, and a device that says what it made of it.
function broadcastUpdates() {
  // A device with no clock of its own asks for the time. The server answers with the
  // difference, and the token keeps a late answer from pulling the clock back.
  const sync = new lorawan.clock.ClockSync();
  const asking = lorawan.packageParse(lorawan.clock.PORT, true, sync.request(1_000_000));
  assert.strictEqual(asking.kind, "appTimeReq", "a device asks for a correction");
  assert.strictEqual(asking.deviceTime, 1_000_000, "saying what it believes the time is");

  const answer = lorawan.packageEncode({
    port: lorawan.clock.PORT,
    kind: "appTimeAns",
    uplink: false,
    timeCorrection: 12,
    token: asking.token,
  });
  assert.strictEqual(sync.heard(answer).correction, 12, "and applies what comes back");
  assert.strictEqual(
    sync.heard(answer).correction,
    undefined,
    "a repeat of the same answer is ignored",
  );

  // A group every device in the field belongs to. Its key travels wrapped under a key
  // each device derives from its own root key and never transmits.
  const rootKey = Buffer.alloc(16, 0x2b);
  const groupKey = Buffer.alloc(16, 0x77);
  const groupAddr = 0x2601_0042;
  const keKey = lorawan.multicast.keKey(lorawan.multicast.rootKey(rootKey));
  const setup = lorawan.packageEncode({
    port: lorawan.multicast.PORT,
    kind: "mcGroupSetupReq",
    uplink: false,
    mcGroupId: 1,
    mcAddr: groupAddr,
    mcKeyEncrypted: lorawan.multicast.wrapKey(keKey, groupKey),
    minMcFcnt: 0,
    maxMcFcnt: 0xffff,
  });
  const read = lorawan.packageParse(lorawan.multicast.PORT, false, setup);
  assert.strictEqual(read.mcAddr, groupAddr, "the group keeps its address across the wire");
  assert.deepStrictEqual(
    Buffer.from(lorawan.multicast.key(keKey, read.mcKeyEncrypted)),
    groupKey,
    "and the device unwraps the key the server wrapped",
  );

  // The image goes out as a block, cut into fragments with more sent than there are.
  const image = Buffer.from("firmware for a field of flow meters, long enough to be cut up");
  const fragSize = 16;
  const { nbFrag, padding } = lorawan.fragment.session(image.length, fragSize);
  assert.strictEqual(nbFrag * fragSize - padding, image.length, "the padding covers the tail");

  const opened = lorawan.packageEncode({
    port: lorawan.fragment.PORT,
    kind: "fragSessionSetupReq",
    uplink: false,
    fragIndex: 0,
    mcGroupBitMask: 0b0010,
    nbFrag,
    fragSize,
    ackReception: false,
    fragAlgo: 0,
    blockAckDelay: 0,
    padding,
    descriptor: Buffer.from("PJU1"),
    sessionCnt: 1,
    mic: Buffer.alloc(4, 0),
  });
  const session = lorawan.packageParse(lorawan.fragment.PORT, false, opened);
  assert.strictEqual(session.nbFrag, nbFrag, "the session says how many fragments there are");
  assert.strictEqual(session.mcGroupBitMask, 0b0010, "and which group feeds it");

  // The link drops every fourth one. The device solves for what it missed.
  const receiver = new lorawan.fragment.Defragmenter(nbFrag, fragSize, 4);
  let done = false;
  for (let n = 1; n <= nbFrag * 2 && !done; n++) {
    if (n % 4 === 0) {
      continue;
    }
    const carried = lorawan.packageEncode({
      port: lorawan.fragment.PORT,
      kind: "dataFragment",
      uplink: false,
      fragIndex: 0,
      fragmentN: n,
      data: lorawan.fragment.fragment(image, fragSize, n),
    });
    const piece = lorawan.packageParse(lorawan.fragment.PORT, false, carried);
    done = receiver.fragment(piece.fragmentN, piece.data);
  }
  assert.ok(done, "the block comes together without every fragment arriving");
  assert.deepStrictEqual(
    Buffer.from(receiver.block.subarray(0, image.length)),
    image,
    "and it is the image the server sent",
  );

  // The code over the block, taken a piece at a time so the image is never held twice.
  const blockKey = lorawan.fragment.dataBlockIntKey(rootKey);
  const whole = new lorawan.fragment.BlockMic(blockKey, 1, 0, Buffer.from("PJU1"), image.length);
  whole.update(image);
  const inPieces = new lorawan.fragment.BlockMic(
    blockKey,
    1,
    0,
    Buffer.from("PJU1"),
    image.length,
  );
  inPieces.update(image.subarray(0, 7));
  inPieces.update(image.subarray(7));
  assert.deepStrictEqual(
    Buffer.from(inPieces.finish()),
    Buffer.from(whole.finish()),
    "a streamed code is the code over the whole block",
  );

  // What the device runs, what it holds, and the one reboot it keeps.
  const manager = new lorawan.firmware.Firmware(2, 7);
  manager.setImage("Valid", 3);
  const held = lorawan.packageParse(
    lorawan.firmware.PORT,
    true,
    manager.heard(
      lorawan.packageEncode({
        port: lorawan.firmware.PORT,
        kind: "devUpgradeImageReq",
        uplink: false,
      }),
      1_000_000,
    ),
  );
  assert.strictEqual(held.imageStatus, "Valid", "the device holds an image it can install");
  assert.strictEqual(held.nextVersion, 3, "and says what it would run");

  manager.heard(
    lorawan.packageEncode({
      port: lorawan.firmware.PORT,
      kind: "devRebootCountdownReq",
      uplink: false,
      reboot: 60,
    }),
    1_000_000,
  );
  assert.strictEqual(manager.rebootInS, 60, "a countdown is kept as a countdown");
  manager.heard(
    lorawan.packageEncode({
      port: lorawan.firmware.PORT,
      kind: "devRebootTimeReq",
      uplink: false,
      reboot: 1_000_060,
    }),
    1_000_000,
  );
  assert.strictEqual(manager.rebootAtS, 1_000_060, "a moment replaces it");
  assert.strictEqual(manager.rebootInS, null, "a device keeps one reboot, not two");
  manager.rebooted();
  assert.strictEqual(manager.rebootAtS, null, "carrying it out clears it");

  // A status answer lists its groups after the count, each five bytes with no
  // identifier of its own.
  const listed = lorawan.packageEncode({
    port: lorawan.multicast.PORT,
    kind: "mcGroupStatusItem",
    uplink: true,
    mcGroupId: 1,
    mcAddr: groupAddr,
  });
  assert.strictEqual(
    lorawan.packageStatusItem(listed).mcAddr,
    groupAddr,
    "a group record reads back on its own",
  );

  assert.throws(
    () => lorawan.packageParse(1, false, Buffer.from([0x00])),
    "a port that names no package is refused",
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
  audit.verifyChain(keeper.publicKey(), [opened, shut]);

  const edited = Buffer.from(shut.toBytes());
  edited[edited.length - 1] ^= 0xff;
  assert.throws(
    () =>
      audit.verifyChain(keeper.publicKey(), [
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

  const upgrade = Buffer.alloc(300, 0x5a);
  const third = update.signManifest(
    { ...manifest, sequence: 3, storage: 0, digest: update.imageDigest(upgrade), size: upgrade.length },
    publisher,
  );
  assert.strictEqual(fleet.begin(third), 0, "the next release names the slot not running");
  fleet.write(upgrade.subarray(0, 200));
  assert.throws(
    () => fleet.write(upgrade),
    /the image is not the size the manifest declares/,
    "a piece that runs past the declared size is refused",
  );
  fleet.write(upgrade.subarray(200));
  assert.strictEqual(
    fleet.progress().written,
    upgrade.length,
    "and the transfer goes on from what the slot holds",
  );
  assert.strictEqual(fleet.finish(), 0, "to an image that matches its manifest");
  for (const now of [Number.NaN, -1, 1.5]) {
    assert.throws(
      () => fleet.stage(third, upgrade, now),
      { message: `now must be a whole number of seconds, not ${now}` },
      `a time of ${now} is refused rather than read as some other time`,
    );
  }

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

  // A file store is bounded too, survives reopening, and drains onto a transport,
  // keeping in order whatever the transport did not take.
  const folder = require("node:fs").mkdtempSync(
    require("node:path").join(require("node:os").tmpdir(), "pamoja-store-"),
  );
  const onDisk = sync.Store.file(folder, 2);
  await onDisk.append("a");
  await onDisk.append("b");
  await assert.rejects(() => onDisk.append("c"), /store is at capacity/);
  const reopened = sync.Store.file(folder, 2);
  assert.strictEqual(await reopened.len(), 2, "the records survive reopening");
  const dropping = transport.Transport.degraded(broker.rung(), { up: 1, down: 5 });
  await dropping.connect();
  await assert.rejects(() => reopened.drainTo(dropping, "outbox"), /link unreachable/);
  assert.strictEqual(await reopened.peekText(), "b", "the record the link refused stays");
  const steady = broker.rung();
  await steady.connect();
  assert.strictEqual(await reopened.drainTo(steady, "outbox"), 1);
  assert.strictEqual(await reopened.len(), 0);
  require("node:fs").rmSync(folder, { recursive: true });

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

  // A subscription placed on the ladder reaches its rungs, and a command published
  // upstream comes back through the ladder.
  await rungs.subscribe("commands/1");
  const upstream = broker.link();
  await upstream.connect();
  await upstream.send("commands/1", Buffer.from("open"));
  const inbound = await rungs.recv();
  assert.strictEqual(
    inbound.payload.toString(),
    "open",
    "the command came back through the ladder",
  );

  // A link written in JavaScript is a rung like any other: what the ladder sends
  // reaches it, and what it delivers comes back through the ladder.
  class QueueLink {
    constructor() {
      this.sent = [];
      this.filters = [];
      this.inbox = [];
      this.waiting = [];
    }
    async connect() {}
    async send(topic, payload) {
      this.sent.push({ topic, payload });
    }
    subscribe(topic) {
      this.filters.push(topic);
    }
    recv() {
      const next = this.inbox.shift();
      return next !== undefined
        ? Promise.resolve(next)
        : new Promise((resolve) => this.waiting.push(resolve));
    }
    deliver(message) {
      const waiter = this.waiting.shift();
      if (waiter) waiter(message);
      else this.inbox.push(message);
    }
  }
  const hostLink = new QueueLink();
  const hosted = new ladder.Ladder(sync.Store.memory());
  await hosted.rung(transport.Transport.fromHandlers(hostLink));
  await hosted.connect();
  await hosted.subscribe("commands/1");
  assert.strictEqual(
    await hosted.send("sensors/1", Buffer.from("21.5")),
    ladder.Delivery.Sent,
    "the host link carried the reading",
  );
  assert.strictEqual(hostLink.sent[0].topic, "sensors/1", "and saw its topic");
  assert.strictEqual(hostLink.sent[0].payload.toString(), "21.5", "and its bytes");
  assert.deepStrictEqual(hostLink.filters, ["commands/1"], "the subscription reached it");
  hostLink.deliver({ topic: "commands/1", payload: Buffer.from("open") });
  const fromHost = await hosted.recv();
  assert.strictEqual(
    fromHost.payload.toString(),
    "open",
    "what the host link delivered came back through the ladder",
  );
  hostLink.deliver(null);
  await assert.rejects(
    () => hosted.recv(),
    /closed/,
    "once the link ends the ladder has nothing to receive from",
  );

  // A send-only host link is an uplink: sends go out, nothing is listened on.
  const carried = [];
  const oneWay = new ladder.Ladder(sync.Store.memory());
  await oneWay.rung(
    transport.Transport.fromHandlers({
      connect() {},
      send(topic, payload) {
        carried.push(payload.toString());
      },
      subscribe() {
        throw new Error("an uplink is never subscribed");
      },
    }),
  );
  await oneWay.connect();
  await oneWay.subscribe("commands/1");
  assert.strictEqual(await oneWay.send("sensors/1", Buffer.from("21.6")), ladder.Delivery.Sent);
  assert.deepStrictEqual(carried, ["21.6"], "an uplink carries sends");
  await assert.rejects(() => oneWay.recv(), /closed/, "and is never listened on");

  // A shipped link goes on as an uplink too, and a broker out of reach refuses its
  // links, a ladder's among them, until it is back.
  const near = new loopback.LoopbackBroker();
  const far = new loopback.LoopbackBroker();
  const ashore = far.link();
  await ashore.connect();
  await ashore.subscribe("reports");
  const reach = new ladder.Ladder(sync.Store.memory());
  await reach.rung(near.rung());
  await reach.uplink(far.rung());
  await reach.connect();
  await reach.subscribe("orders");
  near.reachable = false;
  assert.strictEqual(near.reachable, false);
  assert.strictEqual(await reach.send("reports", "1"), ladder.Delivery.Sent);
  assert.strictEqual((await ashore.recv(5000)).text, "1", "the uplink carried it");
  await ashore.send("orders", "stop");
  await assert.rejects(() => reach.recv(20), /no message arrived/, "an uplink is never listened on");
  far.reachable = false;
  assert.strictEqual(await reach.send("reports", "2"), ladder.Delivery.Buffered);
  near.reachable = true;
  assert.strictEqual(await reach.flush(), 1, "the backlog went out once a link was back");

  // A handler that throws reports its reason, and a handler set missing a method
  // is refused up front.
  const refusing = new ladder.Ladder(sync.Store.memory());
  await refusing.rung(
    transport.Transport.fromHandlers({
      async connect() {},
      async send() {
        throw new Error("the radio is out of range");
      },
      async subscribe() {},
    }),
  );
  await refusing.connect();
  assert.strictEqual(
    await refusing.send("sensors/1", Buffer.from("x")),
    ladder.Delivery.Buffered,
    "a refused send is buffered by the ladder",
  );
  assert.throws(
    () => transport.Transport.fromHandlers({ connect() {}, subscribe() {} }),
    /needs a send method/,
    "a handler set without send is refused",
  );

  // A handler that throws before returning fails the call rather than the process,
  // and a recv that throws is reported once, after which the link has ended until
  // it connects again. A delivered payload may be text.
  let lostSession = false;
  const flaky = transport.Transport.fromHandlers({
    connect() {},
    send() {
      throw new Error("no signal");
    },
    subscribe() {},
    recv() {
      if (lostSession) throw new Error("the modem lost its session");
      return new Promise((resolve) => setTimeout(() => resolve({ topic: "commands/1", payload: "open" }), 20));
    },
  });
  await flaky.connect();
  await assert.rejects(() => flaky.send("sensors/1", "21.5"), /transport error: no signal/);
  assert.strictEqual((await flaky.recv(5000)).text, "open", "a text payload is delivered");
  lostSession = true;
  await assert.rejects(
    async () => {
      for (;;) await flaky.recv(5000);
    },
    /transport error: the modem lost its session/,
    "a recv that throws is reported",
  );
  assert.strictEqual(await flaky.recv(5000), null, "and the link has ended after it");

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

  // A transport is driven directly with the calls every link keeps, one call at a
  // time, and one with a call running is not handed on.
  const listening = broker.rung();
  await listening.connect();
  await listening.subscribe("alarms/1");
  const waiting = listening.recv();
  await new Promise((resolve) => setTimeout(resolve, 50));
  await assert.rejects(
    () => rungs.rung(listening),
    /busy with a call/,
    "a transport with a call running is not handed on",
  );
  assert.ok(listening.isAvailable, "and it is still the caller's");
  await upstream.send("alarms/1", "smoke");
  assert.strictEqual((await waiting).text, "smoke", "the receive took the alarm");

  // A receive with a limit gives up without taking the next message.
  await assert.rejects(
    () => listening.recv(20),
    /no message arrived within 20 ms/,
    "a quiet transport runs out of time",
  );
  await upstream.send("alarms/1", "heat");
  assert.strictEqual(
    (await listening.recv(5000)).text,
    "heat",
    "the next message waited for the next receive",
  );
  const quiet = broker.link();
  await quiet.connect();
  await quiet.subscribe("quiet/1");
  await assert.rejects(() => quiet.recv(20), /no message arrived/, "so does a quiet link");
  await assert.rejects(() => offline.recv(20), /no message arrived/, "and a quiet ladder");

  // A CoAP server takes readings on its filters and refuses other paths, and sends a
  // command while another call waits for a reading.
  const coapGateway = new coap.CoapServer("127.0.0.1:0");
  await coapGateway.connect();
  await coapGateway.subscribe("sensors/#");
  await coapGateway.send("commands/valve", "closed");
  const coapNode = new coap.CoapClient({
    host: "127.0.0.1",
    port: coapGateway.localPort,
    ackTimeoutMs: 200,
  });
  await coapNode.connect();
  await coapNode.subscribe("commands/valve");
  assert.strictEqual((await coapNode.recv(2000)).text, "closed", "the state on registering");
  assert.strictEqual(coapGateway.observers("commands/valve"), 1);
  const awaitedReading = coapGateway.recv(5000);
  await new Promise((resolve) => setTimeout(resolve, 50));
  await coapGateway.send("commands/valve", "open");
  assert.strictEqual((await coapNode.recv(2000)).text, "open", "a command beside a waiting receive");
  await coapNode.send("sensors/1/temperature", "21.5");
  assert.strictEqual((await awaitedReading).topic, "sensors/1/temperature");
  await assert.rejects(() => coapNode.send("pumps/1", "on"), /4\.04 Not Found/);
  await assert.rejects(() => coapGateway.recv(20), /no message arrived/);
  await coapNode.disconnect();
  await coapGateway.disconnect();
  assert.ok(!coapGateway.isConnected);

  // One publisher, many subscribers, in one process.
  const hub = new bus.EventBus(8);
  const first = hub.subscribe();
  const second = hub.subscribe();
  hub.publish(Buffer.from("battery.low"));
  assert.strictEqual((await first.next()).toString(), "battery.low");
  assert.strictEqual((await second.next()).toString(), "battery.low");

  // An endpoint publishes while its own wait is open, and hears itself.
  const ownWait = first.nextText();
  first.publish("heater.off");
  assert.strictEqual(await ownWait, "heater.off");

  // A publisher counts who it reached; a reader that falls behind counts what it lost.
  const power = new bus.EventPublisher(2);
  assert.strictEqual(power.publish("0"), 0);
  const lagging = power.subscribe();
  const sampler = power.publisher();
  for (let sample = 1; sample <= 5; sample += 1) {
    assert.strictEqual(sampler.publish(String(sample)), 1);
  }
  assert.strictEqual(await lagging.nextText(), "4");
  assert.strictEqual(lagging.missed, 3);
  assert.strictEqual(await lagging.nextText(), "5");
  await assert.rejects(lagging.nextText(30), /no event arrived within 30 ms/);
  power.publish("6");
  assert.strictEqual(await lagging.nextText(), "6");

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
    "one second at one meter a second puts it a meter ahead",
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
  assert.ok(fridge.description.includes("safe range"), "a preset says what it is for");

  // A profile of the program's own, built from its parts.
  const band = { kind: profile.ControlKind.Setpoint, setpoint: 37.5, hysteresis: 7.5, safeBand: 15 };
  const hourly = { activeSecs: 300, saverSecs: 1800, criticalSecs: 3600 };
  const drip = new profile.Profile("raised-bed-drip", "garden/bed-1/moisture", band, hourly);
  assert.strictEqual(drip.control.cooling, false, "cooling is false unless given");
  assert.strictEqual(drip.power.criticalBelow.toFixed(1), "0.2", "the thresholds default");
  assert.strictEqual(drip.controller().evaluate(16.7).actuator, true, "a dry bed opens the valve");
  assert.throws(
    () => new profile.Profile("x", "t", { kind: profile.ControlKind.Setpoint, setpoint: 1 }, hourly),
    /a Setpoint control needs hysteresis/,
  );
  assert.throws(
    () => new profile.Profile("x", "t", { kind: profile.ControlKind.Custom, customKind: "level" }, hourly),
    /level is a built-in control kind/,
  );
  assert.throws(
    () => new profile.Profile("x", "t", band, { ...hourly, activeSecs: 2.5 }),
    /activeSecs must be a whole number of seconds, not 2.5/,
  );
  assert.ok(
    fridge.presentation.elements.some((element) => element.key === "fridge_temp"),
    "and says how it should be drawn"
  );

  // A presentation is typed on the way in and on the way out, and travels in the manifest.
  const drawn = fridge
    .withDescription("Holds the clinic fridge at 5 C.")
    .withPresentation({
      elements: [
        {
          key: "door_open",
          unit: "state",
          label: "Door",
          labels: { sw: "Mlango" },
          viz: profile.Viz.Switch,
          state: "state.closed",
        },
        {
          key: "compressor_amps",
          unit: "amps",
          label: "Compressor current",
          viz: profile.Viz.Dial,
          band: [0.5, 3.0],
          scope: ["mesh"],
        },
      ],
      theme: { accent: "#3fb1c8" },
      messages: { "event.door_ajar": { en: "Door left open", sw: "Mlango umeachwa wazi" } },
    });
  assert.strictEqual(drawn.description, "Holds the clinic fridge at 5 C.");
  const shown = profile.Profile.fromJson(drawn.toJson()).presentation;
  assert.strictEqual(shown.elements.length, 2, "both elements survive the manifest");
  assert.strictEqual(shown.elements[0].viz, "switch", "the graphic is its manifest name");
  assert.strictEqual(shown.elements[0].labels.sw, "Mlango");
  assert.deepStrictEqual(shown.elements[1].band, [0.5, 3.0]);
  assert.deepStrictEqual(shown.elements[1].scope, ["mesh"]);
  assert.ok(shown.elements[0].scope == null, "an unscoped element is offered everywhere");
  assert.strictEqual(shown.theme.accent, "#3fb1c8");
  assert.strictEqual(shown.messages["event.door_ajar"].sw, "Mlango umeachwa wazi");
  assert.throws(
    () => fridge.withPresentation({ elements: [{ key: "x", unit: "u", label: "x", viz: "bar", band: [1] }] }),
    /must be \[low, high\]/,
    "a band is two numbers",
  );

  // A kind the library never shipped loads with its parameters beside it.
  const orchard = profile.Profile.fromJson(JSON.stringify({
    name: "orchard-frost",
    topic: "orchard/air/temperature",
    control: { kind: "frost_guard", warn_below: 2, latching: true, zone: "north" },
    power: { active_secs: 60, saver_secs: 300, critical_secs: 900 },
  }));
  assert.strictEqual(orchard.control.kind, profile.ControlKind.Custom);
  assert.strictEqual(orchard.control.customKind, "frost_guard");
  assert.deepStrictEqual(orchard.control.params, { warn_below: 2, latching: true, zone: "north" });
  assert.ok(
    orchard.controller().evaluate(-4).actuator == null,
    "the built-in controller for a custom kind observes only",
  );
  assert.ok(orchard.toJson().includes('"kind": "frost_guard"'), "and it writes back under its own name");

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
