# @pamoja/radio

Budgeting airtime, framing a mesh packet, routing it, and securing a LoRaWAN uplink: everything a node needs to reach a network it cannot see.

One install for the 4 capabilities of this domain. Each is also its own package, and
`pamoja` is the whole framework in one.

```sh
npm install @pamoja/radio
```

| Capability | Package | What it covers |
| --- | --- | --- |
| [LoRa airtime and range](https://pamoja.molex.cloud/docs/guides/lora.html) | `@pamoja/lora` | Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to, and the link budget that sets its range |
| [LoRaWAN](https://pamoja.molex.cloud/docs/guides/lorawan.html) | `@pamoja/lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| [Mesh frames](https://pamoja.molex.cloud/docs/guides/mesh.html) | `@pamoja/mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| [Routing](https://pamoja.molex.cloud/docs/guides/routing.html) | `@pamoja/routing` | Reverse-path routing that learns the cheapest route from overheard traffic |

The guides, with a worked TypeScript example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
