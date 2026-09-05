# pamoja-update

Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/update.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-update
```

```python
from pamoja import update
```

This pulls in `pamoja-native`, the compiled engine, and `pamoja-security`. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/update.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/update.py):

```python
from pamoja.core import PamojaError
from pamoja.security import DeviceIdentity
from pamoja.update import (
    BootAction,
    Manifest,
    SlotState,
    Updater,
    image_digest,
    sign_manifest,
    verify_envelope,
)

# The publisher's key signs releases; devices in the field are anchored to its public half
# and will take firmware from nobody else.
publisher = DeviceIdentity.from_seed(bytes([7]) * 32)
vendor = bytes([0x0A]) * 16
device_class = bytes([0x0B]) * 16

# The release. A manifest says who the image is for, which slot it belongs in, how big it
# is and what it hashes to; nothing about the image itself is taken on trust.
image = b"firmware for a flow meter, version two"
manifest = Manifest(
    sequence=2,
    vendor_id=vendor,
    class_id=device_class,
    storage=1,
    digest=image_digest(image),
    size=len(image),
)
envelope = sign_manifest(manifest, publisher)
print(f"published sequence {manifest.sequence} in a {len(envelope)}-byte envelope")

# On the device. It checks the envelope against the key it was anchored to before it
# accepts a single byte of the image.
opened = verify_envelope(envelope, publisher.public_key)
print(f"accepted  a release for slot {opened.storage}")

# It left the factory running sequence 1 from slot 0, so the release goes to the spare slot
# and the image it is running stays where it is.
fleet = Updater(vendor, device_class, publisher.public_key, 2, 4096)
fleet.provision(0, 1)
fleet.begin(envelope)
for at in range(0, len(image), 16):
    fleet.write(image[at : at + 16])
print(f"staged    {fleet.progress().written} of {len(image)} bytes")
slot = fleet.finish()
print(f"written   to slot {slot}, leaving the running image alone")

# The first boot into a new image is a trial. It reverts on the next boot unless the device
# confirms that it came up, which is what makes a bad release survivable.
print(f"booting   {fleet.on_boot().action}")
fleet.confirm()
print(f"confirmed slot {slot} is now {fleet.slot_record(slot).state}")

# The same release signed by a key this device is not anchored to gets nowhere.
impostor = DeviceIdentity.from_seed(bytes([90]) * 32)
try:
    fleet.stage(sign_manifest(manifest, impostor), image)
    print("a forged release was accepted, which should never happen")
except PamojaError as error:
    print(f"forged    refused: {error}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-update`](https://crates.io/crates/pamoja-update) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html), [docs.rs](https://docs.rs/pamoja-update) |
| TypeScript | [`@pamoja/update`](https://www.npmjs.com/package/@pamoja/update) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html) |
| Python | [`pamoja-update`](https://pypi.org/project/pamoja-update/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html) |
| C# | [`Pamoja.Update`](https://www.nuget.org/packages/Pamoja.Update) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html) |

## Documentation

- [`pamoja.update` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html), every class and function in this module.
- [The Signed updates guide](https://pamoja.molex.cloud/docs/guides/update.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
