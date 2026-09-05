# pamoja-session

X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/session.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
import os

from pamoja.core import PamojaError
from pamoja.session import AgreementKey, Role, Session

# Each device is provisioned with a 32-byte seed and publishes the key it derives. A real
# seed comes from the factory or a secure element; any 32 bytes stand in here.
node = AgreementKey(bytes([7]) * 32)
gateway = AgreementKey(bytes([9]) * 32)

# Neither side sends the session key. Both derive it from the shared secret, a salt that
# travels in the clear, and both public keys, with opposite roles.
#
# The salt must be fresh for every session: reusing one derives the same key from the same
# pair of devices twice. The initiator draws it and sends it in the clear, so the responder
# uses the salt it received rather than one of its own.
salt = os.urandom(16)
uplink = Session(node, gateway.public_key, salt, Role.INITIATOR)
downlink = Session(gateway, node.public_key, salt, Role.RESPONDER)
print("both sides derived a key without sending one")

# The pump id is authenticated but not encrypted, so a router still reads it while any
# change to it fails the tag.
sealed = uplink.seal(b"flow=41.2", b"pump-3")
print(f"sealed    the reading is no longer readable: {sealed.ciphertext != b'flow=41.2'}")
print(f"opened    {downlink.open(sealed, b'pump-3').decode()}")

# The anti-replay window refuses a counter it has already accepted, so a frame captured
# off the air and sent again is not delivered a second time.
try:
    downlink.open(sealed, b"pump-3")
    print("a replayed frame was accepted, which should never happen")
except PamojaError as error:
    print(f"replay    refused: {error}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-session`](https://crates.io/crates/pamoja-session) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html), [docs.rs](https://docs.rs/pamoja-session) |
| TypeScript | [`@pamoja/session`](https://www.npmjs.com/package/@pamoja/session) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) |
| Python | [`pamoja-session`](https://pypi.org/project/pamoja-session/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html) |
| C# | [`Pamoja.Session`](https://www.nuget.org/packages/Pamoja.Session) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html) |

## Documentation

- [`pamoja.session` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html), every class and function in this module.
- [The Secured session guide](https://pamoja.molex.cloud/docs/guides/session.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
