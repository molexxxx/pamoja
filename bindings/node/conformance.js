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
  can,
  gpio,
  modbus,
  serial: serialFraming,
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

identity();
codec();
helpers();
geofence();
serial();
modbusVectors();
canVectors();
gpioVectors();

console.log("conformance ok");
