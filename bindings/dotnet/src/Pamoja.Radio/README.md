# Pamoja.Radio

Budgeting airtime, framing a mesh packet, routing it, and securing a LoRaWAN uplink: everything a node needs to reach a network it cannot see.

One reference for the 5 capabilities of this domain. Each is also its own package,
and `Pamoja` is the whole framework in one.

```sh
dotnet add package Pamoja.Radio
```

This package ships no assembly: it brings in the packages below, and each keeps its own
namespace, so a type is named the way it is when the package is referenced directly.

| Capability | Package | What it covers |
| --- | --- | --- |
| [LoRa airtime and range](https://pamoja.molex.cloud/docs/guides/lora.html) | `Pamoja.Lora` | Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to, and the link budget that sets its range |
| [LoRaWAN](https://pamoja.molex.cloud/docs/guides/lorawan.html) | `Pamoja.Lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| [LoRa radios](https://pamoja.molex.cloud/docs/guides/radios.html) | `Pamoja.Radios` | The Semtech SX126x LoRa command set and its decoders, the amplifier setting a regional EIRP ceiling allows, and a duty-cycle guard |
| [Mesh frames](https://pamoja.molex.cloud/docs/guides/mesh.html) | `Pamoja.Mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| [Routing](https://pamoja.molex.cloud/docs/guides/routing.html) | `Pamoja.Routing` | Reverse-path routing that learns the cheapest route from overheard traffic |

The guides, with a worked C# example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
