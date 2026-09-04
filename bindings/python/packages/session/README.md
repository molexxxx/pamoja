# pamoja-session

X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-session
```

```python
from pamoja import session
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/session.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/session.py):

```python
from pamoja.core import PamojaError
from pamoja.session import AgreementKey, Role, Session

# Each device is provisioned with a 32-byte seed and publishes the key it derives. These
# are the X25519 pair RFC 7748 section 6.1 publishes, so the derivation is pinned to the
# specification rather than checked against itself.
node = AgreementKey(
    bytes.fromhex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
)
gateway = AgreementKey(
    bytes.fromhex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
)
assert node.public_key.hex() == (
    "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
)

# Neither side sends the session key. Both derive it from the shared secret, a salt that
# travels in the clear, and both public keys. The roles have to be opposite.
salt = bytes([0x09]) * 16
uplink = Session(node, gateway.public_key, salt, Role.INITIATOR)
downlink = Session(gateway, node.public_key, salt, Role.RESPONDER)

# The pump id is authenticated but not encrypted, so a router still reads it while any
# change to it fails the tag.
sealed = uplink.seal(b"flow=41.2", b"pump-3")
assert sealed.ciphertext != b"flow=41.2"
assert downlink.open(sealed, b"pump-3") == b"flow=41.2"

# The anti-replay window refuses a counter it has already accepted, so a frame captured
# off the air and sent again is not delivered a second time.
try:
    downlink.open(sealed, b"pump-3")
except PamojaError:
    pass
else:
    raise AssertionError("a replayed message should be refused")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-session`](https://crates.io/crates/pamoja-session) | [docs.rs](https://docs.rs/pamoja-session), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html) |
| TypeScript | [`@pamoja/session`](https://www.npmjs.com/package/@pamoja/session) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) |
| Python | [`pamoja-session`](https://pypi.org/project/pamoja-session/) | [`pamoja.session`](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html) |
| C# | [`Pamoja.Session`](https://www.nuget.org/packages/Pamoja.Session) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.Session.html) |

## Documentation

- [The Secured session guide](https://pamoja.molex.cloud/docs/guides/session.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
