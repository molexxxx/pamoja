# Pamoja.Transports

Reaching the network when no single link always works, and testing all of it with nothing plugged in.

One reference for the 7 capabilities of this domain. Each is also its own package,
and `Pamoja` is the whole framework in one.

```sh
dotnet add package Pamoja.Transports
```

This package ships no assembly: it brings in the packages below, and each keeps its own
namespace, so a type is named the way it is when the package is referenced directly.

| Capability | Package | What it covers |
| --- | --- | --- |
| [MQTT](https://pamoja.molex.cloud/docs/guides/mqtt.html) | `Pamoja.Mqtt` | An MQTT client with the topic and wildcard rules, as the core transport |
| [CoAP](https://pamoja.molex.cloud/docs/guides/coap.html) | `Pamoja.Coap` | A CoAP client over UDP with confirmable delivery and observe |
| [Loopback](https://pamoja.molex.cloud/docs/guides/loopback.html) | `Pamoja.Loopback` | An in-process transport with topic matching and a fault injector, for testing with no broker |
| [Store and forward](https://pamoja.molex.cloud/docs/guides/sync.html) | `Pamoja.Sync` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
| [Transport ladder](https://pamoja.molex.cloud/docs/guides/ladder.html) | `Pamoja.Ladder` | Cheapest reachable link first, buffering to a store when every link is down |
| [Event bus](https://pamoja.molex.cloud/docs/guides/bus.html) | `Pamoja.Bus` | An in-memory typed publish and subscribe event bus |
| [Simulators](https://pamoja.molex.cloud/docs/guides/sim.html) | `Pamoja.Sim` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |

The guides, with a worked C# example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
