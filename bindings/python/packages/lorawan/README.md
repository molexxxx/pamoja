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
from pamoja.lorawan import device, grant, session

# A join accept captured off a live EU868 network, the root key it was signed under, and
# the session keys an independent implementation derived from it. Published at
# https://github.com/anthonykirby/lora-packet/issues/10
captured = bytes.fromhex(
    "204dd85ae608b87fc4889970b7d2042c9e72959b0057aed6094b16003df12de145"
)
app_key = bytes.fromhex("b6b53f4a168a7a88bdf7ea135ce9cfca")
dev_nonce = 0xCC85

# The network half: the address and radio settings this network grants, encrypted and
# signed under the root key, are the frame that was captured.
offer = grant(
    app_nonce=0x00E5063A, net_id=0x13, dev_addr=0x26012E43, dl_settings=0x03,
    rx_delay=0x01, cflist=bytes.fromhex("184f84e85684b85e84886684586e8400"),
)
assert offer.accept(app_key, dev_nonce) == captured

# The device half. A join accept carries no EUI, so only the root key decides whether it
# verifies.
node = device(bytes(8), bytes(8), app_key)
accepted = node.accept_join(captured, dev_nonce)
assert accepted.dev_addr == 0x26012E43

# Neither side transmits a session key; both derive it from the two nonces. What the
# device derived is read back by a session holding the keys published with the capture.
keys = bytes.fromhex("2c96f7028184bb0be8aa49275290d4fcf3a5c8f0232a38c144029c165865802c")
gateway = session(0x26012E43, keys[:16], keys[16:])
uplink = accepted.session().encode_uplink(1, 1, b"real")
assert gateway.decode(uplink, 1).payload == b"real"

# A single byte changed in the air fails the MIC, so no one else can admit the device.
forged = bytearray(captured)
forged[1] ^= 0xFF
try:
    node.accept_join(bytes(forged), dev_nonce)
except PamojaError:
    pass
else:
    raise AssertionError("a join accept nobody signed should not activate a session")
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
