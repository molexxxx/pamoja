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

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
