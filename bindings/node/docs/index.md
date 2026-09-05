One memory-safe Rust core for IoT, robotics, and drones, behind an idiomatic TypeScript
facade. This is the generated reference for every package; the guides, with the same
worked example in four languages, are on [the documentation site](https://pamoja.molex.cloud/docs/).

## Install

```sh
npm install pamoja
```

Each capability is also its own package, so an application that needs one thing depends on one thing.

## A first example

A reading off a wire, smoothed, signed, and packed for a metered link, with nothing plugged in. This runs in CI, and is spliced here from the test that runs it.

From [`bindings/node/guides/quickstart.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/quickstart.ts):

```typescript
import { packSamples, unpackSamples } from '@pamoja/codec'
import { Smoother } from '@pamoja/kit'
import { DeviceIdentity, verify } from '@pamoja/security'
import { ds18b20 } from '@pamoja/sensors'

// A stand-in for the thermometer. On a running node these nine bytes arrive from the
// 1-Wire bus; here the library builds what a part sitting at 25.0625 C would send, so the
// program runs with nothing plugged in.
const offTheBus = ds18b20.buildScratchpad(25.0625, 12, 75, -10)

// Everything below is the node's own code, and none of it cares where the bytes came from.
// The part checksums every read, so a value mangled on a long run comes back as an error
// instead of a plausible temperature a couple of degrees off.
const celsius = ds18b20.parseScratchpad(offTheBus).microCelsius / 1e6
console.log(`read      ${celsius.toFixed(4)} C`) // read      25.0625 C

// Readings jitter. A smoother follows the trend without keeping a history to do it, which
// matters on a part with kilobytes of RAM.
const smoother = new Smoother(0.5)
smoother.update(celsius)
const smoothed = smoother.update(celsius + 1)
console.log(`smoothed  ${smoothed.toFixed(4)} C`) // smoothed  25.5625 C

// Sign it, so the gateway can tell this device's readings from anyone else's.
const device = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const reading = smoothed.toFixed(2)
const signature = device.sign(reading)
if (!verify(device.publicKey(), reading, signature)) {
  throw new Error('the gateway would reject this reading')
}
console.log(`signed    ${reading} C, and the signature checks out`)

// Send a batch rather than a reading at a time. Successive samples differ by very little,
// so writing down the differences costs a fraction of eight bytes each.
const batch = [2506, 2507, 2509, 2508, 2510]
const packed = packSamples(batch)
console.log(`packed    ${batch.length} readings into ${packed.length} bytes`)
```

## Every package

| Chapter | Import | What it covers |
| --- | --- | --- |
| **Engine** | [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html) | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| **Identity** | [`@pamoja/security`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html) | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| **Codecs** | [`@pamoja/codec`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html) | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| **Helpers** | [`@pamoja/kit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html) | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| **Field I/O** | [`@pamoja/serial`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html) | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
|  | [`@pamoja/modbus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html) | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
|  | [`@pamoja/can`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html) | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
|  | [`@pamoja/gpio`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html) | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| **Sensing and actuation** | [`@pamoja/sensors`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html) | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
|  | [`@pamoja/actuators`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html) | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| **Radio and reach** | [`@pamoja/lora`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html) | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
|  | [`@pamoja/lorawan`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html) | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
|  | [`@pamoja/mesh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html) | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
|  | [`@pamoja/routing`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html) | Reverse-path routing that learns the cheapest route from overheard traffic |
| **MAVLink** | [`@pamoja/mavlink`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html) | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| **Trust and operation** | [`@pamoja/audit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html) | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
|  | [`@pamoja/session`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
|  | [`@pamoja/update`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html) | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
|  | [`@pamoja/power`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html) | Duty cycling and an energy-aware governor that stretches work as the battery drains |
|  | [`@pamoja/telemetry`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html) | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| **Transports and testing** | [`@pamoja/mqtt`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html) | An MQTT client with the topic and wildcard rules, as the core transport |
|  | [`@pamoja/coap`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html) | A CoAP client over UDP with confirmable delivery and observe |
|  | [`@pamoja/loopback`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html) | An in-process transport with topic matching and a fault injector, for testing with no broker |
|  | [`@pamoja/sync`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html) | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
|  | [`@pamoja/ladder`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html) | Cheapest reachable link first, buffering to a store when every link is down |
|  | [`@pamoja/bus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html) | An in-memory typed publish and subscribe event bus |
|  | [`@pamoja/sim`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html) | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| **Profiles and robotics** | [`@pamoja/profile`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html) | Named, ready-to-run device profiles from plain data or a JSON manifest |
|  | [`@pamoja/ros2`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html) | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
|  | [`@pamoja/zenoh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html) | Zenoh key expressions: validity, canonical form, and wildcard matching |

## Elsewhere

- [The guides: one page per capability, each with a worked TypeScript example](https://pamoja.molex.cloud/docs/)
- [The install page: taking less than all of it, and what that saves](https://pamoja.molex.cloud/docs/install.html)
- [The source, and every other binding](https://github.com/molexxxx/pamoja)
