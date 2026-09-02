// The Node side of the cross-language conformance suite: the same vectors every
// other binding runs, so a facade that drifts here fails rather than quietly
// disagreeing with Rust, Python, and .NET.
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const {
  DeviceIdentity,
  verify,
  toCbor,
  fromCbor,
  packSamples,
  unpackSamples,
  Quantizer,
  Smoother,
  Pid,
  Thermostat,
  Depletion,
  Calibration,
  Geofence,
  deadband,
  audit,
  can,
  ladder,
  loopback,
  sim,
  sync,
  transport,
  gpio,
  lora,
  lorawan,
  mesh,
  modbus,
  power,
  routing,
  serial: serialFraming,
  session,
  telemetry,
  update,
  actuators,
  sensors,
  Window,
  Median,
  Trend,
  Anomaly,
  WINDOW_CAPACITY,
} = require("./dist/index.js");

const VECTORS = JSON.parse(
  fs.readFileSync(path.join(__dirname, "..", "..", "conformance", "vectors.json"), "utf8"),
);

// The vectors carry f32 values widened to f64, so they compare exactly; the
// tolerance covers the accumulation order of the iterative helpers.
const TOLERANCE = VECTORS.tolerance;

/** Asserts two numbers agree within the vectors' tolerance. */
function close(got, want, message, tolerance = TOLERANCE) {
  assert.ok(
    Math.abs(got - want) <= tolerance,
    `${message}: expected ${want}, got ${got}`,
  );
}

/** Decodes a lowercase hex string from the vectors. */
function unhex(text) {
  return Buffer.from(text, "hex");
}

function identity() {
  const vector = VECTORS.identity;
  const device = DeviceIdentity.fromSeed(unhex(vector.seed));

  assert.deepStrictEqual(device.publicKey(), unhex(vector.publicKey), "public key matches");
  assert.strictEqual(device.fingerprint(), vector.fingerprint, "fingerprint matches");
  assert.deepStrictEqual(
    device.sign(vector.payload),
    unhex(vector.signature),
    "the signature is deterministic for this seed and payload",
  );

  assert.ok(
    verify(unhex(vector.publicKey), vector.payload, unhex(vector.signature)),
    "the signature verifies",
  );
  assert.ok(
    !verify(unhex(vector.publicKey), vector.tamperedPayload, unhex(vector.signature)),
    "a tampered payload does not verify",
  );
}

function codec() {
  const vector = VECTORS.codec;
  const cbor = unhex(vector.cbor);

  assert.deepStrictEqual(toCbor(Buffer.from(vector.json, "utf8")), cbor, "JSON encodes to CBOR");
  assert.deepStrictEqual(fromCbor(cbor), JSON.parse(vector.json), "CBOR decodes to the document");
  assert.deepStrictEqual(
    toCbor(Buffer.from(vector.unsortedJson, "utf8")),
    cbor,
    "keys are sorted on the way through, so the encoding is canonical",
  );

  const deltas = vector.deltas;
  assert.deepStrictEqual(packSamples(deltas.samples), unhex(deltas.packed), "samples pack");
  assert.deepStrictEqual(unpackSamples(unhex(deltas.packed)), deltas.samples, "samples unpack");

  const q = vector.quantizer;
  const quantizer = new Quantizer(q.scale);
  assert.deepStrictEqual(quantizer.encode(q.readings), unhex(q.packed), "readings pack");
  quantizer.decode(unhex(q.packed)).forEach((got, index) => {
    close(got, q.readings[index], "reading decodes to precision", q.tolerance);
  });
}

function helpers() {
  const smoother = new Smoother(VECTORS.smoother.weight);
  VECTORS.smoother.samples.forEach((sample, index) => {
    close(smoother.update(sample), VECTORS.smoother.outputs[index], "smoother output");
  });

  const p = VECTORS.pid;
  const controller = new Pid(p.kp, p.ki, p.kd);
  p.measurements.forEach((measurement, index) => {
    close(controller.update(p.setpoint, measurement, p.dt), p.outputs[index], "pid output");
  });

  const t = VECTORS.thermostat;
  const thermostat = Thermostat.cooling(t.setpoint, t.hysteresis);
  t.readings.forEach((reading, index) => {
    assert.strictEqual(thermostat.update(reading), t.outputs[index], "thermostat output");
  });

  const d = VECTORS.depletion;
  const depletion = new Depletion(d.threshold);
  d.levels.forEach((level, index) => {
    assert.strictEqual(depletion.update(level), d.outputs[index], "depletion output");
  });

  const c = VECTORS.calibration;
  const calibration = Calibration.twoPoint(c.rawLow, c.valueLow, c.rawHigh, c.valueHigh);
  c.inputs.forEach((raw, index) => {
    close(calibration.apply(raw), c.outputs[index], "calibration output");
  });

  const b = VECTORS.deadband;
  b.inputs.forEach((value, index) => {
    close(deadband(value, b.center, b.width), b.outputs[index], "deadband output");
  });
}

function geofence() {
  const vector = VECTORS.geofence;
  const fence = new Geofence(vector.center, vector.radiusM);
  vector.fixes.forEach((fix, index) => {
    assert.strictEqual(fence.update(fix), vector.boundaries[index], "boundary state");
  });
}

function serial() {
  const vector = VECTORS.serial;
  const payload = unhex(vector.payload);

  assert.deepStrictEqual(serialFraming.slip.encode(payload), unhex(vector.slipFrame), "SLIP frame");
  assert.deepStrictEqual(serialFraming.slip.decode(unhex(vector.slipFrame)), payload, "SLIP payload");
  assert.deepStrictEqual(serialFraming.cobs.encode(payload), unhex(vector.cobsFrame), "COBS frame");
  assert.deepStrictEqual(serialFraming.cobs.decode(unhex(vector.cobsFrame)), payload, "COBS payload");

  assert.strictEqual(
    serialFraming.slip.maxEncodedLen(payload.length),
    vector.slipMaxEncodedLen,
    "SLIP worst case",
  );
  assert.strictEqual(
    serialFraming.cobs.maxEncodedLen(payload.length),
    vector.cobsMaxEncodedLen,
    "COBS worst case",
  );

  assert.throws(
    () => serialFraming.slip.decode(unhex(vector.corruptSlipFrame)),
    "a frame with a bad escape must be refused",
  );

  const stream = vector.slipStream;
  const decoder = new serialFraming.SlipDecoder();
  const bytes = unhex(stream.bytes);
  const frames = [];
  for (let at = 0; at < bytes.length; at += stream.chunk) {
    frames.push(...decoder.feed(bytes.subarray(at, at + stream.chunk)));
  }
  assert.deepStrictEqual(
    frames,
    stream.frames.map(unhex),
    "the good frames survive the corrupt one",
  );
  assert.strictEqual(decoder.discarded, stream.discarded, "discarded count");

  const cobsStream = VECTORS.serial.cobsStream;
  const cobsDecoder = new serialFraming.CobsDecoder();
  const cobsBytes = unhex(cobsStream.bytes);
  const cobsFrames = [];
  for (let at = 0; at < cobsBytes.length; at += cobsStream.chunk) {
    cobsFrames.push(...cobsDecoder.feed(cobsBytes.subarray(at, at + cobsStream.chunk)));
  }
  assert.deepStrictEqual(cobsFrames, cobsStream.frames.map(unhex), "COBS frames");
}

function modbusVectors() {
  const vector = VECTORS.modbus;

  const read = vector.readHoldingRegisters;
  assert.deepStrictEqual(
    modbus.readHoldingRegisters(read.address, read.start, read.count),
    unhex(read.frame),
    "read-holding-registers frame",
  );
  const coilsRequest = vector.readCoils;
  assert.deepStrictEqual(
    modbus.readCoils(coilsRequest.address, coilsRequest.start, coilsRequest.count),
    unhex(coilsRequest.frame),
    "read-coils frame",
  );
  const single = vector.writeSingleRegister;
  assert.deepStrictEqual(
    modbus.writeSingleRegister(single.address, single.register, single.value),
    unhex(single.frame),
    "write-single-register frame",
  );
  const many = vector.writeMultipleRegisters;
  assert.deepStrictEqual(
    modbus.writeMultipleRegisters(many.address, many.start, many.values),
    unhex(many.frame),
    "write-multiple-registers frame",
  );
  const bits = vector.writeMultipleCoils;
  assert.deepStrictEqual(
    modbus.writeMultipleCoils(bits.address, bits.start, bits.values),
    unhex(bits.frame),
    "write-multiple-coils frame",
  );

  assert.strictEqual(modbus.crc16(unhex(vector.crc.data)), vector.crc.value, "CRC");

  const reply = modbus.parseFrame(unhex(vector.reply.frame));
  assert.strictEqual(reply.address, vector.reply.address, "reply address");
  assert.strictEqual(reply.functionCode, vector.reply.functionCode, "reply function");
  assert.strictEqual(reply.exception, null, "a served request reports no exception");
  assert.deepStrictEqual(reply.pdu, unhex(vector.reply.pdu), "reply PDU");
  assert.deepStrictEqual(reply.registers(), vector.reply.registers, "reply registers");

  // Registers above 0x7FFF, which catch a binding that reads them as signed.
  const high = modbus.parseFrame(unhex(vector.highRegisterReply.frame));
  assert.deepStrictEqual(
    high.registers(),
    vector.highRegisterReply.registers,
    "registers above 0x7FFF read back unsigned",
  );

  const bitReply = modbus.parseFrame(unhex(vector.bitReply.frame));
  assert.deepStrictEqual(
    bitReply.coils(vector.bitReply.count),
    vector.bitReply.coils,
    "reply coils",
  );

  const refused = modbus.parseFrame(unhex(vector.exceptionReply.frame));
  assert.strictEqual(refused.exception, vector.exceptionReply.exception, "exception code");
  assert.strictEqual(
    refused.functionCode,
    vector.exceptionReply.functionCode,
    "the exception response echoes the function code with its high bit set",
  );

  assert.throws(
    () => modbus.parseFrame(unhex(vector.corruptFrame)),
    "a frame mangled on the wire must not reach the application",
  );
}

function canVectors() {
  const vector = VECTORS.can;

  const classic = can.frame(vector.classic.id, unhex(vector.classic.data), vector.classic.extended);
  assert.strictEqual(classic.dlc, vector.classic.dlc, "classic DLC");
  assert.deepStrictEqual(classic.data, unhex(vector.classic.data), "classic payload");

  const fd = can.fdFrame(vector.fd.id, unhex(vector.fd.data), vector.fd.extended);
  assert.strictEqual(fd.dlc, vector.fd.dlc, "CAN-FD DLC");
  assert.ok(fd.fd && fd.extended, "the frame keeps its flags");

  const remote = can.remoteFrame(vector.remote.id, vector.remote.requested, vector.remote.extended);
  assert.strictEqual(remote.len, vector.remote.len, "a remote frame reports the length it asks for");
  assert.strictEqual(remote.data.length, vector.remote.dataLen, "and carries no bytes");

  assert.throws(
    () => can.frame(0x100, Buffer.alloc(vector.tooLongForClassic)),
    "a classic frame carries at most eight bytes",
  );
  assert.throws(
    () => can.fdFrame(0x100, Buffer.alloc(vector.invalidFdLength)),
    "13 bytes is not a length CAN-FD can carry",
  );

  vector.lengths.forEach((entry) => {
    assert.strictEqual(can.lenToDlc(entry.len), entry.dlc, `DLC for ${entry.len} bytes`);
  });
  vector.codes.forEach((entry) => {
    assert.strictEqual(can.dlcToLen(entry.dlc), entry.len, `length for DLC ${entry.dlc}`);
  });

  vector.j1939.forEach((entry) => {
    const message = can.decodeJ1939(entry.id);
    assert.strictEqual(message.pgn, entry.pgn, "parameter group");
    assert.strictEqual(message.priority, entry.priority, "priority");
    assert.strictEqual(message.source, entry.source, "source address");
    assert.strictEqual(message.destination, entry.destination, "destination address");
    assert.strictEqual(message.broadcast, entry.broadcast, "broadcast flag");
    assert.strictEqual(
      can.composeJ1939(entry.priority, entry.pgn, entry.source, entry.destination ?? 0),
      entry.id,
      "the identifier round-trips",
    );
  });

  assert.strictEqual(
    can.decodeJ1939(vector.standardIsNotJ1939, false),
    null,
    "J1939 never rides an 11-bit identifier",
  );
}

function gpioVectors() {
  const vector = VECTORS.gpio;

  vector.i2c.forEach((entry) => {
    assert.deepStrictEqual(
      gpio.i2c.addressFrame(entry.address, { tenBit: entry.tenBit }),
      unhex(entry.writeFrame),
      "write frame",
    );
    assert.deepStrictEqual(
      gpio.i2c.addressFrame(entry.address, { read: true, tenBit: entry.tenBit }),
      unhex(entry.readFrame),
      "read frame",
    );
    assert.strictEqual(gpio.i2c.frameLen(entry.address, entry.tenBit), entry.frameLen, "frame length");
    assert.strictEqual(gpio.i2c.isReserved(entry.address, entry.tenBit), entry.reserved, "reserved");
    assert.strictEqual(
      gpio.i2c.isGeneralCall(entry.address, entry.tenBit),
      entry.generalCall,
      "general call",
    );
  });

  assert.throws(() => gpio.i2c.addressFrame(vector.outOfRangeSevenBit), "7-bit range");
  assert.throws(() => gpio.i2c.addressFrame(vector.outOfRangeTenBit, { tenBit: true }), "10-bit range");

  vector.spi.forEach((entry) => {
    const clock = gpio.spi.clockFor(entry.mode);
    assert.strictEqual(clock.cpol, entry.cpol, `mode ${entry.mode} CPOL`);
    assert.strictEqual(clock.cpha, entry.cpha, `mode ${entry.mode} CPHA`);
    assert.strictEqual(gpio.spi.modeFor(entry.cpol, entry.cpha), entry.mode, "mode round-trips");
  });
  assert.throws(() => gpio.spi.clockFor(vector.invalidSpiMode), "there are only four SPI modes");

  vector.edges.forEach((entry) => {
    assert.strictEqual(
      gpio.pin.triggers(entry.edge, entry.from, entry.to),
      entry.triggered,
      `${entry.edge} on ${entry.from}->${entry.to}`,
    );
  });

  vector.polarities.forEach((entry) => {
    const level = gpio.pin.levelFor(entry.polarity, entry.asserted);
    assert.strictEqual(level, entry.level, `${entry.polarity} level`);
    assert.strictEqual(gpio.pin.isAsserted(entry.polarity, level), entry.isAsserted, "asserted");
  });
}

function sensorVectors() {
  const vector = VECTORS.sensors;

  const bme = vector.bme280;
  const calibration = sensors.bme280.calibration(
    unhex(bme.calibrationTempPress),
    unhex(bme.calibrationHumidity),
  );
  const reading = calibration.compensate(unhex(bme.measurement));
  close(reading.celsius, bme.celsius, "BME280 temperature", 1e-3);
  assert.strictEqual(reading.pascals, bme.pascals, "BME280 pressure");
  close(reading.hectopascals, bme.hectopascals, "BME280 pressure in hPa", 1e-2);
  close(
    reading.relativeHumidityPercent,
    bme.relativeHumidityPercent,
    "BME280 humidity",
    1e-3,
  );

  const ds = vector.ds18b20;
  const decoded = sensors.ds18b20.parseScratchpad(unhex(ds.scratchpad));
  assert.strictEqual(decoded.rawTemperature, ds.rawTemperature, "DS18B20 register");
  assert.strictEqual(decoded.microCelsius, ds.microCelsius, "DS18B20 temperature");
  assert.strictEqual(decoded.resolutionBits, ds.resolutionBits, "DS18B20 resolution");
  assert.strictEqual(decoded.alarmHigh, ds.alarmHigh, "DS18B20 high alarm");
  assert.strictEqual(sensors.ds18b20.crc8(unhex(ds.crcData)), ds.crc, "DS18B20 CRC");
  assert.throws(
    () => sensors.ds18b20.parseScratchpad(unhex(ds.corruptScratchpad)),
    "a read corrupted on the bus must not be trusted",
  );
  assert.throws(
    () => sensors.ds18b20.configByte(ds.invalidResolution),
    "a resolution the part does not offer is refused",
  );

  ds.resolutions.forEach((entry) => {
    assert.strictEqual(sensors.ds18b20.configByte(entry.bits), entry.configByte, "config byte");
    assert.strictEqual(
      sensors.ds18b20.stepMicroCelsius(entry.bits),
      entry.stepMicroCelsius,
      "resolution step",
    );
    assert.strictEqual(
      sensors.ds18b20.maxConversionMicros(entry.bits),
      entry.maxConversionMicros,
      "conversion time",
    );
    assert.strictEqual(
      sensors.ds18b20.resolutionBits(entry.configByte),
      entry.bits,
      "the resolution round-trips through its config byte",
    );
  });

  const ina = vector.ina219;
  assert.strictEqual(
    sensors.ina219.calibration(ina.currentLsbMicroamps, ina.shuntMilliohms),
    ina.calibration,
    "INA219 calibration",
  );
  assert.strictEqual(
    sensors.ina219.minimumCurrentLsbMicroamps(ina.maxExpectedMicroamps),
    ina.minimumCurrentLsbMicroamps,
    "INA219 minimum resolution",
  );
  assert.strictEqual(
    sensors.ina219.shuntMicrovolts(ina.rawShunt),
    ina.shuntMicrovolts,
    "INA219 shunt voltage",
  );
  assert.strictEqual(
    sensors.ina219.busMillivolts(ina.rawBus),
    ina.busMillivolts,
    "INA219 bus voltage",
  );
  assert.strictEqual(
    sensors.ina219.currentMicroamps(ina.rawCurrent, ina.currentLsbMicroamps),
    ina.currentMicroamps,
    "INA219 current",
  );
  assert.strictEqual(
    sensors.ina219.powerMicrowatts(ina.rawPower, ina.currentLsbMicroamps),
    ina.powerMicrowatts,
    "INA219 power",
  );

  const ads = vector.ads1115;
  const reset = sensors.ads1115.configFromBits(ads.configReset);
  Object.entries(ads.resetConfig).forEach(([field, want]) => {
    assert.strictEqual(reset[field], want, `ADS1115 reset ${field}`);
  });
  assert.strictEqual(
    sensors.ads1115.configBits(reset),
    ads.configReset,
    "the configuration round-trips through its register",
  );
  ads.gains.forEach((entry) => {
    assert.strictEqual(
      sensors.ads1115.fullScaleMicrovolts(entry.pga),
      entry.fullScaleMicrovolts,
      "ADS1115 full scale",
    );
    assert.strictEqual(
      sensors.ads1115.toNanovolts(entry.pga, 32767),
      entry.nanovoltsAtFullScale,
      "ADS1115 conversion",
    );
  });
  ads.rates.forEach((entry) => {
    assert.strictEqual(
      sensors.ads1115.samplesPerSecond(entry.dataRate),
      entry.samplesPerSecond,
      "ADS1115 sample rate",
    );
  });
}

function actuatorVectors() {
  const vector = VECTORS.actuators;

  const pca = vector.pca9685;
  assert.strictEqual(actuators.pca9685.internalOscHz, pca.internalOscHz, "oscillator");
  assert.strictEqual(actuators.pca9685.channels, pca.channels, "channel count");
  assert.strictEqual(actuators.pca9685.counts, pca.counts, "counts per period");
  pca.channelRegisters.forEach((entry) => {
    assert.strictEqual(
      actuators.pca9685.channelRegister(entry.channel),
      entry.register,
      "channel register",
    );
  });
  assert.throws(
    () => actuators.pca9685.channelRegister(pca.invalidChannel),
    "a channel beyond the part is refused",
  );
  assert.strictEqual(
    actuators.pca9685.prescaleForFrequency(pca.updateRateHz, pca.internalOscHz),
    pca.prescale,
    "prescale",
  );

  const pwm = vector.pwm;
  assert.deepStrictEqual(actuators.pwm.duty(pwm.duty.off), unhex(pwm.duty.bytes), "duty bytes");
  assert.deepStrictEqual(
    actuators.pwm.servo(pwm.servoCentre.pulseMicros, pwm.servoCentre.updateRateHz),
    unhex(pwm.servoCentre.bytes),
    "servo bytes",
  );
  assert.deepStrictEqual(actuators.pwm.fullOn(), unhex(pwm.fullOn), "full-on bytes");
  assert.deepStrictEqual(actuators.pwm.fullOff(), unhex(pwm.fullOff), "full-off bytes");

  const motor = vector.stepper;
  const stepper = new actuators.Stepper(actuators.StepDrive.HalfStep);
  const cycle = [stepper.coils];
  for (let step = 0; step < motor.stepCount; step += 1) {
    cycle.push(stepper.step(actuators.StepDirection.Forward));
  }
  assert.deepStrictEqual(cycle, motor.forwardCycle, "the forward cycle");
  assert.strictEqual(stepper.steps, motor.stepCount, "the position counts every step");
  assert.strictEqual(
    actuators.stepCount(actuators.StepDrive.HalfStep),
    motor.stepCount,
    "one cycle of half-step drive",
  );
  assert.strictEqual(
    actuators.stepsForDegrees(motor.degrees, motor.stepsPerRevolution),
    motor.stepsForDegrees,
    "a quarter turn is a quarter of the revolution",
  );
}

function windowedVectors() {
  const vector = VECTORS.windows;
  assert.strictEqual(WINDOW_CAPACITY, vector.capacity, "the documented capacity");

  const window = new Window();
  vector.window.readings.forEach((reading, index) => {
    window.push(reading);
    const want = vector.window.states[index];
    assert.strictEqual(window.len, want.len, "window length");
    close(window.mean(), want.mean, "window mean");
    close(window.min(), want.min, "window minimum");
    close(window.max(), want.max, "window maximum");
    close(window.range(), want.range, "window range");
  });

  const median = new Median();
  vector.median.readings.forEach((reading, index) => {
    close(median.update(reading), vector.median.outputs[index], "median");
  });

  const trend = new Trend();
  vector.trend.readings.forEach((reading, index) => {
    trend.push(reading);
    const want = vector.trend.slopes[index];
    if (want === null) {
      assert.strictEqual(trend.slope(), null, "no slope without enough readings");
    } else {
      close(trend.slope(), want, "trend slope", 1e-4);
    }
  });

  const anomaly = new Anomaly(vector.anomaly.sigmas);
  vector.anomaly.readings.forEach((reading, index) => {
    assert.strictEqual(
      anomaly.check(reading),
      vector.anomaly.flags[index],
      "the detector flags the reading that stands out",
    );
  });
}

identity();
codec();
helpers();
geofence();
serial();
modbusVectors();
canVectors();
gpioVectors();
sensorVectors();
actuatorVectors();

function loraVectors() {
  const vector = VECTORS.lora;

  for (const described of vector.links) {
    const link = linkOf(described);
    assert.strictEqual(
      lora.symbolTimeUs(link),
      described.symbolTimeUs,
      `symbol time for ${described.name}`,
    );

    for (const { payloadLen, airtimeUs } of described.airtimes) {
      assert.strictEqual(
        lora.airtimeUs(link, payloadLen),
        airtimeUs,
        `airtime of ${payloadLen} bytes on ${described.name}`,
      );
    }

    for (const { payloadLen, permille, offTimeUs } of described.budgets) {
      assert.strictEqual(
        lora.minOffTimeUs(link, payloadLen, permille),
        offTimeUs,
        `off time at ${permille} permille on ${described.name}`,
      );
    }
  }

  for (const { asked, used } of vector.clamped) {
    assert.strictEqual(
      lora.link(asked, 125_000).spreadingFactor,
      used,
      "a spreading factor outside 7 to 12 is clamped",
    );
  }

  // Rust saturates the off time when transmitting is forbidden; the facade
  // reports it as null instead, so a caller cannot mistake it for a real wait.
  const forbidden = vector.forbidden;
  const link = linkOf(vector.links.find((entry) => entry.name === forbidden.link));
  assert.strictEqual(
    lora.minOffTimeUs(link, forbidden.payloadLen, forbidden.permille),
    null,
    "a zero duty cycle forbids transmitting",
  );
  assert.strictEqual(
    lora.messagesPerHour(link, forbidden.payloadLen, forbidden.permille),
    0,
    "and so allows no messages at all",
  );
}

/** Rebuilds the link a vector describes. */
function linkOf(described) {
  return {
    spreadingFactor: described.spreadingFactor,
    bandwidthHz: described.bandwidthHz,
    codingRateDenominator: described.codingRateDenominator,
    preambleSymbols: described.preambleSymbols,
    explicitHeader: described.explicitHeader,
    crc: described.crc,
  };
}

function meshVectors() {
  const vector = VECTORS.mesh;

  assert.strictEqual(mesh.MAX_FRAME, vector.maxFrame, "the frame ceiling");
  assert.strictEqual(mesh.MAX_PAYLOAD, vector.maxPayload, "the payload ceiling");
  assert.strictEqual(mesh.BROADCAST, vector.broadcastAddress, "the broadcast address");
  assert.strictEqual(mesh.SEEN_DEFAULT_CAPACITY, vector.seenCapacity, "the cache size");

  const unicast = mesh.frame(
    vector.unicast.src,
    vector.unicast.dst,
    vector.unicast.id,
    unhex(vector.unicast.payload),
    vector.unicast.hopLimit,
  );
  assert.strictEqual(
    unicast.bytes.toString("hex"),
    vector.unicast.bytes,
    "an addressed frame matches byte for byte",
  );

  const broadcast = mesh.broadcast(
    vector.broadcast.src,
    vector.broadcast.id,
    unhex(vector.broadcast.payload),
  );
  assert.strictEqual(
    broadcast.bytes.toString("hex"),
    vector.broadcast.bytes,
    "a broadcast frame matches byte for byte",
  );

  const parsed = mesh.parse(unhex(vector.broadcast.bytes));
  assert.ok(parsed.broadcast, "and parses back as a broadcast");
  assert.strictEqual(parsed.payload.toString("hex"), vector.broadcast.payload);

  const relayed = mesh.relayed(unhex(vector.broadcast.bytes));
  assert.strictEqual(relayed.bytes.toString("hex"), vector.relayed.bytes, "relaying spends a hop");
  assert.strictEqual(relayed.hopLimit, vector.relayed.hopLimit);

  assert.strictEqual(
    mesh.relayed(unhex(vector.exhausted)),
    null,
    "a frame with no hops left must not be relayed",
  );

  assert.throws(
    () => mesh.parse(unhex(vector.corrupt)),
    "a frame the air mangled must be refused",
  );

  assert.strictEqual(
    mesh.crc16(unhex(vector.crc.check)),
    vector.crc.checkValue,
    "the published CRC-16/CCITT-FALSE check value",
  );
  assert.strictEqual(mesh.crc16(unhex(vector.crc.data)), vector.crc.value);

  const seen = new mesh.SeenPackets(vector.seenCapacity);
  vector.seen.keys.forEach(([src, id], index) => {
    assert.strictEqual(
      seen.record(src, id),
      vector.seen.new[index],
      "each packet is new exactly once",
    );
  });

  const small = new mesh.SeenPackets(vector.sizedSeen.capacity);
  assert.strictEqual(small.capacity, vector.sizedSeen.capacity, "the size it was given");
  for (const [src, id] of vector.sizedSeen.keys) {
    small.record(src, id);
  }
  assert.ok(
    !small.contains(...vector.sizedSeen.evicted),
    "a cache sized by the caller evicts at that size",
  );
}

function routingVectors() {
  const vector = VECTORS.routing;
  const router = routing.router(vector.address, vector.capacity);

  assert.strictEqual(router.capacity, vector.capacity, "the table size");

  for (const { origin, via, cost, changed } of vector.observations) {
    assert.strictEqual(
      router.observe(origin, via, cost),
      changed,
      `observing ${origin} via ${via} changes the table`,
    );
  }

  assert.strictEqual(router.size, vector.learned, "the routes it kept");

  const route = router.route(vector.route.dst);
  assert.strictEqual(route.nextHop, vector.route.nextHop, "the cheapest way it knows");
  assert.strictEqual(route.cost, vector.route.cost);

  for (const want of vector.decisions) {
    assertDecision(router, want);
  }

  router.forget(vector.afterForgetting.dst);
  assertDecision(router, vector.afterForgetting.decision);
  assert.strictEqual(router.size, vector.afterForgetting.learned);

  const small = routing.router(0x01, vector.sized.capacity);
  assert.strictEqual(small.capacity, vector.sized.capacity, "the size it was given");
  for (let node = 0; node < vector.sized.offered; node += 1) {
    small.observe(node + 0x100, 0x05, 4);
  }
  assert.strictEqual(
    small.size,
    vector.sized.learned,
    "a table sized by the caller holds exactly what it was asked for",
  );
}

/** Checks one routing decision against the vector that describes it. */
function assertDecision(router, want) {
  const decision = router.forward(want.dst);
  assert.strictEqual(decision.action, want.action, `packet for ${want.dst}`);
  assert.strictEqual(decision.nextHop, want.nextHop, `next hop for ${want.dst}`);
}

function lorawanVectors() {
  const vector = VECTORS.lorawan;
  const session = lorawan.session(
    vector.devAddr,
    unhex(vector.nwkSKey),
    unhex(vector.appSKey),
  );

  assert.strictEqual(session.devAddr, vector.devAddr, "the session is bound to its address");

  const uplink = session.encodeUplink(
    vector.uplink.fcnt,
    vector.uplink.fport,
    unhex(vector.uplink.payload),
    { confirmed: vector.uplink.confirmed, adr: vector.uplink.adr, ack: vector.uplink.ack },
  );
  assert.strictEqual(
    uplink.toString("hex"),
    vector.uplink.frame,
    "a secured uplink matches byte for byte",
  );

  const rx = session.decode(uplink, vector.uplink.fcnt);
  assert.strictEqual(rx.direction, lorawan.Direction.Uplink, "the frame went up");
  assert.strictEqual(rx.confirmed, vector.uplink.confirmed);
  assert.strictEqual(rx.adr, vector.uplink.adr);
  assert.strictEqual(rx.ack, vector.uplink.ack);
  assert.strictEqual(rx.payload.toString("hex"), vector.uplink.payload, "the payload decrypts");

  const downlink = session.encodeDownlink(
    vector.downlink.fcnt,
    vector.downlink.fport,
    unhex(vector.downlink.payload),
    {
      ack: vector.downlink.ack,
      fpending: vector.downlink.fpending,
      fopts: unhex(vector.downlink.fopts),
    },
  );
  assert.strictEqual(
    downlink.toString("hex"),
    vector.downlink.frame,
    "a secured downlink matches byte for byte",
  );

  const down = session.decode(downlink, vector.downlink.fcnt);
  assert.strictEqual(down.direction, lorawan.Direction.Downlink, "the frame came down");
  assert.strictEqual(down.fpending, vector.downlink.fpending);
  assert.strictEqual(down.fopts.toString("hex"), vector.downlink.fopts);

  assert.throws(
    () => session.decode(unhex(vector.forgedUplink), vector.uplink.fcnt),
    "a frame altered after signing must not verify",
  );
  assert.throws(
    () => session.decode(uplink, vector.wrongCounter),
    "a frame out of its place in the counter stream must not verify",
  );

  const device = lorawan.device(
    unhex(vector.join.devEui),
    unhex(vector.join.appEui),
    unhex(vector.join.appKey),
  );
  assert.strictEqual(
    device.joinRequest(vector.join.devNonce).toString("hex"),
    vector.join.request,
    "the join request matches byte for byte",
  );
  assert.throws(
    () => device.acceptJoin(unhex(vector.join.forgedAccept), vector.join.devNonce),
    "a join the network never signed must not activate a session",
  );
}

windowedVectors();
loraVectors();
meshVectors();
routingVectors();

function headerVectors() {
  const vector = VECTORS.header;

  for (const want of vector.frames) {
    const header = lorawan.parseHeader(unhex(want.frame));
    assert.strictEqual(header.messageType, want.messageType, "the message type");
    assert.strictEqual(header.isData, want.isData, "whether it is a data frame");
    assert.strictEqual(header.devAddr, want.devAddr ?? null, "the address a receiver routes by");
    assert.strictEqual(header.fcnt, want.fcnt ?? null, "the counter");
    assert.strictEqual(header.fport, want.fport ?? null, "the port");
    assert.strictEqual(header.confirmed, want.confirmed, "the confirmed bit");
    assert.strictEqual(header.adr, want.adr, "the ADR bit");
    assert.strictEqual(header.ack, want.ack, "the ACK bit");
    assert.strictEqual(header.fpending, want.fpending, "the pending bit");
    assert.strictEqual(header.foptsLen, want.foptsLen, "the options length");
    assert.strictEqual(header.payloadLen, want.payloadLen, "the payload length");
  }

  assert.throws(
    () => lorawan.parseHeader(unhex(vector.unsupported)),
    "a message type this build does not read must be refused",
  );
  assert.throws(
    () => lorawan.parseHeader(unhex(vector.truncated)),
    "a frame too short to hold a header must be refused",
  );
}

/** Checks a grant builds its accept and derives the session both sides share. */
function assertGrant(vector, appKey, devNonce) {
  const grant = {
    appNonce: vector.appNonce,
    netId: vector.netId,
    devAddr: vector.devAddr,
    dlSettings: vector.dlSettings,
    rxDelay: vector.rxDelay,
    cflist: vector.cflist === undefined ? undefined : unhex(vector.cflist),
  };
  assert.strictEqual(
    lorawan.grantAccept(grant, appKey, devNonce).toString("hex"),
    vector.accept,
    "the signed join-accept matches byte for byte",
  );

  // Neither side sent a key, so the proof they agree is that one reads what the
  // other wrote.
  const session = lorawan.grantSession(grant, appKey, devNonce);
  const probe = vector.probe;
  assert.strictEqual(
    session.encodeUplink(probe.fcnt, probe.fport, unhex(probe.payload)).toString("hex"),
    probe.frame,
    "the session this network derived is the one the device holds",
  );
}

function networkVectors() {
  const vector = VECTORS.network;
  const appKey = unhex(vector.appKey);

  const request = lorawan.parseJoinRequest(unhex(vector.joinRequest.frame), appKey);
  assert.strictEqual(request.devEui.toString("hex"), vector.joinRequest.devEui);
  assert.strictEqual(request.appEui.toString("hex"), vector.joinRequest.appEui);
  assert.strictEqual(request.devNonce, vector.joinRequest.devNonce);

  assert.throws(
    () => lorawan.parseJoinRequest(unhex(vector.forgedRequest), appKey),
    "a request signed with another root key must not be trusted",
  );

  assertGrant(vector.grant, appKey, vector.devNonce);

  // The captured join: a third party's numbers, so agreement here is not just this
  // implementation agreeing with itself.
  const published = vector.published;
  const publishedKey = unhex(published.appKey);
  assertGrant(published, publishedKey, published.devNonce);

  const device = lorawan.device(Buffer.alloc(8), Buffer.alloc(8), publishedKey);
  const accepted = device.acceptJoin(unhex(published.accept), published.devNonce);
  assert.strictEqual(accepted.devAddr, published.devAddr, "the captured accept activates");

  const probe = published.probe;
  assert.strictEqual(
    accepted
      .session()
      .encodeUplink(probe.fcnt, probe.fport, unhex(probe.payload))
      .toString("hex"),
    probe.frame,
    "the session the device derived matches the published keys",
  );
}


// A signed, chained log: the records, and what breaks the chain.
function auditVectors() {
  const vector = VECTORS.audit;
  const keeper = new DeviceIdentity(unhex(vector.seed));
  assert.strictEqual(
    keeper.publicKey().toString("hex"),
    vector.publicKey,
    "the key a chain is checked against",
  );

  const log = new audit.AuditLog(keeper);
  const entries = [];
  for (const want of vector.entries) {
    const entry = log.append(Buffer.from(want.payload));
    assert.strictEqual(Number(entry.index), want.index, "the index");
    assert.strictEqual(
      Buffer.from(entry.previous).toString("hex"),
      want.previous,
      "each record carries the hash of the one before it",
    );
    assert.strictEqual(Buffer.from(entry.digest).toString("hex"), want.digest, "the digest");
    assert.strictEqual(
      Buffer.from(entry.signature).toString("hex"),
      want.signature,
      "the signature",
    );
    assert.strictEqual(
      entry.toBytes().toString("hex"),
      want.bytes,
      "a record encodes the same in every language",
    );
    entries.push(entry);
  }

  assert.ok(
    audit.verifyChain(keeper.publicKey(), entries),
    "an untouched chain verifies",
  );
  assert.ok(
    !audit.verifyChain(keeper.publicKey(), [
      entries[0],
      entries[1],
      audit.AuditEntry.fromBytes(unhex(vector.tampered)),
    ]),
    "and an altered record breaks it",
  );

  const resumed = audit.AuditLog.resume(keeper, entries[2]);
  const afterReboot = resumed.append(Buffer.from(vector.resumed.payload));
  assert.strictEqual(Number(afterReboot.index), vector.resumed.index, "a reboot leaves no gap");
  assert.strictEqual(afterReboot.toBytes().toString("hex"), vector.resumed.bytes);
}

// A secured channel: the agreed keys, the sealed bytes, and the refusals.
function sessionVectors() {
  const vector = VECTORS.session;
  const node = new session.AgreementKey(unhex(vector.nodeSeed));
  const gateway = new session.AgreementKey(unhex(vector.gatewaySeed));

  assert.strictEqual(node.publicKey().toString("hex"), vector.nodePublicKey, "the node key");
  assert.strictEqual(
    gateway.publicKey().toString("hex"),
    vector.gatewayPublicKey,
    "the gateway key",
  );

  const salt = unhex(vector.salt);
  const aad = Buffer.from(vector.aad);
  const uplink = new session.Session(node, gateway.publicKey(), salt, session.Role.Initiator);
  const downlink = new session.Session(gateway, node.publicKey(), salt, session.Role.Responder);

  for (const want of vector.messages) {
    const message = uplink.seal(Buffer.from(want.plaintext), aad);
    assert.strictEqual(message.counter, want.counter, "the counter");
    assert.strictEqual(message.tag.toString("hex"), want.tag, "the tag");
    assert.strictEqual(
      message.ciphertext.toString("hex"),
      want.ciphertext,
      "the same key and counter produce the same bytes everywhere",
    );
    assert.strictEqual(
      downlink.open(message, aad).toString(),
      want.plaintext,
      "the peer recovers the reading",
    );
  }

  const first = vector.messages[0];
  const replayed = {
    counter: first.counter,
    tag: unhex(first.tag),
    ciphertext: unhex(first.ciphertext),
  };
  assert.throws(
    () => downlink.open(replayed, aad),
    /repeat|replay/i,
    "a repeated counter is refused",
  );

  const fresh = new session.Session(gateway, node.publicKey(), salt, session.Role.Responder);
  assert.throws(
    () => fresh.open(replayed, Buffer.from(vector.wrongAad)),
    /authenticat/i,
    "and associated data that does not match fails authentication",
  );

  assert.strictEqual(
    session
      .hmacSha256(Buffer.from(vector.hmac.key), Buffer.from(vector.hmac.message))
      .toString("hex"),
    vector.hmac.digest,
    "the keyed hash",
  );
  assert.strictEqual(
    session
      .hkdfSha256(
        Buffer.from(vector.hkdf.salt),
        Buffer.from(vector.hkdf.ikm),
        Buffer.from(vector.hkdf.info),
        vector.hkdf.length,
      )
      .toString("hex"),
    vector.hkdf.output,
    "the expansion",
  );
}

// A signed release: the manifest bytes, the envelope, and the slot lifecycle.
function updateVectors() {
  const vector = VECTORS.update;
  const publisher = new DeviceIdentity(unhex(vector.publisherSeed));
  assert.strictEqual(
    publisher.publicKey().toString("hex"),
    vector.publisherPublicKey,
    "the key a device trusts",
  );

  const manifest = {
    structureVersion: vector.manifest.structureVersion,
    sequence: vector.manifest.sequence,
    vendorId: unhex(vector.vendorId),
    classId: unhex(vector.classId),
    format: vector.manifest.format,
    storage: vector.manifest.storage,
    digest: unhex(vector.manifest.digest),
    size: vector.manifest.size,
    expires: vector.manifest.expires,
  };
  const image = Buffer.alloc(vector.imageLen, vector.imageByte);

  assert.strictEqual(
    update.encodeManifest(manifest).toString("hex"),
    vector.body,
    "a manifest encodes the same in every language",
  );

  const envelope = update.signManifest(manifest, publisher);
  assert.strictEqual(envelope.toString("hex"), vector.envelope, "the signed envelope");
  assert.strictEqual(
    update.verifyEnvelope(envelope, publisher.publicKey()).digest.toString("hex"),
    vector.manifest.digest,
    "which verifies against the key that signed it",
  );
  assert.throws(
    () => update.verifyEnvelope(unhex(vector.forgedEnvelope), publisher.publicKey()),
    /signature/i,
    "a release signed by another key is refused",
  );

  const anchor = new DeviceIdentity(unhex(vector.anchorSeed));
  assert.strictEqual(
    update
      .signDelegation(
        {
          epoch: vector.delegation.epoch,
          releaseKey: unhex(vector.delegation.releaseKey),
          expires: vector.delegation.expires,
        },
        anchor,
      )
      .toString("hex"),
    vector.delegation.envelope,
    "the signed delegation",
  );

  const life = vector.lifecycle;
  const fleet = new update.Updater(
    unhex(vector.vendorId),
    unhex(vector.classId),
    publisher.publicKey(),
    2,
    4096,
  );
  fleet.provision(0, 1);
  assert.strictEqual(fleet.begin(envelope), life.staged, "the release names the same slot");
  for (let at = 0; at < image.length; at += life.chunk) {
    fleet.write(image.subarray(at, at + life.chunk));
  }

  assert.strictEqual(fleet.finish(), life.staged, "and the image matched what was promised");

  const boot = fleet.onBoot();
  assert.strictEqual(boot.action, life.boot, "the boot decision");
  assert.strictEqual(boot.slot, life.bootSlot, "the slot it is about");
  assert.strictEqual(fleet.confirm(), life.confirmed, "the confirmed slot");

  const record = fleet.slotRecord(life.confirmed);
  assert.strictEqual(record.state, life.state, "the slot state");
  assert.strictEqual(record.written, life.written, "the bytes written");
}

// How a work interval stretches as a battery falls.
function powerVectors() {
  const vector = VECTORS.power;
  const plan = new power.PowerPlan(
    vector.plan.activeUs,
    vector.plan.saverUs,
    vector.plan.criticalUs,
  );
  close(plan.saverBelow, vector.plan.saverBelow, "the saver threshold");
  close(plan.criticalBelow, vector.plan.criticalBelow, "the critical threshold");

  vector.charges.forEach((soc, at) => {
    assert.strictEqual(plan.mode(soc), vector.modes[at], `the mode at ${soc}`);
    assert.strictEqual(
      plan.modeWhileCharging(soc, true),
      vector.charging[at],
      `the mode while charging at ${soc}`,
    );
    assert.strictEqual(plan.intervalUs(soc), vector.intervalsUs[at], `the interval at ${soc}`);
  });

  const duty = power.DutyCycle.fromFraction(vector.duty.periodUs, vector.duty.fraction);
  assert.strictEqual(duty.activeUs, vector.duty.activeUs, "the time awake");
  assert.strictEqual(duty.sleepUs, vector.duty.sleepUs, "the time asleep");
}

// What a reporter ships and what it drops once the link gets expensive.
function telemetryVectors() {
  const vector = VECTORS.telemetry;
  vector.costs.forEach((cost, at) => {
    assert.strictEqual(
      telemetry.linkCostThreshold(cost),
      vector.thresholds[at],
      `the bar ${cost} sets`,
    );
  });

  const reporter = new telemetry.Reporter(telemetry.Level.Trace);
  reporter.adaptTo(vector.adaptedTo);
  vector.levels.forEach((level, at) => {
    const shipped = reporter.record({ level, code: "vector" });
    assert.strictEqual(
      shipped !== null,
      vector.shipped[at],
      `whether event ${at} is worth its bytes`,
    );
  });

  const snapshot = reporter.snapshot();
  for (const key of ["trace", "debug", "info", "warn", "error", "emitted", "dropped"]) {
    assert.strictEqual(snapshot[key], vector.snapshot[key], `the ${key} count`);
  }
}

lorawanVectors();
headerVectors();
networkVectors();

// What a ladder does with a message as its links come and go.
async function ladderVectors() {
  const vector = VECTORS.ladder;
  const broker = new loopback.LoopbackBroker();
  const listener = broker.link();
  await listener.connect();
  await listener.subscribe(vector.topic);

  const offline = new ladder.Ladder(sync.Store.memory());
  for (const [at, payload] of vector.payloads.entries()) {
    assert.strictEqual(
      await offline.send(vector.topic, Buffer.from(payload)),
      vector.withNoRung.deliveries[at],
      "a message no rung takes is buffered rather than lost",
    );
  }
  assert.strictEqual(await offline.buffered(), vector.withNoRung.buffered, "the buffer holds them");

  await offline.rung(broker.rung());
  await offline.connect();
  assert.strictEqual(
    await offline.flush(),
    vector.afterTheLinkReturns.flushed,
    "the buffer replays once a link returns",
  );
  assert.strictEqual(await offline.buffered(), vector.afterTheLinkReturns.buffered);

  const rungs = new ladder.Ladder(sync.Store.memory());
  await rungs.rung(
    transport.Transport.faulty(broker.rung(), vector.fallthrough.failuresOnFirstRung),
  );
  await rungs.rung(broker.rung());
  await rungs.connect();
  assert.strictEqual(
    await rungs.send(vector.topic, Buffer.from(vector.fallthrough.payload)),
    vector.fallthrough.delivery,
    "a rung that refuses falls through to the next",
  );
}

// What the simulated devices produce, so every binding invents the same run.
async function simulationVectors() {
  const vector = VECTORS.simulation;

  const sensor = new sim.SimulatedSensor(
    vector.sensor.baseline,
    vector.sensor.driftPerRead,
    vector.sensor.noise,
    vector.sensor.seed,
  );
  for (const want of vector.sensor.readings) {
    assert.strictEqual(
      await sensor.read(),
      want,
      "a seeded sensor invents the same run everywhere",
    );
  }

  const replay = new sim.Replay(vector.replay.capture, vector.replay.repeating);
  for (const want of vector.replay.readings) {
    assert.strictEqual(await replay.read(), want, "a capture reads back the same");
  }

  const robot = new sim.SimulatedRobot(vector.robot.dt);
  for (const want of vector.robot.poses) {
    await robot.apply({ vx: vector.robot.vx, vy: 0, omega: vector.robot.omega });
    const pose = robot.pose;
    close(pose.x, want.x, "the x it reached");
    close(pose.y, want.y, "the y it reached");
    close(pose.theta, want.theta, "the heading it holds");
  }
}

auditVectors();
sessionVectors();
updateVectors();
powerVectors();
telemetryVectors();

(async () => {
  await ladderVectors();
  await simulationVectors();
  console.log("conformance ok");
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
