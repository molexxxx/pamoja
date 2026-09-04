# pamoja for C#

One memory-safe Rust core for IoT, robotics, and drones, behind an idiomatic C#
facade. This is the generated reference for every package; the guides, with the same
worked example in four languages, are on [the documentation site](https://pamoja.molex.cloud/docs/).

## Install

```sh
dotnet add package Pamoja
```

```csharp
using Pamoja.Mqtt;
```

Each capability is also its own package, so an application that needs one thing depends on one thing.

## A first example

A reading off a wire, smoothed, signed, and packed for a metered link, with nothing plugged in. This runs in CI, and is spliced here from the test that runs it.

From [`bindings/dotnet/samples/Pamoja.Guides/Quickstart.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Quickstart.cs):

```csharp
// The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
byte[] scratchpad = [0x91, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
scratchpad[8] = Ds18b20.Crc8(scratchpad.AsSpan(0, 8));
float celsius = Ds18b20.ParseScratchpad(scratchpad).MicroCelsius / 1e6f;
Expect(celsius == 25.0625f, "the register decodes to 25.0625 C");

// Smooth the noise out of successive readings.
using var smoother = new Smoother(0.5f);
smoother.Update(celsius);
float smoothed = smoother.Update(celsius + 1.0f);
Expect(smoothed > celsius && smoothed < celsius + 1.0f, "smoothing lags the step");

// Sign the reading so a gateway can prove which device sent it.
byte[] seed = new byte[DeviceIdentity.KeyLength];
Array.Fill(seed, (byte)7);
using var device = new DeviceIdentity(seed);
string payload = smoothed.ToString("F2", CultureInfo.InvariantCulture);
byte[] signature = device.Sign(payload);
Expect(DeviceIdentity.Verify(device.PublicKey, payload, signature), "the signature verifies");

// Pack a batch of readings for a link where every byte costs money.
long[] samples = [2506, 2507, 2509, 2508, 2510];
byte[] packed = Codec.PackSamples(samples);
Expect(packed.Length < samples.Length * 8, "packing beats eight bytes a sample");
Expect(Codec.UnpackSamples(packed).SequenceEqual(samples), "and the batch round-trips");
```

## Every package

| Chapter | Package | What it covers |
| --- | --- | --- |
| **Engine** | [`Pamoja.Core`](api/Pamoja.Core.html) | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| **Identity** | [`Pamoja.Security`](api/Pamoja.Security.html) | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| **Codecs** | [`Pamoja.Codec`](api/Pamoja.Codec.html) | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| **Helpers** | [`Pamoja.Kit`](api/Pamoja.Kit.html) | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| **Field I/O** | [`Pamoja.Serial`](api/Pamoja.Serial.html) | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
|  | [`Pamoja.Modbus`](api/Pamoja.Modbus.html) | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
|  | [`Pamoja.Can`](api/Pamoja.Can.html) | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
|  | [`Pamoja.Gpio`](api/Pamoja.Gpio.html) | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| **Sensing and actuation** | [`Pamoja.Sensors`](api/Pamoja.Sensors.html) | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
|  | [`Pamoja.Actuators`](api/Pamoja.Actuators.html) | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| **Radio and reach** | [`Pamoja.Lora`](api/Pamoja.Lora.html) | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
|  | [`Pamoja.Lorawan`](api/Pamoja.Lorawan.html) | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
|  | [`Pamoja.Mesh`](api/Pamoja.Mesh.html) | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
|  | [`Pamoja.Routing`](api/Pamoja.Routing.html) | Reverse-path routing that learns the cheapest route from overheard traffic |
| **MAVLink** | [`Pamoja.Mavlink`](api/Pamoja.Mavlink.html) | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| **Trust and operation** | [`Pamoja.Audit`](api/Pamoja.Audit.html) | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
|  | [`Pamoja.Session`](api/Pamoja.Session.html) | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
|  | [`Pamoja.Update`](api/Pamoja.Update.html) | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
|  | [`Pamoja.Power`](api/Pamoja.Power.html) | Duty cycling and an energy-aware governor that stretches work as the battery drains |
|  | [`Pamoja.Telemetry`](api/Pamoja.Telemetry.html) | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| **Transports and testing** | [`Pamoja.Mqtt`](api/Pamoja.Mqtt.html) | An MQTT client with the topic and wildcard rules, as the core transport |
|  | [`Pamoja.Coap`](api/Pamoja.Coap.html) | A CoAP client over UDP with confirmable delivery and observe |
|  | [`Pamoja.Loopback`](api/Pamoja.Loopback.html) | An in-process transport with topic matching and a fault injector, for testing with no broker |
|  | [`Pamoja.Sync`](api/Pamoja.Sync.html) | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
|  | [`Pamoja.Ladder`](api/Pamoja.Ladder.html) | Cheapest reachable link first, buffering to a store when every link is down |
|  | [`Pamoja.Bus`](api/Pamoja.Bus.html) | An in-memory typed publish and subscribe event bus |
|  | [`Pamoja.Sim`](api/Pamoja.Sim.html) | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| **Profiles and robotics** | [`Pamoja.Profile`](api/Pamoja.Profile.html) | Named, ready-to-run device profiles from plain data or a JSON manifest |
|  | [`Pamoja.Ros2`](api/Pamoja.Ros2.html) | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
|  | [`Pamoja.Zenoh`](api/Pamoja.Zenoh.html) | Zenoh key expressions: validity, canonical form, and wildcard matching |

## Elsewhere

- [The guides: one page per capability, each with a worked C# example](https://pamoja.molex.cloud/docs/)
- [The install page: taking less than all of it, and what that saves](https://pamoja.molex.cloud/docs/install.html)
- [The source, and every other binding](https://github.com/molexxxx/pamoja)
