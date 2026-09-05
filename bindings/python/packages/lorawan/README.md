# pamoja-lorawan

LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/lorawan.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-lorawan
```

```python
from pamoja import lorawan
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/lorawan.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lorawan.py):

```python
from pamoja.core import PamojaError
from pamoja.lorawan import device, grant

# The root key is provisioned into the device at the factory and known to the network
# server. It is the only secret either side starts with; any 16 bytes stand in here.
app_key = bytes([7]) * 16

# The device asks to join with a nonce it has not used before, which is what stops an old
# accept being replayed at it.
dev_nonce = 1
node = device(bytes(8), bytes(8), app_key)

# The network grants the join. It draws its own nonce, names the network the device is
# joining, and assigns the address the device will answer to from then on.
dev_addr = 0x26012E43
offer = grant(app_nonce=2, net_id=19, dev_addr=dev_addr)
accept = offer.accept(app_key, dev_nonce)
print(f"granted   address 0x{dev_addr:08X} in a {len(accept)}-byte accept")

# The device verifies it against the root key. A join accept carries no device identifier,
# so only that key decides whether it is for this device.
joined = node.accept_join(accept, dev_nonce)
print(f"joined    the device took address 0x{joined.dev_addr:08X}")

# Neither side transmits a session key. Both derive the same pair from the root key and the
# two nonces, so the network reads what the device sends without ever having been told how.
network = offer.session(app_key, dev_nonce)
uplink = joined.session().encode_uplink(1, 1, b"level=high")
received = network.decode(uplink, 1)
print(f"uplink    the network read {received.payload.decode()}")

# A single byte changed in the air fails that check, so no one else can admit the device or
# put words in its mouth.
forged = bytearray(accept)
forged[1] ^= 0xFF
try:
    node.accept_join(bytes(forged), dev_nonce)
    print("a forged accept was taken, which should never happen")
except PamojaError as error:
    print(f"forged    accept refused: {error}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-lorawan`](https://crates.io/crates/pamoja-lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html), [docs.rs](https://docs.rs/pamoja-lorawan) |
| TypeScript | [`@pamoja/lorawan`](https://www.npmjs.com/package/@pamoja/lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html) |
| Python | [`pamoja-lorawan`](https://pypi.org/project/pamoja-lorawan/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html) |
| C# | [`Pamoja.Lorawan`](https://www.nuget.org/packages/Pamoja.Lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html) |

## Documentation

- [`pamoja.lorawan` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html), every class and function in this module.
- [The LoRaWAN guide](https://pamoja.molex.cloud/docs/guides/lorawan.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
