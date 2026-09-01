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

identity();
codec();
helpers();
geofence();

console.log("conformance ok");
