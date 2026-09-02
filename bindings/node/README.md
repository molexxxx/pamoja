# @pamoja/core

Node.js bindings for the [pamoja](https://github.com/molexxxx/pamoja)
device SDK core, built with [napi-rs](https://napi.rs).

The generated surface is intentionally thin. A hand-written, idiomatic layer is
added on top of it so JavaScript and TypeScript callers get a native-feeling API
while all behavior stays in the Rust core.

## What is here

| Import | Covers |
| --- | --- |
| `@pamoja/core/mqtt` | an MQTT client with async iteration over incoming messages |
| `@pamoja/core/security` | device identity: sign a reading, verify one, label a key |
| `@pamoja/core/codec` | JSON to CBOR and back, and packing samples for a metered link |
| `@pamoja/core/kit` | the helper math: smoothing, PID, thermostat, depletion, geofencing |
| `@pamoja/core/serial` | SLIP and COBS packet framing, with streaming decoders for a UART |
| `@pamoja/core/modbus` | Modbus RTU requests and replies for RS485 field devices |
| `@pamoja/core/can` | CAN 2.0 and CAN-FD frames, and the J1939 identifier above them |
| `@pamoja/core/gpio` | I2C addressing, the SPI clock modes, and active-low pin logic |
| `@pamoja/core/sensors` | BME280, DS18B20, INA219, and ADS1115 register decoding |
| `@pamoja/core/actuators` | PCA9685 PWM and servo commands, and stepper coil sequencing |

`@pamoja/core` re-exports them all, and the generated low-level contract stays
available at `@pamoja/core/raw` for anything the facade does not surface. The
hardware capabilities arrive as namespaces (`serial`, `modbus`, `can`, `gpio`,
`sensors`, `actuators`), because their operations are named for their protocol or
part rather than for the SDK.

```js
const { DeviceIdentity, Smoother, toCbor } = require("@pamoja/core");

const smoother = new Smoother(0.3);
const reading = smoother.update(21.7);

const device = DeviceIdentity.fromSeed(seed);
const payload = toCbor({ c: reading });
const signature = device.sign(payload);
```

Talking to the wires a gateway actually has looks the same way:

```js
const { modbus, serial } = require("@pamoja/core");

// Ask an RS485 energy meter for three holding registers.
port.write(modbus.readHoldingRegisters(0x11, 0x006b, 3));

// Reassemble whole packets from the chunks the port delivers.
const decoder = new serial.SlipDecoder();
port.on("data", (chunk) => {
  for (const frame of decoder.feed(chunk)) handle(frame);
});
```

## Build

```
npm install
npm run build
npm test
```

`npm test` runs the smoke test and then the cross-language conformance suite,
which asserts the same vectors every other binding does.

`npm run build` compiles the Rust core into a native Node addon and emits
`index.js` and `index.d.ts`. Both are generated artifacts, but they are
committed and drift-checked in CI, so they can never fall behind the Rust
source. `index.js` also carries the package version, so a version bump means
rebuilding and committing it.
