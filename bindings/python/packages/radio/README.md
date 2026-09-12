# pamoja-radio

Budgeting airtime, framing a mesh packet, routing it, and securing a LoRaWAN uplink: everything a node needs to reach a network it cannot see.

One install for the 6 capabilities of this domain. Each is also its own
distribution, and `pamoja` is the whole framework in one.

```sh
pip install pamoja-radio
```

```python
from pamoja.radio import lora
```

| Capability | Module | What it covers |
| --- | --- | --- |
| [LoRa airtime and range](https://pamoja.molex.cloud/docs/guides/lora.html) | `pamoja.lora` | Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to, and the link budget that sets its range |
| [LoRaWAN](https://pamoja.molex.cloud/docs/guides/lorawan.html) | `pamoja.lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| [LoRa radios](https://pamoja.molex.cloud/docs/guides/radios.html) | `pamoja.radios` | The Semtech SX126x and SX127x LoRa radios: their commands, registers, and decoders, the amplifier setting a regional EIRP ceiling allows, and a duty-cycle guard |
| [LoRaWAN gateways](https://pamoja.molex.cloud/docs/guides/gateway.html) | `pamoja.gateway` | What a LoRaWAN gateway speaks: the Semtech packet forwarder protocol and the Basics Station protocol on both sides, the network side of a single site, and a bridge from the radio to the link that leaves it |
| [Mesh frames](https://pamoja.molex.cloud/docs/guides/mesh.html) | `pamoja.mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| [Routing](https://pamoja.molex.cloud/docs/guides/routing.html) | `pamoja.routing` | Reverse-path routing that learns the cheapest route from overheard traffic |

The guides, with a worked Python example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
