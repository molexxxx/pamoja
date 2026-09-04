# @pamoja/transports

Reaching the network when no single link always works, and testing all of it with nothing plugged in.

One install for the 7 capabilities of this domain. Each is also its own package, and
`pamoja` is the whole framework in one.

```sh
npm install @pamoja/transports
```

| Capability | Package | What it covers |
| --- | --- | --- |
| [MQTT](https://pamoja.molex.cloud/docs/guides/mqtt.html) | `@pamoja/mqtt` | An MQTT client with the topic and wildcard rules, as the core transport |
| [CoAP](https://pamoja.molex.cloud/docs/guides/coap.html) | `@pamoja/coap` | A CoAP client over UDP with confirmable delivery and observe |
| [Loopback](https://pamoja.molex.cloud/docs/guides/loopback.html) | `@pamoja/loopback` | An in-process transport with topic matching and a fault injector, for testing with no broker |
| [Store and forward](https://pamoja.molex.cloud/docs/guides/sync.html) | `@pamoja/sync` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
| [Transport ladder](https://pamoja.molex.cloud/docs/guides/ladder.html) | `@pamoja/ladder` | Cheapest reachable link first, buffering to a store when every link is down |
| [Event bus](https://pamoja.molex.cloud/docs/guides/bus.html) | `@pamoja/bus` | An in-memory typed publish and subscribe event bus |
| [Simulators](https://pamoja.molex.cloud/docs/guides/sim.html) | `@pamoja/sim` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |

The guides, with a worked TypeScript example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
